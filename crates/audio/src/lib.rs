/// spectral analysis and audio feature extraction.
pub mod analysis;
/// audio device capture (loopback and microphone).
pub mod capture;

use std::sync::Arc;
use std::thread;

use core::bus::AudioTripleBuffer;
use core::config::{AudioInput, AudioProcessingConfig, FftSize};
use core::error::MyceliumError;
use tracing::info;

use crate::analysis::SpectralAnalyzer;
use crate::capture::AudioCapture;

/// owns the audio processing thread; keep alive to maintain the thread.
pub struct AudioEngine {
    _handle: thread::JoinHandle<()>,
}

impl AudioEngine {
    /// opens the capture device, spawns the processing thread, writes frames to
    /// the triple buffer.
    pub fn start(
        input_mode: AudioInput,
        fft_size: FftSize,
        processing: AudioProcessingConfig,
        audio_buffer: Arc<AudioTripleBuffer>,
    ) -> Result<Self, MyceliumError> {
        let capture = AudioCapture::start(input_mode)?;
        let sample_rate = capture.sample_rate();

        let ring = Arc::clone(capture.ring());
        let handle = thread::Builder::new()
            .name("audio".into())
            .spawn(move || {
                info!("Audio thread started");
                let _capture = capture; // keep alive: owns the cpal stream
                let mut analyzer = SpectralAnalyzer::with_config(fft_size, sample_rate, processing);
                let mut sample_buf = vec![0.0f32; fft_size.as_usize()];
                let start = std::time::Instant::now();

                loop {
                    if ring.read_last_into(&mut sample_buf) {
                        let time = start.elapsed().as_secs_f64();
                        let frame = analyzer.analyze(&sample_buf, time);
                        audio_buffer.write(&frame);
                    }

                    thread::sleep(std::time::Duration::from_millis(1));
                }
            })
            .map_err(|e| MyceliumError::AudioDevice(e.to_string()))?;

        Ok(Self { _handle: handle })
    }
}
