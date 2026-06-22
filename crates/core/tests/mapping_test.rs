use core::event::AudioFrame;
use core::mapping::*;

#[test]
fn test_gain_transform() {
    let mut graph = MappingGraph {
        mappings: vec![ParamMapping {
            source: AudioFeature::Energy,
            transform: Transform::Gain(2.0),
            param: ShaderParam::Brightness,
            smoothing: 0.0,
            current: 0.0,
        }],
    };

    let mut frame = AudioFrame::default();
    frame.energy = 0.5;

    let result = graph.evaluate(&frame, 0.0);
    assert!((result.brightness.unwrap() - 1.0).abs() < 0.01);
}

#[test]
fn test_map_range_transform() {
    let mut graph = MappingGraph {
        mappings: vec![ParamMapping {
            source: AudioFeature::Energy,
            transform: Transform::MapRange {
                in_lo: 0.0,
                in_hi: 1.0,
                out_lo: 0.5,
                out_hi: 2.0,
            },
            param: ShaderParam::Zoom,
            smoothing: 0.0,
            current: 0.0,
        }],
    };

    let mut frame = AudioFrame::default();
    frame.energy = 0.5;

    let result = graph.evaluate(&frame, 0.0);
    assert!((result.zoom.unwrap() - 1.25).abs() < 0.01);
}

#[test]
fn test_map_range_clamps() {
    let mut graph = MappingGraph {
        mappings: vec![ParamMapping {
            source: AudioFeature::Energy,
            transform: Transform::MapRange {
                in_lo: 0.0,
                in_hi: 1.0,
                out_lo: 0.0,
                out_hi: 1.0,
            },
            param: ShaderParam::Speed,
            smoothing: 0.0,
            current: 0.0,
        }],
    };

    let mut frame = AudioFrame::default();
    frame.energy = 5.0; // way above in_hi

    let result = graph.evaluate(&frame, 0.0);
    assert!((result.speed.unwrap() - 1.0).abs() < 0.01); // clamped to out_hi
}

#[test]
fn test_smoothing_ema() {
    let mut graph = MappingGraph {
        mappings: vec![ParamMapping {
            source: AudioFeature::Onset,
            transform: Transform::Gain(1.0),
            param: ShaderParam::FlashIntensity,
            smoothing: 0.5,
            current: 0.0,
        }],
    };

    let mut frame = AudioFrame::default();
    frame.onset = true;

    // first frame: smoothed from 0 toward 1
    let r1 = graph.evaluate(&frame, 0.0);
    let v1 = r1.flash_intensity.unwrap();
    assert!(v1 > 0.0 && v1 < 1.0, "Should be partially smoothed: {v1}");

    // second frame: closer to 1
    let r2 = graph.evaluate(&frame, 0.0);
    let v2 = r2.flash_intensity.unwrap();
    assert!(v2 > v1, "Should be closer to target: {v2} > {v1}");
}

#[test]
fn test_unmapped_params_are_none() {
    let mut graph = MappingGraph {
        mappings: vec![ParamMapping {
            source: AudioFeature::Energy,
            transform: Transform::Gain(1.0),
            param: ShaderParam::Brightness,
            smoothing: 0.0,
            current: 0.0,
        }],
    };

    let frame = AudioFrame::default();
    let result = graph.evaluate(&frame, 0.0);

    assert!(result.brightness.is_some());
    assert!(result.speed.is_none());
    assert!(result.zoom.is_none());
    assert!(result.color_shift.is_none());
}

#[test]
fn test_band_out_of_range_returns_zero() {
    let mut graph = MappingGraph {
        mappings: vec![ParamMapping {
            source: AudioFeature::Band(99), // out of range
            transform: Transform::Gain(1.0),
            param: ShaderParam::Speed,
            smoothing: 0.0,
            current: 0.0,
        }],
    };

    let frame = AudioFrame::default();
    let result = graph.evaluate(&frame, 0.0);
    assert!((result.speed.unwrap()).abs() < 0.001);
}
