//! Background computation helpers for expensive statistical operations.
//!
//! These functions offload CPU-intensive work to a blocking thread pool
//! via [`tokio::task::spawn_blocking`], returning a [`JoinHandle`] that
//! resolves to the computed result.

use tokio::task::JoinHandle;

/// Compute a kernel density estimate (KDE) on a background thread.
///
/// Uses a Gaussian kernel with the given `bandwidth`.  Returns a sorted
/// vector of `(x, density)` pairs sampled at 512 points spanning the
/// data range (with a small padding of 3 bandwidths on each side).
///
/// # Example
///
/// ```rust,no_run
/// use ratatui_sim::compute::compute_kde_async;
///
/// # async fn example() {
/// let data = vec![1.0, 1.5, 2.0, 2.5, 3.0];
/// let handle = compute_kde_async(data, 0.3);
/// let kde_points: Vec<(f64, f64)> = handle.await.unwrap();
/// # }
/// ```
pub fn compute_kde_async(data: Vec<f64>, bandwidth: f64) -> JoinHandle<Vec<(f64, f64)>> {
    tokio::task::spawn_blocking(move || compute_kde(&data, bandwidth))
}

/// Compute a histogram on a background thread.
///
/// Returns `(bin_edges, counts)` where `bin_edges` has length `bins + 1`
/// and `counts` has length `bins`.
///
/// # Example
///
/// ```rust,no_run
/// use ratatui_sim::compute::compute_histogram_async;
///
/// # async fn example() {
/// let data = vec![1.0, 1.5, 2.0, 2.5, 3.0, 3.5];
/// let handle = compute_histogram_async(data, 10);
/// let (edges, counts) = handle.await.unwrap();
/// # }
/// ```
pub fn compute_histogram_async(data: Vec<f64>, bins: usize) -> JoinHandle<(Vec<f64>, Vec<f64>)> {
    tokio::task::spawn_blocking(move || compute_histogram(&data, bins))
}

// ---------------------------------------------------------------------------
// Internal implementations
// ---------------------------------------------------------------------------

/// Gaussian KDE implementation.
fn compute_kde(data: &[f64], bandwidth: f64) -> Vec<(f64, f64)> {
    if data.is_empty() {
        return Vec::new();
    }

    let bw = if bandwidth <= 0.0 {
        // Silverman's rule of thumb fallback
        let n = data.len() as f64;
        let mean = data.iter().sum::<f64>() / n;
        let var = data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / n;
        let std = var.sqrt().max(1e-12);
        1.06 * std * n.powf(-0.2)
    } else {
        bandwidth
    };

    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let pad = 3.0 * bw;
    let lo = min - pad;
    let hi = max + pad;

    let n_points = 512;
    let step = (hi - lo) / (n_points - 1) as f64;
    let n = data.len() as f64;
    let norm = 1.0 / (n * bw * (2.0 * std::f64::consts::PI).sqrt());

    (0..n_points)
        .map(|i| {
            let x = lo + i as f64 * step;
            let density: f64 = data
                .iter()
                .map(|&xi| {
                    let z = (x - xi) / bw;
                    (-0.5 * z * z).exp()
                })
                .sum::<f64>()
                * norm;
            (x, density)
        })
        .collect()
}

/// Histogram implementation.
fn compute_histogram(data: &[f64], bins: usize) -> (Vec<f64>, Vec<f64>) {
    let bins = bins.max(1);

    if data.is_empty() {
        let edges = vec![0.0; bins + 1];
        let counts = vec![0.0; bins];
        return (edges, counts);
    }

    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    // Avoid zero-width bins when all values are identical.
    let (lo, hi) = if (max - min).abs() < f64::EPSILON {
        (min - 0.5, max + 0.5)
    } else {
        (min, max)
    };

    let bin_width = (hi - lo) / bins as f64;
    let edges: Vec<f64> = (0..=bins)
        .map(|i| lo + i as f64 * bin_width)
        .collect();
    let mut counts = vec![0.0_f64; bins];

    for &v in data {
        if !v.is_finite() {
            continue;
        }
        let idx = ((v - lo) / bin_width).floor() as usize;
        // Clamp the last-edge case (value == max) into the final bin.
        let idx = idx.min(bins - 1);
        counts[idx] += 1.0;
    }

    (edges, counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kde_basic() {
        let pts = compute_kde(&[0.0, 1.0, 2.0], 0.5);
        assert_eq!(pts.len(), 512);
        // density should be positive in the data range
        let mid = pts.iter().find(|(x, _)| *x >= 1.0).unwrap();
        assert!(mid.1 > 0.0);
    }

    #[test]
    fn kde_empty() {
        let pts = compute_kde(&[], 0.5);
        assert!(pts.is_empty());
    }

    #[test]
    fn histogram_basic() {
        let (edges, counts) = compute_histogram(&[1.0, 2.0, 3.0, 4.0, 5.0], 5);
        assert_eq!(edges.len(), 6);
        assert_eq!(counts.len(), 5);
        let total: f64 = counts.iter().sum();
        assert!((total - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn histogram_empty() {
        let (edges, counts) = compute_histogram(&[], 10);
        assert_eq!(edges.len(), 11);
        assert_eq!(counts.len(), 10);
    }

    #[test]
    fn histogram_identical_values() {
        let (edges, counts) = compute_histogram(&[5.0, 5.0, 5.0], 4);
        assert_eq!(edges.len(), 5);
        let total: f64 = counts.iter().sum();
        assert!((total - 3.0).abs() < f64::EPSILON);
    }
}
