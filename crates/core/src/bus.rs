use std::cell::UnsafeCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use crossbeam::channel::{self, Receiver, Sender};

use crate::event::{AudioFrame, BusEvent};

const EVENT_CHANNEL_CAPACITY: usize = 4096;

/// lock-free triple buffer for AudioFrame.
///
/// three buffers rotate between writer, middle, and reader roles. the atomic
/// `shared` byte encodes the middle index (bits 0-1) and a dirty flag (bit 2).
/// writer and reader each track their own buffer index and swap with middle,
/// ensuring they never touch the same buffer.
pub struct AudioTripleBuffer {
    buffers: [UnsafeCell<AudioFrame>; 3],
    shared: AtomicU8, // bits 0-1: middle index, bit 2: dirty
    write_idx: UnsafeCell<usize>,
    read_idx: UnsafeCell<usize>,
}

// SAFETY: writer (audio thread) and reader (render thread) each own a
// distinct buffer index. they swap with the middle buffer atomically, so
// all three buffers are always assigned to exactly one of {writer, middle,
// reader}. no two roles ever reference the same buffer.
unsafe impl Send for AudioTripleBuffer {}
unsafe impl Sync for AudioTripleBuffer {}

impl AudioTripleBuffer {
    /// initial assignment: writer=0, middle=1, reader=2.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            buffers: [
                UnsafeCell::new(AudioFrame::default()),
                UnsafeCell::new(AudioFrame::default()),
                UnsafeCell::new(AudioFrame::default()),
            ],
            shared: AtomicU8::new(1), // middle=1, not dirty
            write_idx: UnsafeCell::new(0),
            read_idx: UnsafeCell::new(2),
        })
    }

    /// called by the audio thread. writes frame, then swaps writer buffer
    /// into middle and marks dirty.
    pub fn write(&self, frame: &AudioFrame) {
        // SAFETY: only the audio thread calls write(), so write_idx is
        // exclusively owned by this thread.
        let w = unsafe { &mut *self.write_idx.get() };

        // SAFETY: buffer at *w is owned by the writer; neither middle nor
        // reader reference it.
        unsafe {
            *self.buffers[*w].get() = frame.clone();
        }

        // swap our buffer into middle, get old middle back as our new write
        // buffer, and set dirty so the reader knows fresh data is available.
        let old_shared = self.shared.swap((*w as u8) | 0x04, Ordering::AcqRel);
        *w = (old_shared & 0x03) as usize;
    }

    /// called by the render thread. reads the latest frame if new data available.
    pub fn read(&self, target: &mut AudioFrame) {
        let shared = self.shared.load(Ordering::Acquire);
        if (shared & 0x04) == 0 {
            return; // not dirty, keep previous
        }

        // SAFETY: only the render thread calls read(), so read_idx is
        // exclusively owned by this thread.
        let r = unsafe { &mut *self.read_idx.get() };

        // swap our read buffer into middle (clearing dirty), get the fresh
        // middle buffer as our new read buffer.
        let old_shared = self.shared.swap(*r as u8, Ordering::AcqRel);
        *r = (old_shared & 0x03) as usize;

        // SAFETY: buffer at *r is now owned by the reader; the writer and
        // middle do not reference it.
        unsafe {
            *target = (*self.buffers[*r].get()).clone();
        }
    }
}

impl Default for AudioTripleBuffer {
    fn default() -> Self {
        Self {
            buffers: [
                UnsafeCell::new(AudioFrame::default()),
                UnsafeCell::new(AudioFrame::default()),
                UnsafeCell::new(AudioFrame::default()),
            ],
            shared: AtomicU8::new(1),
            write_idx: UnsafeCell::new(0),
            read_idx: UnsafeCell::new(2),
        }
    }
}

/// plugin bus: separate channels for discrete events, lock-free triple
/// buffer for continuous audio data.
pub struct PluginBus {
    event_sender: Sender<BusEvent>,
    event_receiver: Receiver<BusEvent>,
    audio_buffer: Arc<AudioTripleBuffer>,
}

impl PluginBus {
    pub fn new() -> Self {
        let (event_sender, event_receiver) = channel::bounded(EVENT_CHANNEL_CAPACITY);
        Self {
            event_sender,
            event_receiver,
            audio_buffer: AudioTripleBuffer::new(),
        }
    }

    /// clone the event sender for use by plugins and I/O handlers.
    pub fn event_sender(&self) -> Sender<BusEvent> {
        self.event_sender.clone()
    }

    pub fn audio_buffer(&self) -> Arc<AudioTripleBuffer> {
        Arc::clone(&self.audio_buffer)
    }

    /// drain discrete events into a pre-allocated buffer. zero allocation.
    pub fn drain_events(&self, buf: &mut Vec<BusEvent>) {
        buf.clear();
        while let Ok(event) = self.event_receiver.try_recv() {
            buf.push(event);
        }
        buf.sort_unstable_by_key(|e| e.priority());
    }

    /// read the latest audio frame. zero allocation.
    pub fn read_audio(&self, target: &mut AudioFrame) {
        self.audio_buffer.read(target);
    }
}

impl Default for PluginBus {
    fn default() -> Self {
        Self::new()
    }
}
