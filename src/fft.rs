//! FFT utilities for spectral analysis (behind `fft` feature).
//!
//! Provides Short-Time Fourier Transform (STFT), Power Spectral Density (PSD),
//! and window functions for spectral analysis of time-series data.

use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

use crate::series::{GridData, Series};

/// Compute a Hann window of the given size.
///
/// The Hann window is defined as `0.5 * (1 - cos(2*pi*n / (N-1)))`.
///
/// # Example
///
/// ```
/// use ratatui_plt::fft::hann_window;
///
/// let window = hann_window(256);
/// assert_eq!(window.len(), 256);
/// assert!((window[0]).abs() < 1e-10);
/// ```
pub fn hann_window(size: usize) -> Vec<f64> {
    if size <= 1 {
        return vec![1.0; size];
    }
    (0..size)
        .map(|n| 0.5 * (1.0 - (2.0 * std::f64::consts::PI * n as f64 / (size - 1) as f64).cos()))
        .collect()
}

/// Compute a Hamming window of the given size.
///
/// The Hamming window is defined as `0.54 - 0.46 * cos(2*pi*n / (N-1))`.
///
/// # Example
///
/// ```
/// use ratatui_plt::fft::hamming_window;
///
/// let window = hamming_window(256);
/// assert_eq!(window.len(), 256);
/// ```
pub fn hamming_window(size: usize) -> Vec<f64> {
    if size <= 1 {
        return vec![1.0; size];
    }
    (0..size)
        .map(|n| 0.54 - 0.46 * (2.0 * std::f64::consts::PI * n as f64 / (size - 1) as f64).cos())
        .collect()
}

/// Compute the Short-Time Fourier Transform (STFT) of a signal.
///
/// Returns a [`GridData`] where x = time bins, y = frequency bins,
/// and values = magnitude (linear scale).
///
/// Uses a Hann window by default. The number of frequency bins is `window_size / 2 + 1`
/// (one-sided spectrum).
///
/// # Arguments
///
/// * `signal` - Input time-domain signal samples
/// * `window_size` - Number of samples per FFT window
/// * `hop_size` - Number of samples to advance between consecutive windows
///
/// # Example
///
/// ```
/// use ratatui_plt::fft::stft;
///
/// let signal: Vec<f64> = (0..1024).map(|i| (i as f64 * 0.1).sin()).collect();
/// let grid = stft(&signal, 256, 128);
/// ```
pub fn stft(signal: &[f64], window_size: usize, hop_size: usize) -> GridData {
    if signal.is_empty() || window_size == 0 || hop_size == 0 {
        return GridData::new(vec![0.0], vec![0.0], vec![vec![0.0]]);
    }

    let window = hann_window(window_size);
    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(window_size);

    let n_freq = window_size / 2 + 1;
    let n_time = if signal.len() >= window_size {
        (signal.len() - window_size) / hop_size + 1
    } else {
        0
    };

    if n_time == 0 {
        return GridData::new(vec![0.0], vec![0.0], vec![vec![0.0]]);
    }

    // Time bin centers (in sample indices, normalized to [0, 1] range)
    let total_samples = signal.len() as f64;
    let x: Vec<f64> = (0..n_time)
        .map(|t| {
            let center = t * hop_size + window_size / 2;
            center as f64 / total_samples
        })
        .collect();

    // Frequency bins (normalized to [0, 0.5] = Nyquist)
    let y: Vec<f64> = (0..n_freq).map(|f| f as f64 / window_size as f64).collect();

    // Compute STFT magnitudes
    // GridData values are indexed as values[freq_bin][time_bin]
    let mut values = vec![vec![0.0; n_time]; n_freq];
    let mut scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];

    for (t, time_offset) in (0..signal.len().saturating_sub(window_size - 1))
        .step_by(hop_size)
        .enumerate()
    {
        if t >= n_time {
            break;
        }

        // Apply window and convert to complex
        let mut frame: Vec<Complex<f64>> = (0..window_size)
            .map(|i| Complex::new(signal[time_offset + i] * window[i], 0.0))
            .collect();

        fft.process_with_scratch(&mut frame, &mut scratch);

        // Store magnitude for one-sided spectrum
        for (f, val) in frame.iter().enumerate().take(n_freq) {
            values[f][t] = val.norm();
        }
    }

    GridData::new(x, y, values)
}

/// Compute the Power Spectral Density (PSD) of a signal.
///
/// Returns a [`Series`] with x = frequency (Hz) and y = power (dB).
/// Uses Welch's method with a Hann window for spectral estimation.
///
/// # Arguments
///
/// * `signal` - Input time-domain signal samples
/// * `sample_rate` - Sampling rate in Hz
///
/// # Example
///
/// ```
/// use ratatui_plt::fft::psd;
///
/// let signal: Vec<f64> = (0..1024).map(|i| (i as f64 * 0.1).sin()).collect();
/// let series = psd(&signal, 44100.0);
/// ```
pub fn psd(signal: &[f64], sample_rate: f64) -> Series {
    if signal.is_empty() || sample_rate <= 0.0 {
        return Series::new("PSD").data(vec![(0.0, 0.0)]);
    }

    // Use Welch's method: divide signal into overlapping segments
    let window_size = 256.min(signal.len());
    let hop_size = window_size / 2;
    let window = hann_window(window_size);
    let window_power: f64 = window.iter().map(|w| w * w).sum();

    let mut planner = FftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(window_size);
    let n_freq = window_size / 2 + 1;

    let mut psd_accum = vec![0.0; n_freq];
    let mut n_segments = 0u64;
    let mut scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];

    let mut offset = 0;
    while offset + window_size <= signal.len() {
        let mut frame: Vec<Complex<f64>> = (0..window_size)
            .map(|i| Complex::new(signal[offset + i] * window[i], 0.0))
            .collect();

        fft.process_with_scratch(&mut frame, &mut scratch);

        for (f, val) in frame.iter().enumerate().take(n_freq) {
            let mag_sq = val.norm_sqr();
            psd_accum[f] += mag_sq;
        }
        n_segments += 1;
        offset += hop_size;
    }

    if n_segments == 0 {
        return Series::new("PSD").data(vec![(0.0, 0.0)]);
    }

    // Normalize: PSD = |X|^2 / (fs * window_power * n_segments)
    let scale = sample_rate * window_power * n_segments as f64;
    let freq_resolution = sample_rate / window_size as f64;

    let data: Vec<(f64, f64)> = (0..n_freq)
        .map(|f| {
            let freq = f as f64 * freq_resolution;
            let power = psd_accum[f] / scale;
            // Convert to dB (10 * log10), with floor to avoid -inf
            let db = 10.0 * power.max(1e-20).log10();
            (freq, db)
        })
        .collect();

    Series::new("PSD").data(data)
}
