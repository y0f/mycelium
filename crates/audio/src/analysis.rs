use std::collections::VecDeque;

use core::config::{AudioProcessingConfig, FftSize};
use core::event::{AudioFrame, MAX_SPECTRUM_BINS};
use rustfft::{num_complex::Complex, FftPlanner};

/// band frequency ranges (Hz)
const BAND_RANGES: [(f32, f32); 6] = [
    (20.0, 60.0),
    (60.0, 250.0),
    (250.0, 1000.0),
    (1000.0, 4000.0),
    (4000.0, 8000.0),
    (8000.0, 20000.0),
];

const BPM_HISTORY_SIZE: usize = 16;

/// FFT-based spectral analyzer for real-time audio feature extraction.
pub struct SpectralAnalyzer {
    fft_size: usize,
    sample_rate: f32,
    planner: FftPlanner<f32>,
    window: Vec<f32>,
    prev_spectrum: Vec<f32>,
    // pre-allocated FFT work buffer, avoids per-frame allocation
    fft_buffer: Vec<Complex<f32>>,
    // smoothing state persists across frames
    smoothed_bands: [f32; 6],
    peak_bands: [f32; 6],
    config: AudioProcessingConfig,
    bpm_history: VecDeque<f64>,
    last_onset_time: f64,
    current_bpm: f32,
    last_beat_time: f64,
    // gates flux calculation until a valid previous spectrum exists
    frame_count: u64,
}

impl SpectralAnalyzer {
    /// new analyzer with default processing config.
    pub fn new(fft_size: FftSize, sample_rate: u32) -> Self {
        Self::with_config(fft_size, sample_rate, AudioProcessingConfig::default())
    }

    /// new analyzer with the given processing config.
    pub fn with_config(
        fft_size: FftSize,
        sample_rate: u32,
        config: AudioProcessingConfig,
    ) -> Self {
        let size = fft_size.as_usize();
        let window: Vec<f32> = (0..size)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (size - 1) as f32).cos())
            })
            .collect();

        Self {
            fft_size: size,
            sample_rate: sample_rate as f32,
            planner: FftPlanner::new(),
            window,
            prev_spectrum: vec![0.0; size / 2],
            fft_buffer: vec![Complex::new(0.0, 0.0); size],
            smoothed_bands: [0.0; 6],
            peak_bands: [0.0; 6],
            config,
            bpm_history: VecDeque::with_capacity(BPM_HISTORY_SIZE + 1),
            last_onset_time: 0.0,
            current_bpm: 0.0,
            last_beat_time: 0.0,
            frame_count: 0,
        }
    }

    /// update processing config at runtime (e.g. from GUI).
    pub fn set_config(&mut self, config: AudioProcessingConfig) {
        self.config = config;
    }

    /// analyze a buffer of audio samples into a filled AudioFrame.
    pub fn analyze(&mut self, samples: &[f32], time: f64) -> AudioFrame {
        let mut frame = AudioFrame::default();

        if samples.len() < self.fft_size {
            return frame;
        }

        // apply window and compute FFT using pre-allocated buffer
        let fft = self.planner.plan_fft_forward(self.fft_size);
        for (i, (&s, &w)) in samples[..self.fft_size].iter().zip(&self.window).enumerate() {
            self.fft_buffer[i] = Complex::new(s * w, 0.0);
        }

        fft.process(&mut self.fft_buffer);

        // magnitude spectrum, first half only up to Nyquist
        let half = self.fft_size / 2;
        let spectrum_len = half.min(MAX_SPECTRUM_BINS);
        for (i, b) in self.fft_buffer.iter().enumerate().take(spectrum_len) {
            frame.spectrum[i] = b.norm() / (self.fft_size as f32).sqrt();
        }
        frame.spectrum_len = spectrum_len;

        let rms: f32 = samples[..self.fft_size]
            .iter()
            .map(|s| s * s)
            .sum::<f32>()
            / self.fft_size as f32;
        frame.energy = rms.sqrt();

        // spectral centroid (brightness measure)
        let bin_hz = self.sample_rate / self.fft_size as f32;
        let total_magnitude: f32 = (0..spectrum_len).map(|i| frame.spectrum[i]).sum();
        if total_magnitude > 0.001 {
            let weighted_sum: f32 = (0..spectrum_len)
                .map(|i| frame.spectrum[i] * (i as f32 * bin_hz))
                .sum();
            frame.spectral_centroid = weighted_sum / total_magnitude;
        }

        // multi-band energy with per-band gain
        for (band_idx, &(lo, hi)) in BAND_RANGES.iter().enumerate() {
            let lo_bin = (lo / bin_hz).floor() as usize;
            let hi_bin = ((hi / bin_hz).ceil() as usize).min(spectrum_len);
            let energy: f32 = (lo_bin..hi_bin)
                .map(|i| frame.spectrum[i] * frame.spectrum[i])
                .sum();
            frame.bands[band_idx] = energy.sqrt() * self.config.gain[band_idx];
        }

        let alpha = 1.0 - self.config.smoothing.clamp(0.0, 0.99);
        for i in 0..6 {
            self.smoothed_bands[i] = self.smoothed_bands[i] * (1.0 - alpha) + frame.bands[i] * alpha;
            frame.smoothed_bands[i] = self.smoothed_bands[i];
        }

        // peak hold with decay
        for i in 0..6 {
            if frame.bands[i] > self.peak_bands[i] {
                self.peak_bands[i] = frame.bands[i];
            } else {
                self.peak_bands[i] *= self.config.peak_decay;
            }
            frame.peak_bands[i] = self.peak_bands[i];
        }

        // spectral flux onset detection. skip on first frame to avoid a false
        // onset from comparing against an all-zeros prev_spectrum.
        if self.frame_count > 0 {
            let flux: f32 = (0..spectrum_len)
                .map(|i| {
                    let diff = frame.spectrum[i] - self.prev_spectrum.get(i).copied().unwrap_or(0.0);
                    if diff > 0.0 { diff * diff } else { 0.0 }
                })
                .sum();
            frame.spectral_flux = flux;
            frame.onset = flux > self.config.onset_threshold;
        }

        // BPM tracking from onsets
        if frame.onset && time - self.last_onset_time > self.config.onset_cooldown as f64 {
            let interval = time - self.last_onset_time;
            self.last_onset_time = time;

            if interval > 0.2 && interval < 2.0 {
                let bpm = 60.0 / interval;
                self.bpm_history.push_back(bpm);
                if self.bpm_history.len() > BPM_HISTORY_SIZE {
                    self.bpm_history.pop_front();
                }
                if self.bpm_history.len() >= 4 {
                    let avg: f64 =
                        self.bpm_history.iter().sum::<f64>() / self.bpm_history.len() as f64;
                    self.current_bpm = avg as f32;
                }
            }
            self.last_beat_time = time;
        }

        frame.bpm = self.current_bpm;

        // beat phase: 0.0 at beat, 1.0 before next beat
        if self.current_bpm > 0.0 {
            let beat_interval = 60.0 / self.current_bpm as f64;
            let elapsed = time - self.last_beat_time;
            frame.beat_phase = (elapsed / beat_interval).fract() as f32;
        }

        // store spectrum for next frame, copy in place to avoid allocation
        let copy_len = spectrum_len.min(self.prev_spectrum.len());
        self.prev_spectrum[..copy_len].copy_from_slice(&frame.spectrum[..copy_len]);

        self.frame_count += 1;
        frame
    }
}
