use serde::{Deserialize, Serialize};

use crate::event::AudioFrame;
use crate::quad::Uniforms;

/// audio feature that can drive a shader parameter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AudioFeature {
    Band(usize),
    SmoothedBand(usize),
    PeakBand(usize),
    Energy,
    /// spectral centroid (brightness) in Hz.
    SpectralCentroid,
    /// beat phase, 0.0 at beat, 1.0 just before next.
    BeatPhase,
    Bpm,
    /// onset detection flag, 1.0 on onset else 0.0.
    Onset,
    /// elapsed time in seconds.
    Time,
}

/// transform applied to an audio feature before mapping to a shader param.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Transform {
    /// output = input * gain.
    Gain(f32),
    /// map from [in_lo, in_hi] to [out_lo, out_hi].
    MapRange {
        in_lo: f32,
        in_hi: f32,
        out_lo: f32,
        out_hi: f32,
    },
    /// smooth 0->1 transition between edge0 and edge1.
    Smoothstep { edge0: f32, edge1: f32 },
    /// output = input^exponent.
    Power(f32),
}

impl Default for Transform {
    fn default() -> Self {
        Transform::Gain(1.0)
    }
}

/// a single mapping: audio feature -> transform -> shader parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamMapping {
    pub source: AudioFeature,
    pub transform: Transform,
    pub param: ShaderParam,
    /// EMA smoothing applied to the output (0 = raw, 0.99 = very smooth).
    pub smoothing: f32,
    /// current smoothed value (runtime state, not serialized).
    #[serde(skip)]
    pub current: f32,
}

/// shader parameter that can be driven by a mapping.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ShaderParam {
    Speed,
    Intensity,
    Zoom,
    ColorShift,
    RotationSpeed,
    BassReactivity,
    FlashIntensity,
    Brightness,
}

/// the full mapping graph: a list of mappings evaluated each frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingGraph {
    pub mappings: Vec<ParamMapping>,
}

impl Default for MappingGraph {
    fn default() -> Self {
        Self {
            mappings: vec![
                ParamMapping {
                    source: AudioFeature::SmoothedBand(0),
                    transform: Transform::MapRange {
                        in_lo: 0.0, in_hi: 1.0, out_lo: 0.8, out_hi: 2.5,
                    },
                    param: ShaderParam::Zoom,
                    smoothing: 0.3,
                    current: 1.0,
                },
                ParamMapping {
                    source: AudioFeature::SmoothedBand(2),
                    transform: Transform::MapRange {
                        in_lo: 0.0, in_hi: 0.8, out_lo: 0.2, out_hi: 2.0,
                    },
                    param: ShaderParam::RotationSpeed,
                    smoothing: 0.4,
                    current: 0.5,
                },
                ParamMapping {
                    source: AudioFeature::Energy,
                    transform: Transform::MapRange {
                        in_lo: 0.0, in_hi: 0.5, out_lo: 0.8, out_hi: 1.6,
                    },
                    param: ShaderParam::Brightness,
                    smoothing: 0.2,
                    current: 1.0,
                },
                ParamMapping {
                    source: AudioFeature::SpectralCentroid,
                    transform: Transform::MapRange {
                        in_lo: 500.0, in_hi: 8000.0, out_lo: 0.0, out_hi: 1.0,
                    },
                    param: ShaderParam::ColorShift,
                    smoothing: 0.5,
                    current: 0.3,
                },
                ParamMapping {
                    source: AudioFeature::Onset,
                    transform: Transform::Gain(1.0),
                    param: ShaderParam::FlashIntensity,
                    smoothing: 0.0,
                    current: 0.0,
                },
            ],
        }
    }
}

impl MappingGraph {
    /// evaluate all mappings for the current frame's audio data.
    pub fn evaluate(&mut self, audio: &AudioFrame, time: f32) -> EvalResult {
        let mut result = EvalResult::default();

        for mapping in &mut self.mappings {
            let raw = read_feature(&mapping.source, audio, time);
            let transformed = apply_transform(raw, &mapping.transform);

            if mapping.smoothing > 0.001 {
                let alpha = 1.0 - mapping.smoothing;
                mapping.current = mapping.current * mapping.smoothing + transformed * alpha;
            } else {
                mapping.current = transformed;
            }

            match mapping.param {
                ShaderParam::Speed => result.speed = Some(mapping.current),
                ShaderParam::Intensity => result.intensity = Some(mapping.current),
                ShaderParam::Zoom => result.zoom = Some(mapping.current),
                ShaderParam::ColorShift => result.color_shift = Some(mapping.current),
                ShaderParam::RotationSpeed => result.rotation_speed = Some(mapping.current),
                ShaderParam::BassReactivity => result.bass_reactivity = Some(mapping.current),
                ShaderParam::FlashIntensity => result.flash_intensity = Some(mapping.current),
                ShaderParam::Brightness => result.brightness = Some(mapping.current),
            }
        }

        result
    }
}

/// result of evaluating the mapping graph. `None` means "use GUI slider value".
#[derive(Debug, Default)]
pub struct EvalResult {
    pub speed: Option<f32>,
    pub intensity: Option<f32>,
    pub zoom: Option<f32>,
    pub color_shift: Option<f32>,
    pub rotation_speed: Option<f32>,
    pub bass_reactivity: Option<f32>,
    pub flash_intensity: Option<f32>,
    pub brightness: Option<f32>,
}

impl EvalResult {
    /// apply mapped values to uniforms. non-mapped params keep their current value.
    pub fn apply_to(&self, uniforms: &mut Uniforms) {
        if let Some(v) = self.speed { uniforms.speed = v; }
        if let Some(v) = self.intensity { uniforms.intensity = v; }
        if let Some(v) = self.zoom { uniforms.zoom = v; }
        if let Some(v) = self.color_shift { uniforms.color_shift = v; }
        if let Some(v) = self.rotation_speed { uniforms.rotation_speed = v; }
        if let Some(v) = self.bass_reactivity { uniforms.bass_reactivity = v; }
        if let Some(v) = self.flash_intensity { uniforms.flash_intensity = v; }
        if let Some(v) = self.brightness { uniforms.brightness = v; }
    }
}

fn read_feature(feature: &AudioFeature, audio: &AudioFrame, time: f32) -> f32 {
    match *feature {
        AudioFeature::Band(i) => audio.bands.get(i).copied().unwrap_or(0.0),
        AudioFeature::SmoothedBand(i) => audio.smoothed_bands.get(i).copied().unwrap_or(0.0),
        AudioFeature::PeakBand(i) => audio.peak_bands.get(i).copied().unwrap_or(0.0),
        AudioFeature::Energy => audio.energy,
        AudioFeature::SpectralCentroid => audio.spectral_centroid,
        AudioFeature::BeatPhase => audio.beat_phase,
        AudioFeature::Bpm => audio.bpm,
        AudioFeature::Onset => if audio.onset { 1.0 } else { 0.0 },
        AudioFeature::Time => time,
    }
}

fn apply_transform(value: f32, transform: &Transform) -> f32 {
    match *transform {
        Transform::Gain(g) => value * g,
        Transform::MapRange { in_lo, in_hi, out_lo, out_hi } => {
            let t = ((value - in_lo) / (in_hi - in_lo).max(0.0001)).clamp(0.0, 1.0);
            out_lo + t * (out_hi - out_lo)
        }
        Transform::Smoothstep { edge0, edge1 } => {
            let t = ((value - edge0) / (edge1 - edge0).max(0.0001)).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        }
        Transform::Power(exp) => value.max(0.0).powf(exp),
    }
}
