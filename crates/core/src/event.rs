use serde::{Deserialize, Serialize};

/// max FFT bins supported (4096-point FFT = 2048 bins).
pub const MAX_SPECTRUM_BINS: usize = 2048;

/// max samples per stem channel for ML separation.
pub const MAX_STEM_SAMPLES: usize = 2048;

/// numeric node id, no heap allocation in hot paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u32);

/// numeric parameter id, no heap allocation in hot paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ParamId(pub u32);

/// audio analysis data sent from the audio thread.
/// all fixed-size to avoid heap allocations on the render thread.
#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub spectrum: [f32; MAX_SPECTRUM_BINS],
    pub spectrum_len: usize,
    pub bands: [f32; 6],
    pub smoothed_bands: [f32; 6],
    pub peak_bands: [f32; 6],
    pub energy: f32,
    pub spectral_centroid: f32,
    pub spectral_flux: f32,
    pub bpm: f32,
    pub beat_phase: f32,
    pub onset: bool,
    pub chromagram: [f32; 12],
    pub has_stems: bool,
    pub stems: StemData,
    pub mood: Option<[f32; 2]>,
}

/// ML stem separation data, fixed-size arrays.
#[derive(Debug, Clone)]
pub struct StemData {
    pub drums: [f32; MAX_STEM_SAMPLES],
    pub bass: [f32; MAX_STEM_SAMPLES],
    pub vocals: [f32; MAX_STEM_SAMPLES],
    pub other: [f32; MAX_STEM_SAMPLES],
    pub len: usize,
}

impl Default for StemData {
    fn default() -> Self {
        Self {
            drums: [0.0; MAX_STEM_SAMPLES],
            bass: [0.0; MAX_STEM_SAMPLES],
            vocals: [0.0; MAX_STEM_SAMPLES],
            other: [0.0; MAX_STEM_SAMPLES],
            len: 0,
        }
    }
}

impl Default for AudioFrame {
    fn default() -> Self {
        Self {
            spectrum: [0.0; MAX_SPECTRUM_BINS],
            spectrum_len: 0,
            bands: [0.0; 6],
            smoothed_bands: [0.0; 6],
            peak_bands: [0.0; 6],
            energy: 0.0,
            spectral_centroid: 0.0,
            spectral_flux: 0.0,
            bpm: 0.0,
            beat_phase: 0.0,
            onset: false,
            chromagram: [0.0; 12],
            has_stems: false,
            stems: StemData::default(),
            mood: None,
        }
    }
}

/// input source id, enum instead of String for zero-alloc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSource {
    Midi,
    Osc,
    Gamepad,
    Mouse,
    Keyboard,
}

/// unified control event, all fields are Copy, no heap allocation.
#[derive(Debug, Clone, Copy)]
pub enum ControlEvent {
    Continuous {
        source: InputSource,
        channel: u8,
        value: f32,
    },
    Trigger {
        source: InputSource,
        channel: u8,
    },
    Position {
        source: InputSource,
        x: f32,
        y: f32,
    },
}

/// discrete events sent through the plugin bus.
/// separate from AudioFrame which uses a triple buffer.
#[derive(Debug, Clone)]
pub enum BusEvent {
    Control(ControlEvent),
    BeatTrigger { bpm: f32, phase: f32 },
    ParamChange { node: NodeId, param: ParamId, value: f32 },
    NodeGraphUpdate,
    ConfigReload,
    ModeSwitch(RuntimeMode),
}

impl BusEvent {
    /// priority for render loop event ordering. lower number = higher priority.
    pub fn priority(&self) -> u8 {
        match self {
            BusEvent::Control(_) => 0,
            BusEvent::BeatTrigger { .. } => 0,
            BusEvent::ParamChange { .. } => 1,
            BusEvent::NodeGraphUpdate => 2,
            BusEvent::ModeSwitch(_) => 2,
            BusEvent::ConfigReload => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeMode {
    Live,
    Studio,
    Render,
    Ambient,
}
