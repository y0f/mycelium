pub mod lua;

use core::engine::ScriptEvaluator;
use core::event::AudioFrame;
use core::quad::Uniforms;

/// wraps LuaRuntime to implement the ScriptEvaluator trait.
pub struct LuaScriptEvaluator {
    runtime: lua::LuaRuntime,
}

impl LuaScriptEvaluator {
    pub fn from_file(path: &str) -> Result<Self, String> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read script {path}: {e}"))?;
        let mut runtime = lua::LuaRuntime::new()
            .map_err(|e| format!("Failed to create Lua runtime: {e}"))?;
        runtime.load_script(&source)
            .map_err(|e| format!("Failed to load Lua script: {e}"))?;
        Ok(Self { runtime })
    }
}

impl ScriptEvaluator for LuaScriptEvaluator {
    fn update(&self, audio: &AudioFrame, time: f32, uniforms: &mut Uniforms) {
        if let Some(output) = self.runtime.update(audio, time) {
            output.apply_to(uniforms);
        }
    }
}
