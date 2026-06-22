use mlua::{Function, Lua, Result as LuaResult, Table};
use tracing::{error, info, warn};

use core::event::AudioFrame;

/// Lua scripting runtime for custom parameter logic.
///
/// exposes audio data to Lua scripts that compute shader parameter values.
/// scripts run once per frame on the render thread; keep them fast.
pub struct LuaRuntime {
    lua: Lua,
    has_update: bool,
}

impl LuaRuntime {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();

        // utility functions exposed to scripts
        lua.globals().set("clamp", lua.create_function(|_, (x, lo, hi): (f64, f64, f64)| {
            Ok(x.max(lo).min(hi))
        })?)?;

        lua.globals().set("lerp", lua.create_function(|_, (a, b, t): (f64, f64, f64)| {
            Ok(a + (b - a) * t)
        })?)?;

        lua.globals().set("smoothstep", lua.create_function(|_, (edge0, edge1, x): (f64, f64, f64)| {
            let t = ((x - edge0) / (edge1 - edge0).max(0.0001)).clamp(0.0, 1.0);
            Ok(t * t * (3.0 - 2.0 * t))
        })?)?;

        Ok(Self {
            lua,
            has_update: false,
        })
    }

    /// loads a Lua script that should define an `update(audio, params)` function.
    pub fn load_script(&mut self, source: &str) -> LuaResult<()> {
        self.lua.load(source).exec()?;
        self.has_update = self.lua.globals().get::<Function>("update").is_ok();
        if self.has_update {
            info!("Lua script loaded with update() function");
        } else {
            warn!("Lua script loaded but no update() function found");
        }
        Ok(())
    }

    /// calls the script's update() function with current audio data.
    /// returns a table of parameter overrides (name -> f32).
    pub fn update(&self, audio: &AudioFrame, time: f32) -> Option<ScriptOutput> {
        if !self.has_update {
            return None;
        }

        let result: LuaResult<ScriptOutput> = self.lua.scope(|_scope| {
            let audio_table = self.lua.create_table()?;
            audio_table.set("band0", audio.bands[0] as f64)?;
            audio_table.set("band1", audio.bands[1] as f64)?;
            audio_table.set("band2", audio.bands[2] as f64)?;
            audio_table.set("band3", audio.bands[3] as f64)?;
            audio_table.set("band4", audio.bands[4] as f64)?;
            audio_table.set("band5", audio.bands[5] as f64)?;
            audio_table.set("sband0", audio.smoothed_bands[0] as f64)?;
            audio_table.set("sband1", audio.smoothed_bands[1] as f64)?;
            audio_table.set("sband2", audio.smoothed_bands[2] as f64)?;
            audio_table.set("sband3", audio.smoothed_bands[3] as f64)?;
            audio_table.set("sband4", audio.smoothed_bands[4] as f64)?;
            audio_table.set("sband5", audio.smoothed_bands[5] as f64)?;
            audio_table.set("energy", audio.energy as f64)?;
            audio_table.set("spectral_centroid", audio.spectral_centroid as f64)?;
            audio_table.set("bpm", audio.bpm as f64)?;
            audio_table.set("beat_phase", audio.beat_phase as f64)?;
            audio_table.set("onset", audio.onset)?;
            audio_table.set("time", time as f64)?;

            let update_fn: Function = self.lua.globals().get("update")?;
            let result: Table = update_fn.call(audio_table)?;

            let mut output = ScriptOutput::default();
            if let Ok(v) = result.get::<f64>("speed") { output.speed = Some(v as f32); }
            if let Ok(v) = result.get::<f64>("intensity") { output.intensity = Some(v as f32); }
            if let Ok(v) = result.get::<f64>("zoom") { output.zoom = Some(v as f32); }
            if let Ok(v) = result.get::<f64>("color_shift") { output.color_shift = Some(v as f32); }
            if let Ok(v) = result.get::<f64>("rotation_speed") { output.rotation_speed = Some(v as f32); }
            if let Ok(v) = result.get::<f64>("bass_reactivity") { output.bass_reactivity = Some(v as f32); }
            if let Ok(v) = result.get::<f64>("flash_intensity") { output.flash_intensity = Some(v as f32); }
            if let Ok(v) = result.get::<f64>("brightness") { output.brightness = Some(v as f32); }

            Ok(output)
        });

        match result {
            Ok(output) => Some(output),
            Err(e) => {
                error!("Lua update() error: {e}");
                None
            }
        }
    }
}

/// output from a Lua script update; None means don't override that param.
#[derive(Debug, Default)]
pub struct ScriptOutput {
    pub speed: Option<f32>,
    pub intensity: Option<f32>,
    pub zoom: Option<f32>,
    pub color_shift: Option<f32>,
    pub rotation_speed: Option<f32>,
    pub bass_reactivity: Option<f32>,
    pub flash_intensity: Option<f32>,
    pub brightness: Option<f32>,
}

impl ScriptOutput {
    /// applies script overrides to uniforms.
    pub fn apply_to(&self, uniforms: &mut core::quad::Uniforms) {
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
