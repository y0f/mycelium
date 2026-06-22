use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::event::RuntimeMode;

/// top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyceliumConfig {
    pub window: WindowConfig,
    pub audio: AudioConfig,
    pub render: RenderConfig,
    pub mode: RuntimeMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub target_fps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub input: AudioInput,
    pub fft_size: FftSize,
    pub ml_enabled: bool,
    #[serde(default)]
    pub processing: AudioProcessingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioProcessingConfig {
    /// EMA smoothing factor per band (0.0 = raw, 1.0 = frozen). lower = more responsive.
    pub smoothing: f32,
    /// per-band gain multipliers [sub-bass, bass, low-mid, high-mid, presence, brilliance].
    pub gain: [f32; 6],
    /// peak hold decay rate per frame (0.0 = instant, 1.0 = never decays).
    pub peak_decay: f32,
    /// onset detection spectral flux threshold.
    pub onset_threshold: f32,
    /// minimum seconds between onsets.
    pub onset_cooldown: f32,
}

impl Default for AudioProcessingConfig {
    fn default() -> Self {
        Self {
            smoothing: 0.15,
            gain: [2.0, 1.5, 1.0, 1.0, 0.8, 0.6],
            peak_decay: 0.95,
            onset_threshold: 1.5,
            onset_cooldown: 0.1,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioInput {
    Mic,
    /// system audio loopback (WASAPI).
    Loopback,
}

/// constrained FFT sizes, must be power of two.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FftSize {
    #[serde(rename = "512")]
    S512,
    #[serde(rename = "1024")]
    S1024,
    #[serde(rename = "2048")]
    S2048,
    #[serde(rename = "4096")]
    S4096,
}

impl FftSize {
    pub fn as_usize(self) -> usize {
        match self {
            FftSize::S512 => 512,
            FftSize::S1024 => 1024,
            FftSize::S2048 => 2048,
            FftSize::S4096 => 4096,
        }
    }
}

/// constrained MSAA sample counts, valid Vulkan values only.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MsaaSamples {
    #[serde(rename = "1")]
    S1,
    #[serde(rename = "2")]
    S2,
    #[serde(rename = "4")]
    S4,
    #[serde(rename = "8")]
    S8,
}

impl MsaaSamples {
    pub fn as_u32(self) -> u32 {
        match self {
            MsaaSamples::S1 => 1,
            MsaaSamples::S2 => 2,
            MsaaSamples::S4 => 4,
            MsaaSamples::S8 => 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderConfig {
    pub msaa: MsaaSamples,
    pub internal_resolution: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    pub lua_enabled: bool,
    pub wasm_enabled: bool,
    pub script_dir: String,
}

/// I/O subsystem settings (MIDI, OSC, gamepad).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoConfig {
    pub midi_enabled: bool,
    pub osc_enabled: bool,
    pub osc_port: u16,
    pub gamepad_enabled: bool,
}

impl MyceliumConfig {
    /// load config from a TOML file path. returns default if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, crate::error::MyceliumError> {
        if !path.exists() {
            let config = Self::default();
            let toml_str = toml::to_string_pretty(&config)
                .map_err(|e| crate::error::MyceliumError::ConfigSerialize(e.to_string()))?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, &toml_str)?;
            return Ok(config);
        }
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }
}

impl Default for MyceliumConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig {
                width: 1920,
                height: 1080,
                fullscreen: false,
                vsync: false,
                target_fps: 144,
            },
            audio: AudioConfig {
                input: AudioInput::Loopback,
                fft_size: FftSize::S2048,
                ml_enabled: false,
                processing: AudioProcessingConfig::default(),
            },
            render: RenderConfig {
                msaa: MsaaSamples::S4,
                internal_resolution: 1.0,
            },
            mode: RuntimeMode::Studio,
        }
    }
}
