use shaders::registry::ShaderRegistry;

#[test]
fn test_builtins_load() {
    let reg = ShaderRegistry::default();
    assert!(reg.len() >= 3, "Expected at least 3 shaders, got {}", reg.len());
    for name in reg.names() {
        assert!(reg.get(name).is_some(), "Shader {name} not found");
    }
}

#[test]
fn test_shader_source_is_valid_wgsl() {
    let reg = ShaderRegistry::default();
    for name in reg.names() {
        let entry = reg.get(name).unwrap();
        assert!(
            entry.source.contains("@fragment"),
            "Shader {name} must have @fragment entry"
        );
        assert!(
            entry.source.contains("@vertex"),
            "Shader {name} must have @vertex entry"
        );
        assert!(
            entry.source.contains("Uniforms"),
            "Shader {name} must use Uniforms struct"
        );
    }
}
