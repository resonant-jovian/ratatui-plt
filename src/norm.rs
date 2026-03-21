//! Data normalization for mapping values to the [0, 1] range.
//!
//! Used by colormaps and color-mapped widgets (heatmaps, contour plots) to
//! convert data values into colors. Inspired by matplotlib's Normalize classes.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::norm::{Normalize, LogNorm};
//!
//! let norm = LogNorm::new(1.0, 1000.0);
//! assert!((norm.normalize(1.0) - 0.0).abs() < 1e-10);
//! assert!((norm.normalize(1000.0) - 1.0).abs() < 1e-10);
//! assert!((norm.normalize(31.62) - 0.5).abs() < 0.01); // sqrt(1000)
//! ```

/// Trait for normalizing data values to [0, 1].
pub trait Normalize: Send + Sync {
    /// Map a data value to [0, 1]. Values outside bounds are clamped.
    fn normalize(&self, value: f64) -> f64;

    /// Clone this normalizer into a boxed trait object.
    fn box_clone(&self) -> Box<dyn Normalize>;
}

impl Clone for Box<dyn Normalize> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// Linear normalization: linearly maps [vmin, vmax] → [0, 1].
#[derive(Clone, Debug)]
pub struct LinearNorm {
    pub vmin: f64,
    pub vmax: f64,
}

impl LinearNorm {
    pub fn new(vmin: f64, vmax: f64) -> Self {
        Self { vmin, vmax }
    }
}

impl Normalize for LinearNorm {
    fn normalize(&self, value: f64) -> f64 {
        if self.vmax == self.vmin {
            return 0.5;
        }
        ((value - self.vmin) / (self.vmax - self.vmin)).clamp(0.0, 1.0)
    }

    fn box_clone(&self) -> Box<dyn Normalize> {
        Box::new(self.clone())
    }
}

/// Logarithmic normalization: maps [vmin, vmax] → [0, 1] on a log₁₀ scale.
///
/// Both vmin and vmax must be positive.
#[derive(Clone, Debug)]
pub struct LogNorm {
    pub vmin: f64,
    pub vmax: f64,
}

impl LogNorm {
    pub fn new(vmin: f64, vmax: f64) -> Self {
        assert!(vmin > 0.0, "LogNorm requires positive vmin");
        assert!(vmax > 0.0, "LogNorm requires positive vmax");
        Self { vmin, vmax }
    }
}

impl Normalize for LogNorm {
    fn normalize(&self, value: f64) -> f64 {
        if value <= 0.0 {
            return 0.0;
        }
        let log_min = self.vmin.log10();
        let log_max = self.vmax.log10();
        if log_max == log_min {
            return 0.5;
        }
        ((value.log10() - log_min) / (log_max - log_min)).clamp(0.0, 1.0)
    }

    fn box_clone(&self) -> Box<dyn Normalize> {
        Box::new(self.clone())
    }
}

/// Symmetric log normalization: linear near zero, logarithmic away from it.
///
/// Essential for data with both positive and negative values spanning decades.
#[derive(Clone, Debug)]
pub struct SymLogNorm {
    /// Threshold below which the mapping is linear.
    pub lin_thresh: f64,
    /// Stretch factor for the linear region.
    pub lin_scale: f64,
    pub vmin: f64,
    pub vmax: f64,
    pub base: f64,
}

impl SymLogNorm {
    pub fn new(lin_thresh: f64, vmin: f64, vmax: f64) -> Self {
        Self {
            lin_thresh,
            lin_scale: 1.0,
            vmin,
            vmax,
            base: 10.0,
        }
    }

    fn transform(&self, value: f64) -> f64 {
        let log_base = self.base.ln();
        if value.abs() <= self.lin_thresh {
            value * self.lin_scale
        } else if value > 0.0 {
            self.lin_thresh * self.lin_scale + (value / self.lin_thresh).ln() / log_base
        } else {
            -(self.lin_thresh * self.lin_scale + (-value / self.lin_thresh).ln() / log_base)
        }
    }
}

impl Normalize for SymLogNorm {
    fn normalize(&self, value: f64) -> f64 {
        let t_min = self.transform(self.vmin);
        let t_max = self.transform(self.vmax);
        let t_val = self.transform(value);
        if t_max == t_min {
            return 0.5;
        }
        ((t_val - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
    }

    fn box_clone(&self) -> Box<dyn Normalize> {
        Box::new(self.clone())
    }
}

/// Power-law normalization: y = x^gamma.
///
/// Gamma < 1 emphasizes low values, gamma > 1 emphasizes high values.
#[derive(Clone, Debug)]
pub struct PowerNorm {
    pub gamma: f64,
    pub vmin: f64,
    pub vmax: f64,
}

impl PowerNorm {
    pub fn new(gamma: f64, vmin: f64, vmax: f64) -> Self {
        Self { gamma, vmin, vmax }
    }
}

impl Normalize for PowerNorm {
    fn normalize(&self, value: f64) -> f64 {
        if self.vmax == self.vmin {
            return 0.5;
        }
        let t = ((value - self.vmin) / (self.vmax - self.vmin)).clamp(0.0, 1.0);
        t.powf(self.gamma)
    }

    fn box_clone(&self) -> Box<dyn Normalize> {
        Box::new(self.clone())
    }
}

/// Boundary normalization: maps values to discrete bins defined by boundaries.
///
/// Useful for classified/categorical color mapping (e.g., elevation bands).
#[derive(Clone, Debug)]
pub struct BoundaryNorm {
    /// Monotonically increasing boundary values.
    pub boundaries: Vec<f64>,
}

impl BoundaryNorm {
    pub fn new(boundaries: Vec<f64>) -> Self {
        Self { boundaries }
    }
}

impl Normalize for BoundaryNorm {
    fn normalize(&self, value: f64) -> f64 {
        if self.boundaries.len() < 2 {
            return 0.5;
        }
        let n = self.boundaries.len() - 1;
        for i in 0..n {
            if value < self.boundaries[i + 1] {
                return i as f64 / n as f64;
            }
        }
        1.0
    }

    fn box_clone(&self) -> Box<dyn Normalize> {
        Box::new(self.clone())
    }
}

/// Centered normalization: symmetric around a center value.
///
/// Maps [vmin, vcenter] to [0, 0.5] and [vcenter, vmax] to [0.5, 1.0],
/// ensuring the center maps to 0.5. Useful for diverging colormaps.
#[derive(Clone, Debug)]
pub struct CenteredNorm {
    pub vcenter: f64,
    pub halfrange: f64,
}

impl CenteredNorm {
    /// Create a centered norm. `halfrange` is the distance from center to each extreme.
    /// If 0, it will be computed as max(|vmin - vcenter|, |vmax - vcenter|).
    pub fn new(vcenter: f64, halfrange: f64) -> Self {
        Self { vcenter, halfrange }
    }

    /// Create from data bounds, automatically computing the symmetric range.
    pub fn from_bounds(vcenter: f64, vmin: f64, vmax: f64) -> Self {
        let halfrange = (vmin - vcenter).abs().max((vmax - vcenter).abs());
        Self { vcenter, halfrange }
    }
}

impl Normalize for CenteredNorm {
    fn normalize(&self, value: f64) -> f64 {
        if self.halfrange == 0.0 {
            return 0.5;
        }
        (0.5 + 0.5 * (value - self.vcenter) / self.halfrange).clamp(0.0, 1.0)
    }

    fn box_clone(&self) -> Box<dyn Normalize> {
        Box::new(self.clone())
    }
}

/// Inverse hyperbolic sine normalization. Matches the `Asinh` scale.
///
/// Provides smooth transition between linear (near zero) and logarithmic behavior.
#[derive(Clone, Debug)]
pub struct AsinhNorm {
    pub linear_width: f64,
    pub vmin: f64,
    pub vmax: f64,
}

impl AsinhNorm {
    pub fn new(linear_width: f64, vmin: f64, vmax: f64) -> Self {
        Self {
            linear_width,
            vmin,
            vmax,
        }
    }
}

impl Normalize for AsinhNorm {
    fn normalize(&self, value: f64) -> f64 {
        let t_min = (self.vmin / self.linear_width).asinh();
        let t_max = (self.vmax / self.linear_width).asinh();
        let t_val = (value / self.linear_width).asinh();
        if t_max == t_min {
            return 0.5;
        }
        ((t_val - t_min) / (t_max - t_min)).clamp(0.0, 1.0)
    }

    fn box_clone(&self) -> Box<dyn Normalize> {
        Box::new(self.clone())
    }
}

/// User-defined normalization function.
///
/// Wraps an arbitrary function that maps values to [0, 1].
pub struct FuncNorm {
    func: Box<dyn Fn(f64) -> f64 + Send + Sync>,
}

impl FuncNorm {
    pub fn new(f: impl Fn(f64) -> f64 + Send + Sync + 'static) -> Self {
        Self { func: Box::new(f) }
    }
}

impl Normalize for FuncNorm {
    fn normalize(&self, value: f64) -> f64 {
        (self.func)(value).clamp(0.0, 1.0)
    }

    fn box_clone(&self) -> Box<dyn Normalize> {
        // FuncNorm cannot be cloned; return a LinearNorm fallback.
        Box::new(LinearNorm::new(0.0, 1.0))
    }
}

impl std::fmt::Debug for FuncNorm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuncNorm").finish()
    }
}

/// Two-slope normalization: different rates above and below a center value.
///
/// Critical for diverging colormaps where the center represents a meaningful value
/// (e.g., zero velocity, reference temperature).
#[derive(Clone, Debug)]
pub struct TwoSlopeNorm {
    pub vcenter: f64,
    pub vmin: f64,
    pub vmax: f64,
}

impl TwoSlopeNorm {
    pub fn new(vcenter: f64, vmin: f64, vmax: f64) -> Self {
        assert!(vmin <= vcenter && vcenter <= vmax);
        Self {
            vcenter,
            vmin,
            vmax,
        }
    }
}

impl Normalize for TwoSlopeNorm {
    fn normalize(&self, value: f64) -> f64 {
        if value <= self.vcenter {
            if self.vcenter == self.vmin {
                0.5
            } else {
                0.5 * (value - self.vmin) / (self.vcenter - self.vmin)
            }
        } else if self.vmax == self.vcenter {
            0.5
        } else {
            0.5 + 0.5 * (value - self.vcenter) / (self.vmax - self.vcenter)
        }
        .clamp(0.0, 1.0)
    }

    fn box_clone(&self) -> Box<dyn Normalize> {
        Box::new(self.clone())
    }
}
