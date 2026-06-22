use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use tracing::{error, info};

use core::config::AudioInput;
use core::error::MyceliumError;

/// fixed-size single-threaded ring buffer for audio samples.
pub struct RingBuffer {
    data: Vec<f32>,
    write_pos: usize,
    len: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0.0; capacity],
            write_pos: 0,
            len: capacity,
        }
    }

    /// push samples, overwriting oldest data.
    pub fn push(&mut self, samples: &[f32]) {
        for &s in samples {
            self.data[self.write_pos] = s;
            self.write_pos = (self.write_pos + 1) % self.len;
        }
    }

    /// read the last `out.len()` samples oldest-first into `out`.
    /// returns the number of samples actually written.
    pub fn read_last_into(&self, out: &mut [f32]) -> usize {
        let count = out.len().min(self.len);
        let start = (self.write_pos + self.len - count) % self.len;
        for (i, slot) in out[..count].iter_mut().enumerate() {
            *slot = self.data[(start + i) % self.len];
        }
        count
    }
}

/// lock-free SPSC ring buffer passing audio samples from the cpal callback to
/// the processing thread without heap allocation.
///
/// sized to hold ~200ms of audio. if the writer laps the reader (extreme
/// scheduling delay), the reader may see partially-updated samples, producing a
/// brief visual glitch, not a crash.
pub(crate) struct SpscRingBuffer {
    data: UnsafeCell<Box<[f32]>>,
    capacity: usize,
    write_pos: AtomicUsize,
    has_new_data: AtomicBool,
}

// SAFETY: single producer (cpal callback) writes via push(), single consumer
// (audio thread) reads via read_last_into(). interior mutation goes through
// UnsafeCell. write_pos with Release/Acquire ordering ensures the reader sees
// writes in order. the ~200ms buffer is large enough that the writer does not
// lap the reader under normal scheduling.
unsafe impl Send for SpscRingBuffer {}
unsafe impl Sync for SpscRingBuffer {}

impl SpscRingBuffer {
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            data: UnsafeCell::new(vec![0.0; capacity].into_boxed_slice()),
            capacity,
            write_pos: AtomicUsize::new(0),
            has_new_data: AtomicBool::new(false),
        })
    }

    /// push samples, called from the cpal callback. zero allocation, writes
    /// directly into the pre-allocated buffer.
    pub fn push(&self, samples: &[f32]) {
        let mut pos = self.write_pos.load(Ordering::Relaxed);
        for &s in samples {
            // SAFETY: pos is always < capacity due to modular arithmetic. only
            // the cpal callback calls push(). interior mutation is legal because
            // data is behind UnsafeCell.
            unsafe {
                let ptr = (*self.data.get()).as_mut_ptr();
                ptr.add(pos).write(s);
            }
            pos = (pos + 1) % self.capacity;
        }
        // store write_pos before has_new_data so the reader's Acquire on
        // has_new_data establishes happens-before for the position.
        self.write_pos.store(pos, Ordering::Release);
        self.has_new_data.store(true, Ordering::Release);
    }

    /// read the last `out.len()` samples oldest-first into `out`.
    /// returns true if new data was available since the last read.
    pub fn read_last_into(&self, out: &mut [f32]) -> bool {
        let had_new = self.has_new_data.swap(false, Ordering::Acquire);
        if !had_new {
            return false;
        }
        let wp = self.write_pos.load(Ordering::Acquire);
        let count = out.len().min(self.capacity);
        let start = (wp + self.capacity - count) % self.capacity;
        // SAFETY: we are the only reader. interior read through UnsafeCell is
        // legal. the writer may be concurrently writing to a different region;
        // the buffer is large enough that overlap is not expected under normal
        // OS scheduling (~200ms headroom vs 1ms read interval).
        unsafe {
            let data = &*self.data.get();
            for (i, slot) in out[..count].iter_mut().enumerate() {
                *slot = data[(start + i) % self.capacity];
            }
        }
        true
    }
}

/// audio input capture from the system default device.
pub struct AudioCapture {
    _stream: Stream,
    ring: Arc<SpscRingBuffer>,
    sample_rate: u32,
}

impl AudioCapture {
    /// start capturing audio, respecting the input mode config.
    pub fn start(input_mode: AudioInput) -> Result<Self, MyceliumError> {
        let host = cpal::default_host();

        match input_mode {
            AudioInput::Loopback => Self::start_loopback(&host),
            AudioInput::Mic => Self::start_mic(&host),
        }
    }

    /// WASAPI loopback: capture system audio output. cpal sets
    /// AUDCLNT_STREAMFLAGS_LOOPBACK when build_input_stream is called on a
    /// render (output) device.
    fn start_loopback(host: &cpal::Host) -> Result<Self, MyceliumError> {
        let device = host
            .default_output_device()
            .ok_or_else(|| MyceliumError::AudioDevice("No output device for loopback".into()))?;

        let dev_name = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| "unknown".into());
        info!("Loopback device: {}", dev_name);

        // WASAPI loopback captures using the output device's config.
        let output_config = device
            .default_output_config()
            .map_err(|e| MyceliumError::AudioDevice(format!("No output config: {e}")))?;

        let sample_rate = output_config.sample_rate();
        let channels = output_config.channels() as usize;
        let sample_format = output_config.sample_format();
        info!("Loopback config: {sample_rate}Hz, {channels}ch, {sample_format:?}");

        let stream_config: StreamConfig = output_config.into();
        // ~200ms of audio at sample_rate
        let ring_capacity = (sample_rate as usize / 5).max(8192);
        let ring = SpscRingBuffer::new(ring_capacity);

        let stream = match sample_format {
            SampleFormat::F32 => Self::build_stream::<f32>(&device, &stream_config, Arc::clone(&ring), channels),
            SampleFormat::I16 => Self::build_stream::<i16>(&device, &stream_config, Arc::clone(&ring), channels),
            SampleFormat::U16 => Self::build_stream::<u16>(&device, &stream_config, Arc::clone(&ring), channels),
            _ => return Err(MyceliumError::AudioDevice("Unsupported sample format".into())),
        }?;

        stream
            .play()
            .map_err(|e| MyceliumError::AudioDevice(e.to_string()))?;

        info!("Loopback capture started");

        Ok(Self {
            _stream: stream,
            ring,
            sample_rate,
        })
    }

    /// microphone capture.
    fn start_mic(host: &cpal::Host) -> Result<Self, MyceliumError> {
        let device = host
            .default_input_device()
            .ok_or_else(|| MyceliumError::AudioDevice("No input device found".into()))?;

        let dev_name = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| "unknown".into());
        info!("Mic device: {}", dev_name);

        let config = device
            .default_input_config()
            .map_err(|e| MyceliumError::AudioDevice(e.to_string()))?;

        let sample_rate = config.sample_rate();
        let channels = config.channels() as usize;
        info!("Mic config: {sample_rate}Hz, {channels} channels");

        let ring_capacity = (sample_rate as usize / 5).max(8192);
        let ring = SpscRingBuffer::new(ring_capacity);
        let stream_config: StreamConfig = config.clone().into();

        let stream = match config.sample_format() {
            SampleFormat::F32 => {
                Self::build_stream::<f32>(&device, &stream_config, Arc::clone(&ring), channels)
            }
            SampleFormat::I16 => {
                Self::build_stream::<i16>(&device, &stream_config, Arc::clone(&ring), channels)
            }
            SampleFormat::U16 => {
                Self::build_stream::<u16>(&device, &stream_config, Arc::clone(&ring), channels)
            }
            _ => return Err(MyceliumError::AudioDevice("Unsupported sample format".into())),
        }?;

        stream
            .play()
            .map_err(|e| MyceliumError::AudioDevice(e.to_string()))?;

        Ok(Self {
            _stream: stream,
            ring,
            sample_rate,
        })
    }

    fn build_stream<T: cpal::Sample + cpal::SizedSample + Send + 'static>(
        device: &Device,
        config: &StreamConfig,
        ring: Arc<SpscRingBuffer>,
        channels: usize,
    ) -> Result<Stream, MyceliumError>
    where
        f32: cpal::FromSample<T>,
    {
        let ch = channels.max(1);
        // mono conversion buffer reused by the callback via move capture, zero
        // allocation per invocation. capped at ring capacity to never exceed it.
        let buf_cap = ring.capacity.min(16384);
        let mut mono_buf = Vec::with_capacity(buf_cap);
        let stream = device
            .build_input_stream(
                config,
                move |data: &[T], _: &_| {
                    mono_buf.clear();
                    let max_frames = buf_cap.min(data.len() / ch);
                    for frame in data[..max_frames * ch].chunks(ch) {
                        let sum: f32 = frame
                            .iter()
                            .map(|s| <f32 as cpal::FromSample<T>>::from_sample_(*s))
                            .sum();
                        mono_buf.push(sum / ch as f32);
                    }
                    ring.push(&mono_buf);
                },
                move |err| {
                    error!("Audio stream error: {err}");
                },
                None,
            )
            .map_err(|e| MyceliumError::AudioDevice(e.to_string()))?;

        Ok(stream)
    }

    /// shared ring buffer for direct reading.
    pub(crate) fn ring(&self) -> &Arc<SpscRingBuffer> {
        &self.ring
    }

    /// sample rate of the captured audio stream.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
