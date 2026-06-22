use audio::analysis::SpectralAnalyzer;
use core::config::FftSize;

#[test]
fn test_analyze_silence_returns_zero_bands() {
    let mut analyzer = SpectralAnalyzer::new(FftSize::S1024, 48000);
    let silence = vec![0.0f32; 1024];
    let frame = analyzer.analyze(&silence, 0.0);
    for band in &frame.bands {
        assert!(*band < 0.001, "Silent input should produce near-zero bands");
    }
    assert!(!frame.onset);
}

#[test]
fn test_analyze_sine_wave_detects_correct_band() {
    let mut analyzer = SpectralAnalyzer::new(FftSize::S2048, 48000);
    let samples: Vec<f32> = (0..2048)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
        .collect();
    let frame = analyzer.analyze(&samples, 0.0);

    let max_band = frame
        .bands
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap()
        .0;
    assert_eq!(max_band, 2, "440Hz should be loudest in low-mid band");
}

#[test]
fn test_analyze_short_buffer_returns_default() {
    let mut analyzer = SpectralAnalyzer::new(FftSize::S2048, 48000);
    let short = vec![0.0f32; 100];
    let frame = analyzer.analyze(&short, 0.0);
    assert_eq!(frame.spectrum_len, 0);
}

#[test]
fn test_bpm_detection_from_regular_onsets() {
    let mut analyzer = SpectralAnalyzer::new(FftSize::S1024, 48000);
    let loud: Vec<f32> = (0..1024).map(|i| (i as f32 * 0.1).sin() * 10.0).collect();
    let silence = vec![0.0f32; 1024];

    for beat in 0..8 {
        let time = beat as f64 * 0.5;
        analyzer.analyze(&loud, time);
        analyzer.analyze(&silence, time + 0.1);
    }

    let frame = analyzer.analyze(&loud, 4.0);
    assert!(
        frame.bpm > 100.0 && frame.bpm < 140.0,
        "Expected ~120 BPM, got {}",
        frame.bpm
    );
}
