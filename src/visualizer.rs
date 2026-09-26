use std::collections::VecDeque;
use std::f32::consts::PI;
use std::sync::Arc;
use parking_lot::Mutex;
use rustfft::{num_complex::Complex, Fft, FftPlanner};

pub const FFT_SIZE: usize = 1024;
const MIN_FREQ: f32 = 25.0;
const MAX_FREQ: f32 = 20000.0;
const MIN_DB: f32 = -60.0;
const MAX_DB: f32 = 0.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizerMode {
    Spectrum,
    Waveform,
}

pub struct AudioVisualizer {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    fft_buffer: Vec<Complex<f32>>,
    bar_values: Vec<f32>,
    peak_values: Vec<f32>,
    sample_rate: u32,
    pub mode: VisualizerMode,
}

impl AudioVisualizer {
    pub fn new(sample_rate: u32) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        let window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos()))
            .collect();

        Self {
            fft,
            window,
            fft_buffer: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            bar_values: Vec::new(),
            peak_values: Vec::new(),
            sample_rate,
            mode: VisualizerMode::Spectrum,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        if sample_rate > 0 {
            self.sample_rate = sample_rate;
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            VisualizerMode::Spectrum => VisualizerMode::Waveform,
            VisualizerMode::Waveform => VisualizerMode::Spectrum,
        };
    }

    /// Process PCM samples and compute smoothed bar heights for `num_bars` columns
    pub fn compute_spectrum(
        &mut self,
        samples_queue: &Arc<Mutex<VecDeque<f32>>>,
        num_bars: usize,
        is_playing: bool,
    ) -> Vec<f32> {
        if num_bars == 0 {
            return Vec::new();
        }

        if self.bar_values.len() != num_bars {
            self.bar_values.resize(num_bars, 0.0);
            self.peak_values.resize(num_bars, 0.0);
        }

        if !is_playing {
            // Decay gracefully to zero when paused
            for b in 0..num_bars {
                self.bar_values[b] = (self.bar_values[b] * 0.85 - 0.01).max(0.0);
                self.peak_values[b] = (self.peak_values[b] * 0.90 - 0.01).max(0.0);
            }
            return self.bar_values.clone();
        }

        // Fetch the latest FFT_SIZE samples from the ring buffer
        let samples: Vec<f32> = {
            let q = samples_queue.lock();
            let count = q.len();
            if count >= FFT_SIZE {
                q.iter().skip(count - FFT_SIZE).copied().collect()
            } else {
                let mut v = vec![0.0; FFT_SIZE - count];
                v.extend(q.iter().copied());
                v
            }
        };

        // Apply Hann window and fill FFT buffer
        for i in 0..FFT_SIZE {
            self.fft_buffer[i] = Complex::new(samples[i] * self.window[i], 0.0);
        }

        self.fft.process(&mut self.fft_buffer);

        let half = FFT_SIZE / 2;
        let nyquist = self.sample_rate as f32 / 2.0;
        let mut raw_magnitudes = vec![0.0f32; half];

        for i in 0..half {
            let mag = (self.fft_buffer[i].re.powi(2) + self.fft_buffer[i].im.powi(2)).sqrt()
                / (FFT_SIZE as f32);
            raw_magnitudes[i] = mag;
        }

        // Divide into logarithmically spaced frequency bands
        let f_min = MIN_FREQ.min(nyquist * 0.1);
        let f_max = MAX_FREQ.min(nyquist);

        for b in 0..num_bars {
            let start_f = f_min * (f_max / f_min).powf(b as f32 / num_bars as f32);
            let end_f = f_min * (f_max / f_min).powf((b + 1) as f32 / num_bars as f32);

            let start_bin = ((start_f / nyquist) * half as f32).floor() as usize;
            let end_bin = (((end_f / nyquist) * half as f32).ceil() as usize)
                .max(start_bin + 1)
                .min(half);

            let mut band_max = 0.0f32;
            let mut band_sum = 0.0f32;
            let count = (end_bin - start_bin).max(1);

            for bin in start_bin..end_bin {
                let m = raw_magnitudes[bin];
                if m > band_max {
                    band_max = m;
                }
                band_sum += m;
            }

            // Weighted average: combination of peak and mean
            let band_val = 0.7 * band_max + 0.3 * (band_sum / count as f32);

            // Convert to dB scale
            let db = 20.0 * (band_val + 1e-5).log10();
            let normalized = ((db - MIN_DB) / (MAX_DB - MIN_DB)).clamp(0.0, 1.0);

            // Equalization curve: slightly boost highs and bass for balanced visualization
            let eq_boost = 1.0 + 0.3 * (b as f32 / num_bars as f32);
            let target = (normalized * eq_boost).clamp(0.0, 1.0);

            // Smooth attack and decay (gravity)
            if target > self.bar_values[b] {
                self.bar_values[b] = self.bar_values[b] * 0.3 + target * 0.7; // Fast attack
            } else {
                self.bar_values[b] = (self.bar_values[b] * 0.85).max(0.0); // Smooth decay
            }

            if self.bar_values[b] > self.peak_values[b] {
                self.peak_values[b] = self.bar_values[b];
            } else {
                self.peak_values[b] = (self.peak_values[b] * 0.95 - 0.005).max(0.0);
            }
        }

        self.bar_values.clone()
    }

    /// Sample raw PCM points for waveform view
    pub fn get_waveform_points(
        &self,
        samples_queue: &Arc<Mutex<VecDeque<f32>>>,
        num_points: usize,
    ) -> Vec<f32> {
        let q = samples_queue.lock();
        if q.is_empty() || num_points == 0 {
            return vec![0.0; num_points];
        }

        let len = q.len();
        let step = (len as f32 / num_points as f32).max(1.0);
        let mut points = Vec::with_capacity(num_points);

        for i in 0..num_points {
            let idx = ((i as f32 * step) as usize).min(len - 1);
            let val = q[idx].clamp(-1.0, 1.0);
            points.push(val);
        }

        points
    }
}
