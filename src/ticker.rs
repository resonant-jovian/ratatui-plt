//! Tick locators and formatters for axis labeling.
//!
//! Inspired by matplotlib's Locator/Formatter system. Locators determine where
//! ticks are placed; formatters determine how tick values are displayed as text.
//!
//! # Example
//!
//! ```
//! use ratatui_sim::ticker::{MaxNLocator, TickLocator};
//!
//! let locator = MaxNLocator::new(5);
//! let ticks = locator.tick_values(0.0, 100.0);
//! assert!(ticks.len() <= 6); // at most n+1 ticks
//! ```

/// Trait for determining tick positions along an axis.
pub trait TickLocator: Send + Sync {
    /// Compute tick positions for the range [vmin, vmax].
    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64>;

    /// Clone into a boxed trait object.
    fn box_clone(&self) -> Box<dyn TickLocator>;
}

impl Clone for Box<dyn TickLocator> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// Trait for formatting tick values as text.
pub trait TickFormatter: Send + Sync {
    /// Format a tick value as a display string.
    fn format(&self, value: f64) -> String;

    /// Clone into a boxed trait object.
    fn box_clone(&self) -> Box<dyn TickFormatter>;
}

impl Clone for Box<dyn TickFormatter> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// Locator that picks at most N "nice" tick values.
///
/// Produces round numbers (multiples of 1, 2, 5 × 10^k) within the range.
#[derive(Clone, Debug)]
pub struct MaxNLocator {
    /// Maximum number of tick intervals (ticks = intervals + 1).
    pub n_ticks: usize,
}

impl MaxNLocator {
    pub fn new(n_ticks: usize) -> Self {
        Self { n_ticks }
    }
}

impl TickLocator for MaxNLocator {
    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        if vmin >= vmax || self.n_ticks == 0 {
            return vec![];
        }

        let range = vmax - vmin;
        let rough_step = range / self.n_ticks as f64;

        // Find the "nice" step size: 1, 2, or 5 × 10^k
        let exponent = rough_step.log10().floor();
        let fraction = rough_step / 10.0_f64.powf(exponent);

        let nice_fraction = if fraction <= 1.5 {
            1.0
        } else if fraction <= 3.5 {
            2.0
        } else if fraction <= 7.5 {
            5.0
        } else {
            10.0
        };

        let step = nice_fraction * 10.0_f64.powf(exponent);
        let start = (vmin / step).ceil() * step;

        let mut ticks = Vec::new();
        let mut v = start;
        while v <= vmax + step * 0.001 {
            // Round to avoid floating-point drift
            let rounded = (v / step).round() * step;
            if rounded >= vmin && rounded <= vmax {
                ticks.push(rounded);
            }
            v += step;
        }

        ticks
    }

    fn box_clone(&self) -> Box<dyn TickLocator> {
        Box::new(self.clone())
    }
}

/// Locator that places ticks at multiples of a base value.
#[derive(Clone, Debug)]
pub struct MultipleLocator {
    /// Tick spacing.
    pub base: f64,
}

impl MultipleLocator {
    pub fn new(base: f64) -> Self {
        Self { base }
    }
}

impl TickLocator for MultipleLocator {
    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        let start = (vmin / self.base).ceil() as i64;
        let end = (vmax / self.base).floor() as i64;
        (start..=end).map(|i| i as f64 * self.base).collect()
    }

    fn box_clone(&self) -> Box<dyn TickLocator> {
        Box::new(self.clone())
    }
}

/// Locator for logarithmic axes. Places ticks at base^n.
#[derive(Clone, Debug)]
pub struct LogLocator {
    /// Logarithm base.
    pub base: f64,
    /// Sub-decade ticks (e.g., [2, 5] for ticks at 2×10^n, 5×10^n).
    pub subs: Vec<f64>,
}

impl LogLocator {
    pub fn new(base: f64) -> Self {
        Self {
            base,
            subs: vec![1.0],
        }
    }

    /// Add sub-decade ticks.
    pub fn subs(mut self, subs: Vec<f64>) -> Self {
        self.subs = subs;
        self
    }
}

impl TickLocator for LogLocator {
    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        if vmin <= 0.0 || vmax <= 0.0 {
            return vec![];
        }
        let exp_min = vmin.log(self.base).floor() as i64;
        let exp_max = vmax.log(self.base).ceil() as i64;

        let mut ticks = Vec::new();
        for exp in exp_min..=exp_max {
            for &sub in &self.subs {
                let val = sub * self.base.powi(exp as i32);
                if val >= vmin && val <= vmax {
                    ticks.push(val);
                }
            }
        }
        ticks
    }

    fn box_clone(&self) -> Box<dyn TickLocator> {
        Box::new(self.clone())
    }
}

/// Locator at fixed positions.
#[derive(Clone, Debug)]
pub struct FixedLocator {
    pub values: Vec<f64>,
}

impl FixedLocator {
    pub fn new(values: Vec<f64>) -> Self {
        Self { values }
    }
}

impl TickLocator for FixedLocator {
    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        self.values
            .iter()
            .copied()
            .filter(|&v| v >= vmin && v <= vmax)
            .collect()
    }

    fn box_clone(&self) -> Box<dyn TickLocator> {
        Box::new(self.clone())
    }
}

/// Default scalar formatter: displays numbers with appropriate precision.
#[derive(Clone, Debug)]
pub struct ScalarFormatter;

impl TickFormatter for ScalarFormatter {
    fn format(&self, value: f64) -> String {
        if value == 0.0 {
            return "0".to_string();
        }
        let abs = value.abs();
        if !(1e-3..1e6).contains(&abs) {
            format!("{:.2e}", value)
        } else if abs >= 100.0 {
            format!("{:.0}", value)
        } else if abs >= 1.0 {
            format!("{:.1}", value)
        } else {
            format!("{:.3}", value)
        }
    }

    fn box_clone(&self) -> Box<dyn TickFormatter> {
        Box::new(self.clone())
    }
}

/// Formatter for logarithmic axes: displays as base^exponent.
#[derive(Clone, Debug)]
pub struct LogFormatter {
    pub base: f64,
}

impl LogFormatter {
    pub fn new(base: f64) -> Self {
        Self { base }
    }
}

impl TickFormatter for LogFormatter {
    fn format(&self, value: f64) -> String {
        if value <= 0.0 {
            return "0".to_string();
        }
        let exp = value.log(self.base).round();
        if (self.base.powf(exp) - value).abs() / value < 0.01 {
            if self.base == 10.0 {
                format!("10^{}", exp as i64)
            } else {
                format!("{}^{}", self.base as i64, exp as i64)
            }
        } else {
            format!("{:.2e}", value)
        }
    }

    fn box_clone(&self) -> Box<dyn TickFormatter> {
        Box::new(self.clone())
    }
}

/// User-provided formatting function.
pub struct FuncFormatter(pub Box<dyn Fn(f64) -> String + Send + Sync>);

impl FuncFormatter {
    pub fn new(f: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        Self(Box::new(f))
    }
}

impl TickFormatter for FuncFormatter {
    fn format(&self, value: f64) -> String {
        (self.0)(value)
    }

    fn box_clone(&self) -> Box<dyn TickFormatter> {
        // FuncFormatter cannot be cloned; return a ScalarFormatter as fallback.
        // Users should avoid cloning axes with FuncFormatter.
        Box::new(ScalarFormatter)
    }
}

/// Locator for categorical axes: places ticks at integer positions 0, 1, 2, ...
#[derive(Clone, Debug)]
pub struct CategoricalLocator {
    /// Category labels.
    pub categories: Vec<String>,
}

impl CategoricalLocator {
    pub fn new(categories: Vec<String>) -> Self {
        Self { categories }
    }
}

impl TickLocator for CategoricalLocator {
    fn tick_values(&self, _vmin: f64, _vmax: f64) -> Vec<f64> {
        (0..self.categories.len()).map(|i| i as f64).collect()
    }

    fn box_clone(&self) -> Box<dyn TickLocator> {
        Box::new(self.clone())
    }
}

/// Formatter for categorical axes: maps integer positions to category labels.
#[derive(Clone, Debug)]
pub struct CategoricalFormatter {
    pub categories: Vec<String>,
}

impl CategoricalFormatter {
    pub fn new(categories: Vec<String>) -> Self {
        Self { categories }
    }
}

impl TickFormatter for CategoricalFormatter {
    fn format(&self, value: f64) -> String {
        let idx = value.round() as usize;
        self.categories.get(idx).cloned().unwrap_or_else(|| format!("{}", idx))
    }

    fn box_clone(&self) -> Box<dyn TickFormatter> {
        Box::new(self.clone())
    }
}

/// SI prefix formatter: k, M, G, T, m, µ, n, etc.
#[derive(Clone, Debug)]
pub struct SiFormatter;

impl TickFormatter for SiFormatter {
    fn format(&self, value: f64) -> String {
        if value == 0.0 {
            return "0".to_string();
        }
        let abs = value.abs();
        let (suffix, divisor) = if abs >= 1e12 {
            ("T", 1e12)
        } else if abs >= 1e9 {
            ("G", 1e9)
        } else if abs >= 1e6 {
            ("M", 1e6)
        } else if abs >= 1e3 {
            ("k", 1e3)
        } else if abs >= 1.0 {
            ("", 1.0)
        } else if abs >= 1e-3 {
            ("m", 1e-3)
        } else if abs >= 1e-6 {
            ("\u{00b5}", 1e-6) // µ
        } else if abs >= 1e-9 {
            ("n", 1e-9)
        } else {
            ("p", 1e-12)
        };
        let scaled = value / divisor;
        if scaled.fract().abs() < 0.01 {
            format!("{:.0}{}", scaled, suffix)
        } else {
            format!("{:.1}{}", scaled, suffix)
        }
    }

    fn box_clone(&self) -> Box<dyn TickFormatter> {
        Box::new(self.clone())
    }
}
