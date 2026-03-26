//! Statistical computation module for scientific visualization.
//!
//! Provides descriptive statistics, kernel density estimation, regression,
//! LOWESS smoothing, bootstrap confidence intervals, and histogram normalization.
//!
//! Enabled by the `statistics` feature flag.
//!
//! # Example
//!
//! ```rust
//! use ratatui_plt::statistics::{mean, std_dev, linear_regression, Kde};
//!
//! let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
//! assert!((mean(&data) - 3.0).abs() < 1e-10);
//!
//! let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
//! let y = vec![3.0, 5.0, 7.0, 9.0, 11.0];
//! let fit = linear_regression(&x, &y).unwrap();
//! assert!((fit.slope - 2.0).abs() < 1e-10);
//! ```

use ratatui::style::Color;

use crate::series::Series;

// ---------------------------------------------------------------------------
// Descriptive Statistics
// ---------------------------------------------------------------------------

/// Compute the arithmetic mean of a slice.
///
/// Returns 0.0 for empty slices.
pub fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    data.iter().sum::<f64>() / data.len() as f64
}

/// Compute the population variance of a slice.
///
/// Returns 0.0 for empty slices.
pub fn variance(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let m = mean(data);
    data.iter().map(|&x| (x - m) * (x - m)).sum::<f64>() / data.len() as f64
}

/// Compute the population standard deviation of a slice.
///
/// Returns 0.0 for empty slices.
pub fn std_dev(data: &[f64]) -> f64 {
    variance(data).sqrt()
}

/// Compute a percentile from pre-sorted data using linear interpolation.
///
/// `p` must be in [0, 100]. Data must be sorted in ascending order.
/// Returns 0.0 for empty slices.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let k = (p / 100.0) * (sorted.len() - 1) as f64;
    let f = k.floor() as usize;
    let c = f.min(sorted.len() - 1);
    let d = k - f as f64;
    if c + 1 < sorted.len() {
        sorted[c] + d * (sorted[c + 1] - sorted[c])
    } else {
        sorted[c]
    }
}

/// Compute the median, sorting a copy of the data internally.
///
/// Returns 0.0 for empty slices.
pub fn median(data: &[f64]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    percentile(&sorted, 50.0)
}

/// Compute the interquartile range (Q3 - Q1) from pre-sorted data.
///
/// Returns 0.0 for empty slices.
pub fn iqr(sorted: &[f64]) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    percentile(sorted, 75.0) - percentile(sorted, 25.0)
}

// ---------------------------------------------------------------------------
// KDE (Kernel Density Estimation)
// ---------------------------------------------------------------------------

/// Bandwidth selection method for KDE.
#[derive(Clone, Debug, Default)]
pub enum BandwidthMethod {
    /// Silverman's rule of thumb: h = 0.9 * min(std, IQR/1.34) * n^(-1/5).
    #[default]
    Silverman,
    /// Scott's rule: h = 1.06 * std * n^(-1/5).
    Scott,
    /// Fixed bandwidth value.
    Fixed(f64),
}

/// Kernel function for KDE.
#[derive(Clone, Debug, Default)]
pub enum Kernel {
    /// Gaussian kernel: (1/sqrt(2*pi)) * exp(-0.5 * x^2).
    #[default]
    Gaussian,
    /// Epanechnikov kernel: 0.75 * (1 - x^2) for |x| <= 1, else 0.
    Epanechnikov,
    /// Uniform kernel: 0.5 for |x| <= 1, else 0.
    Uniform,
}

/// Kernel Density Estimation (KDE) with configurable bandwidth and kernel.
///
/// # Example
///
/// ```rust
/// use ratatui_plt::statistics::{Kde, BandwidthMethod, Kernel};
///
/// let data = vec![1.0, 2.0, 2.5, 3.0, 3.5, 4.0, 5.0];
/// let kde = Kde::new()
///     .bandwidth(BandwidthMethod::Silverman)
///     .kernel(Kernel::Gaussian)
///     .n_points(100);
/// let (x_vals, densities) = kde.fit(&data);
/// assert_eq!(x_vals.len(), 100);
/// assert_eq!(densities.len(), 100);
/// ```
pub struct Kde {
    bandwidth: BandwidthMethod,
    kernel: Kernel,
    n_points: usize,
}

impl Kde {
    /// Create a new KDE with default settings (Silverman bandwidth, Gaussian kernel, 200 points).
    pub fn new() -> Self {
        Self {
            bandwidth: BandwidthMethod::default(),
            kernel: Kernel::default(),
            n_points: 200,
        }
    }

    /// Set the bandwidth selection method.
    pub fn bandwidth(mut self, bw: BandwidthMethod) -> Self {
        self.bandwidth = bw;
        self
    }

    /// Set the kernel function.
    pub fn kernel(mut self, k: Kernel) -> Self {
        self.kernel = k;
        self
    }

    /// Set the number of evaluation points for `fit()`.
    pub fn n_points(mut self, n: usize) -> Self {
        self.n_points = n;
        self
    }

    /// Compute the bandwidth for the given data.
    pub fn compute_bandwidth(&self, data: &[f64]) -> f64 {
        if data.is_empty() {
            return 1.0;
        }
        let n = data.len() as f64;

        match &self.bandwidth {
            BandwidthMethod::Fixed(h) => *h,
            BandwidthMethod::Silverman => {
                let sd = std_dev(data);
                let mut sorted = data.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let data_iqr = iqr(&sorted);

                let spread = if data_iqr > 0.0 {
                    sd.min(data_iqr / 1.34)
                } else if sd > 0.0 {
                    sd
                } else {
                    1.0
                };
                let h = 0.9 * spread * n.powf(-0.2);
                if h <= 0.0 { 1.0 } else { h }
            }
            BandwidthMethod::Scott => {
                let sd = std_dev(data);
                let sd = if sd <= 0.0 { 1.0 } else { sd };
                let h = 1.06 * sd * n.powf(-0.2);
                if h <= 0.0 { 1.0 } else { h }
            }
        }
    }

    /// Fit the KDE to data, returning evenly-spaced evaluation points and densities.
    ///
    /// Generates `n_points` values from `min(data) - 3h` to `max(data) + 3h`.
    /// Returns `(eval_points, densities)`.
    pub fn fit(&self, data: &[f64]) -> (Vec<f64>, Vec<f64>) {
        if data.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let h = self.compute_bandwidth(data);

        let mut min_val = f64::MAX;
        let mut max_val = f64::MIN;
        for &v in data {
            if v < min_val {
                min_val = v;
            }
            if v > max_val {
                max_val = v;
            }
        }

        let lo = min_val - 3.0 * h;
        let hi = max_val + 3.0 * h;

        let n = self.n_points.max(2);
        let step = (hi - lo) / (n - 1) as f64;
        let points: Vec<f64> = (0..n).map(|i| lo + i as f64 * step).collect();
        let densities = self.evaluate(data, &points);

        (points, densities)
    }

    /// Evaluate the KDE density at specific points.
    pub fn evaluate(&self, data: &[f64], points: &[f64]) -> Vec<f64> {
        if data.is_empty() {
            return vec![0.0; points.len()];
        }

        let h = self.compute_bandwidth(data);
        let n = data.len() as f64;
        let inv_h = 1.0 / h;

        points
            .iter()
            .map(|&pt| {
                let sum: f64 = data
                    .iter()
                    .map(|&xi| {
                        let u = (pt - xi) * inv_h;
                        self.kernel_fn(u)
                    })
                    .sum();
                sum / (n * h)
            })
            .collect()
    }

    /// Evaluate the kernel function at u.
    fn kernel_fn(&self, u: f64) -> f64 {
        match &self.kernel {
            Kernel::Gaussian => (1.0 / (2.0 * std::f64::consts::PI).sqrt()) * (-0.5 * u * u).exp(),
            Kernel::Epanechnikov => {
                if u.abs() <= 1.0 {
                    0.75 * (1.0 - u * u)
                } else {
                    0.0
                }
            }
            Kernel::Uniform => {
                if u.abs() <= 1.0 {
                    0.5
                } else {
                    0.0
                }
            }
        }
    }
}

impl Default for Kde {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Regression
// ---------------------------------------------------------------------------

/// Result of a linear regression fit.
#[derive(Clone, Debug)]
pub struct LinearFitResult {
    /// Slope of the fitted line.
    pub slope: f64,
    /// Intercept of the fitted line.
    pub intercept: f64,
    /// Coefficient of determination (R-squared).
    pub r_squared: f64,
    /// Standard error of the slope.
    pub std_err_slope: f64,
    /// Standard error of the intercept.
    pub std_err_intercept: f64,
}

impl LinearFitResult {
    /// Evaluate the fitted line at x.
    pub fn eval(&self, x: f64) -> f64 {
        self.slope * x + self.intercept
    }
}

/// Result of a polynomial regression fit.
#[derive(Clone, Debug)]
pub struct PolyFitResult {
    /// Coefficients from highest to lowest degree: [a_n, ..., a_1, a_0].
    pub coefficients: Vec<f64>,
    /// Coefficient of determination (R-squared).
    pub r_squared: f64,
}

impl PolyFitResult {
    /// Evaluate the polynomial at a given x using Horner's method.
    pub fn eval(&self, x: f64) -> f64 {
        let mut result = 0.0;
        for &c in &self.coefficients {
            result = result * x + c;
        }
        result
    }

    /// Generate a `Series` from the polynomial over a range.
    pub fn to_series(&self, x_min: f64, x_max: f64, n_points: usize, name: &str) -> Series {
        let n = n_points.max(2);
        let step = (x_max - x_min) / (n - 1) as f64;
        let data: Vec<(f64, f64)> = (0..n)
            .map(|i| {
                let x = x_min + i as f64 * step;
                (x, self.eval(x))
            })
            .collect();
        Series::new(name).data(data)
    }
}

/// Fit a linear regression (y = slope*x + intercept) using the normal equations.
///
/// Returns `None` if x and y have different lengths, fewer than 2 points,
/// or if all x values are identical.
pub fn linear_regression(x: &[f64], y: &[f64]) -> Option<LinearFitResult> {
    if x.len() != y.len() || x.len() < 2 {
        return None;
    }

    let n = x.len() as f64;
    let x_mean = mean(x);
    let y_mean = mean(y);

    let mut ss_xx = 0.0;
    let mut ss_xy = 0.0;
    for i in 0..x.len() {
        let dx = x[i] - x_mean;
        let dy = y[i] - y_mean;
        ss_xx += dx * dx;
        ss_xy += dx * dy;
    }

    if ss_xx.abs() < 1e-15 {
        return None;
    }

    let slope = ss_xy / ss_xx;
    let intercept = y_mean - slope * x_mean;

    // R-squared
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for i in 0..x.len() {
        let y_pred = slope * x[i] + intercept;
        ss_res += (y[i] - y_pred) * (y[i] - y_pred);
        ss_tot += (y[i] - y_mean) * (y[i] - y_mean);
    }
    let r_squared = if ss_tot.abs() < 1e-15 {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    // Standard errors
    let dof = n - 2.0;
    let (std_err_slope, std_err_intercept) = if dof > 0.0 {
        let mse = ss_res / dof;
        let se_slope = (mse / ss_xx).sqrt();
        let se_intercept = (mse * (1.0 / n + x_mean * x_mean / ss_xx)).sqrt();
        (se_slope, se_intercept)
    } else {
        (0.0, 0.0)
    };

    Some(LinearFitResult {
        slope,
        intercept,
        r_squared,
        std_err_slope,
        std_err_intercept,
    })
}

/// Fit a polynomial of given degree to (x, y) data using Gaussian elimination
/// with partial pivoting on the normal equations (Vandermonde approach).
///
/// Returns coefficients from highest to lowest degree: [a_n, ..., a_1, a_0].
/// Returns `None` if data is too short, lengths mismatch, or the system is singular.
pub fn poly_fit(x: &[f64], y: &[f64], degree: usize) -> Option<PolyFitResult> {
    if x.len() != y.len() || x.len() <= degree || degree == 0 {
        return None;
    }

    let n = x.len();
    let m = degree + 1; // number of coefficients

    // Build V^T * V and V^T * y where V is the Vandermonde matrix.
    // V[i][j] = x_i^j, so (V^T * V)[j][k] = sum_i x_i^(j+k)
    // and (V^T * y)[j] = sum_i x_i^j * y_i
    let mut vtv = vec![vec![0.0; m]; m];
    let mut vty = vec![0.0; m];

    for i in 0..n {
        let mut xi_pow = vec![1.0; 2 * m];
        for j in 1..2 * m {
            xi_pow[j] = xi_pow[j - 1] * x[i];
        }
        for j in 0..m {
            for k in 0..m {
                vtv[j][k] += xi_pow[j + k];
            }
            vty[j] += xi_pow[j] * y[i];
        }
    }

    // Gaussian elimination with partial pivoting
    let mut aug = vec![vec![0.0; m + 1]; m];
    for i in 0..m {
        for j in 0..m {
            aug[i][j] = vtv[i][j];
        }
        aug[i][m] = vty[i];
    }

    for col in 0..m {
        // Find pivot
        let mut max_val = aug[col][col].abs();
        let mut max_row = col;
        for (row, aug_row) in aug.iter().enumerate().skip(col + 1).take(m - col - 1) {
            let val = aug_row[col].abs();
            if val > max_val {
                max_val = val;
                max_row = row;
            }
        }

        if max_val < 1e-15 {
            return None; // singular
        }

        if max_row != col {
            aug.swap(col, max_row);
        }

        let pivot = aug[col][col];
        for val in aug[col].iter_mut().skip(col).take(m + 1 - col) {
            *val /= pivot;
        }

        for row in 0..m {
            if row == col {
                continue;
            }
            let factor = aug[row][col];
            // Copy the pivot row values we need to avoid double borrow
            let pivot_vals: Vec<f64> = aug[col][col..=m].to_vec();
            for (j, &pv) in pivot_vals.iter().enumerate() {
                aug[row][col + j] -= factor * pv;
            }
        }
    }

    // Extract coefficients (they are in order a_0, a_1, ..., a_n)
    let coeffs_low_to_high: Vec<f64> = (0..m).map(|i| aug[i][m]).collect();

    // Reverse to get highest-to-lowest: [a_n, ..., a_1, a_0]
    let mut coefficients = coeffs_low_to_high;
    coefficients.reverse();

    // Compute R-squared
    let y_mean = mean(y);
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for i in 0..n {
        let mut y_pred = 0.0;
        let mut xi_pow = 1.0;
        // Evaluate using low-to-high order (reversed coefficients)
        for j in (0..m).rev() {
            y_pred += coefficients[j] * xi_pow;
            xi_pow *= x[i];
        }
        ss_res += (y[i] - y_pred) * (y[i] - y_pred);
        ss_tot += (y[i] - y_mean) * (y[i] - y_mean);
    }
    let r_squared = if ss_tot.abs() < 1e-15 {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    Some(PolyFitResult {
        coefficients,
        r_squared,
    })
}

// ---------------------------------------------------------------------------
// LOWESS (Locally Weighted Scatterplot Smoothing)
// ---------------------------------------------------------------------------

/// Result of LOWESS smoothing.
#[derive(Clone, Debug)]
pub struct LowessResult {
    /// Sorted x coordinates.
    pub x: Vec<f64>,
    /// Smoothed y values.
    pub y: Vec<f64>,
}

impl LowessResult {
    /// Convert to a `Series` for plotting.
    pub fn to_series(&self, name: &str, color: Color) -> Series {
        let data: Vec<(f64, f64)> = self.x.iter().copied().zip(self.y.iter().copied()).collect();
        Series::new(name).data(data).color(color)
    }
}

/// Compute LOWESS (Locally Weighted Scatterplot Smoothing).
///
/// `frac` controls the fraction of data used for each local fit (0 < frac <= 1).
/// Returns sorted (x, y_smooth) pairs.
///
/// Returns `None` if data is empty, lengths mismatch, or `frac` is out of range.
pub fn lowess(x: &[f64], y: &[f64], frac: f64) -> Option<LowessResult> {
    if x.len() != y.len() || x.is_empty() || frac <= 0.0 || frac > 1.0 {
        return None;
    }

    let n = x.len();

    // Sort by x
    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by(|&a, &b| x[a].partial_cmp(&x[b]).unwrap_or(std::cmp::Ordering::Equal));

    let sorted_x: Vec<f64> = indices.iter().map(|&i| x[i]).collect();
    let sorted_y: Vec<f64> = indices.iter().map(|&i| y[i]).collect();

    let k = ((frac * n as f64).ceil() as usize).max(2).min(n);
    let mut result_y = Vec::with_capacity(n);

    for i in 0..n {
        let xi = sorted_x[i];

        // Find k nearest neighbors by distance
        let mut dists: Vec<(usize, f64)> = (0..n).map(|j| (j, (sorted_x[j] - xi).abs())).collect();
        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let max_dist = dists[k - 1].1;
        let max_dist = if max_dist <= 0.0 { 1.0 } else { max_dist };

        // Compute tricube weights and weighted least squares (local linear fit)
        let mut sum_w = 0.0;
        let mut sum_wx = 0.0;
        let mut sum_wy = 0.0;
        let mut sum_wxx = 0.0;
        let mut sum_wxy = 0.0;

        for &(j, dist) in dists.iter().take(k) {
            let u = dist / max_dist;
            let w = if u < 1.0 {
                let t = 1.0 - u * u * u;
                t * t * t
            } else {
                0.0
            };
            let xj = sorted_x[j];
            let yj = sorted_y[j];
            sum_w += w;
            sum_wx += w * xj;
            sum_wy += w * yj;
            sum_wxx += w * xj * xj;
            sum_wxy += w * xj * yj;
        }

        // Solve 2x2 system for local slope and intercept
        // [ sum_w    sum_wx  ] [ b0 ]   [ sum_wy  ]
        // [ sum_wx   sum_wxx ] [ b1 ] = [ sum_wxy ]
        let det = sum_w * sum_wxx - sum_wx * sum_wx;
        let yi = if det.abs() < 1e-15 {
            // Degenerate: use weighted mean
            if sum_w.abs() < 1e-15 {
                sorted_y[i]
            } else {
                sum_wy / sum_w
            }
        } else {
            let b0 = (sum_wxx * sum_wy - sum_wx * sum_wxy) / det;
            let b1 = (sum_w * sum_wxy - sum_wx * sum_wy) / det;
            b0 + b1 * xi
        };

        result_y.push(yi);
    }

    Some(LowessResult {
        x: sorted_x,
        y: result_y,
    })
}

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

/// Bootstrap confidence interval result.
#[derive(Clone, Debug)]
pub struct BootstrapCI {
    /// Lower bound of the confidence interval.
    pub lower: f64,
    /// Upper bound of the confidence interval.
    pub upper: f64,
    /// Point estimate (estimator applied to original data).
    pub estimate: f64,
}

/// Function type for bootstrap estimators.
pub type EstimatorFn = fn(&[f64]) -> f64;

/// Mean estimator for use with `bootstrap_ci`.
pub fn mean_estimator(data: &[f64]) -> f64 {
    mean(data)
}

/// Median estimator for use with `bootstrap_ci`.
pub fn median_estimator(data: &[f64]) -> f64 {
    median(data)
}

/// Simple SplitMix64-based pseudo-random number generator.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

/// Compute a bootstrap confidence interval for an estimator.
///
/// Resamples `data` with replacement `n_resamples` times, applies `estimator`
/// to each resample, and returns the percentile-based confidence interval.
///
/// `confidence` should be in (0, 1), e.g. 0.95 for a 95% CI.
pub fn bootstrap_ci(
    data: &[f64],
    estimator: EstimatorFn,
    n_resamples: usize,
    confidence: f64,
    seed: u64,
) -> BootstrapCI {
    let estimate = estimator(data);

    if data.len() < 2 || n_resamples == 0 {
        return BootstrapCI {
            lower: estimate,
            upper: estimate,
            estimate,
        };
    }

    let n = data.len();
    let mut rng = SimpleRng::new(seed);
    let mut estimates = Vec::with_capacity(n_resamples);

    for _ in 0..n_resamples {
        let mut sample = Vec::with_capacity(n);
        for _ in 0..n {
            sample.push(data[rng.next_usize(n)]);
        }
        estimates.push(estimator(&sample));
    }

    estimates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let alpha = 1.0 - confidence;
    let lower = percentile(&estimates, alpha / 2.0 * 100.0);
    let upper = percentile(&estimates, (1.0 - alpha / 2.0) * 100.0);

    BootstrapCI {
        lower,
        upper,
        estimate,
    }
}

// ---------------------------------------------------------------------------
// Histogram Normalization
// ---------------------------------------------------------------------------

/// Extended histogram normalization modes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HistNormExt {
    /// Raw bin counts.
    #[default]
    Count,
    /// Count divided by bin width (frequency density).
    Frequency,
    /// Count divided by total count (probability).
    Probability,
    /// Same as `Probability`.
    Proportion,
    /// Probability times 100.
    Percent,
    /// Count divided by (total * bin_width), forming a proper PDF.
    Density,
}

// ── 2D KDE ──────────────────────────────────────────────────────────────────

/// 2D kernel density estimation using a product (separable) Gaussian kernel.
///
/// Computes independent 1D KDEs on x and y, then multiplies them on a grid.
/// Returns a [`crate::series::GridData`] suitable for heatmap/contour rendering.
///
/// # Example
///
/// ```
/// use ratatui_plt::statistics::Kde2D;
///
/// let x = vec![0.0, 1.0, 2.0, 1.5, 0.5];
/// let y = vec![0.0, 1.0, 0.5, 1.5, 0.8];
/// let grid = Kde2D::new().fit(&x, &y);
/// assert!(grid.nrows() > 0);
/// assert!(grid.ncols() > 0);
/// ```
pub struct Kde2D {
    bandwidth: BandwidthMethod,
    n_points: usize,
}

impl Default for Kde2D {
    fn default() -> Self {
        Self {
            bandwidth: BandwidthMethod::default(),
            n_points: 50,
        }
    }
}

impl Kde2D {
    /// Create a new 2D KDE with default settings (Silverman bandwidth, 50×50 grid).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the bandwidth selection method (applied independently to x and y).
    pub fn bandwidth(mut self, bw: BandwidthMethod) -> Self {
        self.bandwidth = bw;
        self
    }

    /// Set the number of grid points per axis (default: 50).
    pub fn n_points(mut self, n: usize) -> Self {
        self.n_points = n;
        self
    }

    /// Compute the 2D KDE and return a [`crate::series::GridData`].
    ///
    /// Uses a product kernel: f(x, y) ≈ kde_x(x) × kde_y(y) × N, where N is the
    /// number of data points. This is an approximation that works well when x and y
    /// are roughly independent.
    pub fn fit(&self, x: &[f64], y: &[f64]) -> crate::series::GridData {
        let n = x.len().min(y.len());
        if n == 0 {
            return crate::series::GridData {
                x: vec![0.0],
                y: vec![0.0],
                values: vec![vec![0.0]],
            };
        }

        let kde = Kde::new().bandwidth(self.bandwidth.clone()).n_points(self.n_points);

        // Filter to finite pairs
        let (xf, yf): (Vec<f64>, Vec<f64>) = x
            .iter()
            .zip(y.iter())
            .filter(|(a, b)| a.is_finite() && b.is_finite())
            .map(|(&a, &b)| (a, b))
            .unzip();

        if xf.is_empty() {
            return crate::series::GridData {
                x: vec![0.0],
                y: vec![0.0],
                values: vec![vec![0.0]],
            };
        }

        let (x_pts, x_dens) = kde.fit(&xf);
        let (y_pts, y_dens) = kde.fit(&yf);

        let n_data = xf.len() as f64;

        // Compute 2D density as product of marginal densities, scaled by N
        let mut values = Vec::with_capacity(y_pts.len());
        for y_d in &y_dens {
            let row: Vec<f64> = x_dens.iter().map(|x_d| x_d * y_d * n_data).collect();
            values.push(row);
        }

        crate::series::GridData {
            x: x_pts,
            y: y_pts,
            values,
        }
    }
}

// ── Q-Q Plot Points ─────────────────────────────────────────────────────────

/// Theoretical distribution for Q-Q plot comparison.
#[derive(Clone, Debug, Default)]
pub enum QQDistribution {
    /// Standard normal distribution (default).
    #[default]
    Normal,
}

/// Compute quantile-quantile points for comparing data against a theoretical distribution.
///
/// Returns pairs of (theoretical_quantile, sample_quantile) suitable for scatter plotting.
/// Points falling on the diagonal y=x indicate the data matches the distribution.
///
/// # Example
///
/// ```
/// use ratatui_plt::statistics::{QQDistribution, qq_points};
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let points = qq_points(&data, &QQDistribution::Normal);
/// assert_eq!(points.len(), 5);
/// ```
pub fn qq_points(data: &[f64], distribution: &QQDistribution) -> Vec<(f64, f64)> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();
    if n == 0 {
        return Vec::new();
    }

    sorted
        .iter()
        .enumerate()
        .map(|(i, &sample_q)| {
            // Probability plotting position (Hazen formula): (i + 0.5) / n
            let p = (i as f64 + 0.5) / n as f64;
            let theoretical_q = match distribution {
                QQDistribution::Normal => normal_ppf(p),
            };
            (theoretical_q, sample_q)
        })
        .collect()
}

/// Approximate inverse normal CDF (probit function) using rational approximation.
///
/// Abramowitz & Stegun approximation 26.2.23, accurate to ~4.5×10⁻⁴.
fn normal_ppf(p: f64) -> f64 {
    let p = p.clamp(1e-10, 1.0 - 1e-10);

    if p < 0.5 {
        -rational_approx((- 2.0 * p.ln()).sqrt())
    } else {
        rational_approx((- 2.0 * (1.0 - p).ln()).sqrt())
    }
}

/// Rational approximation helper for normal_ppf.
fn rational_approx(t: f64) -> f64 {
    // Coefficients from Peter Acklam's approximation
    const C0: f64 = 2.515_517;
    const C1: f64 = 0.802_853;
    const C2: f64 = 0.010_328;
    const D1: f64 = 1.432_788;
    const D2: f64 = 0.189_269;
    const D3: f64 = 0.001_308;

    t - (C0 + C1 * t + C2 * t * t) / (1.0 + D1 * t + D2 * t * t + D3 * t * t * t)
}
