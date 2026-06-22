use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectPreset {
    pub name: String,
    pub speed: f32,
    pub intensity: f32,
    pub zoom: f32,
    pub color_shift: f32,
    pub rotation_speed: f32,
    pub bass_reactivity: f32,
}

impl Default for EffectPreset {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            speed: 0.8,
            intensity: 1.5,
            zoom: 1.0,
            color_shift: 0.3,
            rotation_speed: 0.5,
            bass_reactivity: 2.5,
        }
    }
}

pub fn recommended_preset(shader_name: &str) -> EffectPreset {
    match shader_name {
        "fractal" => EffectPreset {
            name: "cosmic".to_string(),
            speed: 0.6, intensity: 1.5, zoom: 1.0,
            color_shift: 0.3, rotation_speed: 0.3, bass_reactivity: 2.0,
        },
        "hypnotic" => EffectPreset {
            name: "trance".to_string(),
            speed: 1.2, intensity: 1.8, zoom: 1.0,
            color_shift: 0.5, rotation_speed: 0.8, bass_reactivity: 3.0,
        },
        "voronoi" => EffectPreset {
            name: "organic".to_string(),
            speed: 0.5, intensity: 1.2, zoom: 1.5,
            color_shift: 0.2, rotation_speed: 0.3, bass_reactivity: 2.0,
        },
        "kaleidoscope" => EffectPreset {
            name: "mirror".to_string(),
            speed: 0.7, intensity: 1.5, zoom: 0.8,
            color_shift: 0.4, rotation_speed: 0.6, bass_reactivity: 2.5,
        },
        "neural" => EffectPreset {
            name: "brain".to_string(),
            speed: 0.4, intensity: 1.3, zoom: 1.2,
            color_shift: 0.6, rotation_speed: 0.2, bass_reactivity: 1.5,
        },
        "strobe" => EffectPreset {
            name: "rave".to_string(),
            speed: 2.0, intensity: 2.0, zoom: 1.0,
            color_shift: 0.0, rotation_speed: 1.5, bass_reactivity: 4.0,
        },
        "geometry" => EffectPreset {
            name: "sacred".to_string(),
            speed: 0.5, intensity: 1.0, zoom: 1.5,
            color_shift: 0.3, rotation_speed: 0.4, bass_reactivity: 2.0,
        },
        "nebula" => EffectPreset {
            name: "deep space".to_string(),
            speed: 0.3, intensity: 1.0, zoom: 1.0,
            color_shift: 0.3, rotation_speed: 0.1, bass_reactivity: 1.5,
        },
        "electric" => EffectPreset {
            name: "discharge".to_string(),
            speed: 0.8, intensity: 1.2, zoom: 1.0,
            color_shift: 0.2, rotation_speed: 0.3, bass_reactivity: 2.5,
        },
        "liquid" => EffectPreset {
            name: "flow".to_string(),
            speed: 0.5, intensity: 1.0, zoom: 1.0,
            color_shift: 0.4, rotation_speed: 0.2, bass_reactivity: 2.0,
        },
        _ => EffectPreset::default(),
    }
}
