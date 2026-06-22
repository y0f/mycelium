use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::mapping::MappingGraph;

/// shader selection, parameter values, and audio mappings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub shader: String,
    pub params: PresetParams,
    #[serde(default)]
    pub mappings: MappingGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetParams {
    pub speed: f32,
    pub intensity: f32,
    pub zoom: f32,
    /// hue rotation offset.
    pub color_shift: f32,
    pub rotation_speed: f32,
    /// sub-bass reactivity multiplier.
    pub bass_reactivity: f32,
    /// flash overlay intensity on onset.
    pub flash_intensity: f32,
    pub brightness: f32,
}

impl Default for PresetParams {
    fn default() -> Self {
        Self {
            speed: 0.8,
            intensity: 1.5,
            zoom: 1.0,
            color_shift: 0.3,
            rotation_speed: 0.5,
            bass_reactivity: 2.5,
            flash_intensity: 0.3,
            brightness: 1.1,
        }
    }
}

impl Preset {
    pub fn save(&self, path: &Path) -> Result<(), crate::error::MyceliumError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| crate::error::MyceliumError::ConfigSerialize(e.to_string()))?;
        std::fs::write(path, &toml_str)?;
        info!("Preset saved: {}", path.display());
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, crate::error::MyceliumError> {
        let content = std::fs::read_to_string(path)?;
        let preset: Self = toml::from_str(&content)?;
        info!("Preset loaded: {} ({})", preset.name, path.display());
        Ok(preset)
    }

    pub fn list_presets(dir: &Path) -> Vec<std::path::PathBuf> {
        let mut presets = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "toml") {
                    presets.push(path);
                }
            }
        }
        presets.sort();
        presets
    }
}
