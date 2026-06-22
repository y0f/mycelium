use std::io::Write;

use core::config::MyceliumConfig;
use tempfile::NamedTempFile;

#[test]
fn test_default_config_roundtrips_through_toml() {
    let config = MyceliumConfig::default();
    let toml_str = toml::to_string_pretty(&config).unwrap();
    let parsed: MyceliumConfig = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed.window.width, 1920);
    assert_eq!(parsed.window.target_fps, 144);
    assert_eq!(parsed.audio.fft_size.as_usize(), 2048);
    assert_eq!(parsed.render.msaa.as_u32(), 4);
}

#[test]
fn test_config_loads_from_toml_string() {
    let toml_str = r#"
        mode = "Live"

        [window]
        width = 2560
        height = 1440
        fullscreen = true
        vsync = true
        target_fps = 60

        [audio]
        input = "mic"
        fft_size = "1024"
        ml_enabled = false

        [render]
        msaa = "8"
        internal_resolution = 0.5
    "#;
    let config: MyceliumConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.window.width, 2560);
    assert!(config.window.fullscreen);
    assert_eq!(config.audio.fft_size.as_usize(), 1024);
    assert_eq!(config.render.msaa.as_u32(), 8);
}

#[test]
fn test_invalid_fft_size_fails() {
    let toml_str = r#"
        mode = "Studio"

        [window]
        width = 1920
        height = 1080
        fullscreen = false
        vsync = false
        target_fps = 144

        [audio]
        input = "loopback"
        fft_size = "3000"
        ml_enabled = false

        [render]
        msaa = "4"
        internal_resolution = 1.0
    "#;
    let result: Result<MyceliumConfig, _> = toml::from_str(toml_str);
    assert!(result.is_err());
}

#[test]
fn test_config_load_from_file() {
    let mut tmp = NamedTempFile::new().unwrap();
    let toml_str = toml::to_string_pretty(&MyceliumConfig::default()).unwrap();
    write!(tmp, "{}", toml_str).unwrap();
    let config = MyceliumConfig::load(tmp.path()).unwrap();
    assert_eq!(config.window.width, 1920);
}

#[test]
fn test_config_load_creates_default_if_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let config = MyceliumConfig::load(&path).unwrap();
    assert_eq!(config.window.width, 1920);
    assert!(path.exists());
}
