use core::mapping::MappingGraph;
use core::preset::{Preset, PresetParams};

#[test]
fn test_preset_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.toml");

    let preset = Preset {
        name: "test_preset".to_string(),
        shader: "fractal".to_string(),
        params: PresetParams {
            speed: 1.5,
            intensity: 2.0,
            zoom: 3.0,
            color_shift: 0.7,
            rotation_speed: 0.3,
            bass_reactivity: 4.0,
            flash_intensity: 0.8,
            brightness: 1.2,
        },
        mappings: MappingGraph::default(),
    };

    preset.save(&path).unwrap();
    let loaded = Preset::load(&path).unwrap();

    assert_eq!(loaded.name, "test_preset");
    assert_eq!(loaded.shader, "fractal");
    assert!((loaded.params.speed - 1.5).abs() < 0.001);
    assert!((loaded.params.zoom - 3.0).abs() < 0.001);
    assert!((loaded.params.brightness - 1.2).abs() < 0.001);
}

#[test]
fn test_preset_mapping_skip_current() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test2.toml");

    let preset = Preset {
        name: "with_mappings".to_string(),
        shader: "voronoi".to_string(),
        params: PresetParams::default(),
        mappings: MappingGraph::default(),
    };

    preset.save(&path).unwrap();
    let loaded = Preset::load(&path).unwrap();

    // default has 5 mappings
    assert!(!loaded.mappings.mappings.is_empty());

    // current field is 0.0 (serde skip)
    for m in &loaded.mappings.mappings {
        assert!((m.current).abs() < 0.001, "current should be 0.0 after deserialization");
    }
}

#[test]
fn test_preset_load_missing_file() {
    let result = Preset::load(std::path::Path::new("nonexistent.toml"));
    assert!(result.is_err());
}

#[test]
fn test_list_presets() {
    let dir = tempfile::tempdir().unwrap();

    let p1 = Preset {
        name: "a".to_string(),
        shader: "fractal".to_string(),
        params: PresetParams::default(),
        mappings: MappingGraph::default(),
    };
    p1.save(&dir.path().join("a.toml")).unwrap();
    p1.save(&dir.path().join("b.toml")).unwrap();

    let list = Preset::list_presets(dir.path());
    assert_eq!(list.len(), 2);
}
