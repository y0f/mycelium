use shaders::effects;
use shaders::registry::ShaderRegistry;

#[test]
fn test_recommended_presets_exist_for_all_shaders() {
    let reg = ShaderRegistry::default();
    for name in reg.names() {
        let preset = effects::recommended_preset(name);
        assert!(preset.speed > 0.0, "Preset for {name} has zero speed");
        assert!(preset.intensity > 0.0, "Preset for {name} has zero intensity");
    }
}

#[test]
fn test_default_preset_has_sane_values() {
    let preset = effects::EffectPreset::default();
    assert!(preset.speed > 0.0 && preset.speed <= 5.0);
    assert!(preset.intensity > 0.0 && preset.intensity <= 5.0);
    assert!(preset.zoom > 0.0 && preset.zoom <= 20.0);
    assert!(preset.color_shift >= 0.0 && preset.color_shift <= 1.0);
}
