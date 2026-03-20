//! Axis configuration: labels, bounds, scales, ticks, aspect ratio.
//!
//! The axis system is inspired by matplotlib's Axes API, providing logarithmic
//! and symmetric-log scales, auto-ticking, and aspect ratio control that
//! compensates for terminal cell geometry.

use crate::ticker::{
    LogFormatter, LogLocator, MaxNLocator, ScalarFormatter, TickFormatter, TickLocator,
};

/// Axis scale types.
#[derive(Clone, Debug, Default)]
pub enum Scale {
    /// Linear scale (default).
    #[default]
    Linear,
    /// Logarithmic scale with given base (typically 10).
    Log(f64),
    /// Symmetric log: linear near zero, logarithmic away from it.
    /// Useful for data with both positive and negative values spanning decades.
    SymLog {
        /// Threshold below which the scale is linear.
        lin_thresh: f64,
        /// Stretch factor for the linear region.
        lin_scale: f64,
        /// Logarithm base.
        base: f64,
    },
    /// Power-law scale: y = x^gamma.
    Power(f64),
}

impl Scale {
    /// Transform a value according to this scale.
    pub fn transform(&self, value: f64) -> f64 {
        match self {
            Self::Linear => value,
            Self::Log(base) => {
                if value <= 0.0 {
                    f64::NEG_INFINITY
                } else {
                    value.log(*base)
                }
            }
            Self::SymLog {
                lin_thresh,
                lin_scale,
                base,
            } => {
                let log_base = base.ln();
                if value.abs() <= *lin_thresh {
                    value * *lin_scale
                } else if value > 0.0 {
                    lin_thresh * lin_scale + (value / lin_thresh).ln() / log_base
                } else {
                    -(lin_thresh * lin_scale + (-value / lin_thresh).ln() / log_base)
                }
            }
            Self::Power(gamma) => {
                if value >= 0.0 {
                    value.powf(*gamma)
                } else {
                    -((-value).powf(*gamma))
                }
            }
        }
    }

    /// Inverse transform (from scaled space back to data space).
    pub fn inverse(&self, value: f64) -> f64 {
        match self {
            Self::Linear => value,
            Self::Log(base) => base.powf(value),
            Self::SymLog {
                lin_thresh,
                lin_scale,
                base,
            } => {
                let log_base = base.ln();
                let lin_boundary = lin_thresh * lin_scale;
                if value.abs() <= lin_boundary {
                    value / lin_scale
                } else if value > 0.0 {
                    lin_thresh * ((value - lin_boundary) * log_base).exp()
                } else {
                    -lin_thresh * ((-value - lin_boundary) * log_base).exp()
                }
            }
            Self::Power(gamma) => {
                if value >= 0.0 {
                    value.powf(1.0 / gamma)
                } else {
                    -((-value).powf(1.0 / gamma))
                }
            }
        }
    }
}

/// Axis bounds specification.
#[derive(Clone, Debug, Default)]
pub enum Bounds {
    /// Automatically determined from data.
    #[default]
    Auto,
    /// Manually specified (min, max).
    Manual(f64, f64),
}

/// Aspect ratio control for plots.
///
/// Terminal cells are typically ~2:1 (height:width in pixels), so `Equal`
/// automatically compensates to produce visually square data units.
#[derive(Clone, Debug, Default)]
pub enum AspectRatio {
    /// Aspect ratio determined by available area (default).
    #[default]
    Auto,
    /// Equal scaling: one data unit in x equals one data unit in y visually.
    /// Compensates for terminal cell aspect ratio (~2:1).
    Equal,
    /// Fixed ratio: x_scale / y_scale.
    Fixed(f64),
}

/// Terminal cell aspect ratio (width / height in pixels).
/// Most terminals have cells approximately twice as tall as wide.
pub const TERMINAL_CELL_ASPECT: f64 = 0.5;

/// Axis configuration.
///
/// # Example
///
/// ```
/// use ratatui_plt::axis::{Axis, Scale, Bounds};
///
/// let x_axis = Axis::new()
///     .label("Time (s)")
///     .scale(Scale::Linear)
///     .bounds(Bounds::Manual(0.0, 100.0))
///     .grid(true);
/// ```
#[derive(Clone)]
pub struct Axis {
    /// Axis label text.
    pub label: Option<String>,
    /// Data bounds.
    pub bounds: Bounds,
    /// Scale type.
    pub scale: Scale,
    /// Tick locator (determines tick positions).
    pub locator: Box<dyn TickLocator>,
    /// Tick formatter (determines tick label text).
    pub formatter: Box<dyn TickFormatter>,
    /// Whether to draw grid lines at tick positions.
    pub grid: bool,
    /// Whether the axis is visible.
    pub visible: bool,
    /// Whether the axis direction is inverted (max at start, min at end).
    pub inverted: bool,
    /// Minor grid configuration.
    pub minor_grid: bool,
    /// Number of minor ticks between major ticks.
    pub minor_tick_count: usize,
}

/// Configuration for major/minor grid lines.
#[derive(Clone, Debug)]
pub struct GridConfig {
    /// Show major grid lines.
    pub major: bool,
    /// Show minor grid lines.
    pub minor: bool,
    /// Number of minor divisions between major ticks.
    pub minor_count: usize,
}

impl std::fmt::Debug for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Axis")
            .field("label", &self.label)
            .field("bounds", &self.bounds)
            .field("scale", &self.scale)
            .field("grid", &self.grid)
            .field("visible", &self.visible)
            .finish()
    }
}

impl Default for Axis {
    fn default() -> Self {
        Self {
            label: None,
            bounds: Bounds::Auto,
            scale: Scale::Linear,
            locator: Box::new(MaxNLocator::new(8)),
            formatter: Box::new(ScalarFormatter),
            grid: false,
            visible: true,
            inverted: false,
            minor_grid: false,
            minor_tick_count: 4,
        }
    }
}

impl Axis {
    /// Create a new axis with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the axis label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the axis bounds.
    pub fn bounds(mut self, bounds: Bounds) -> Self {
        self.bounds = bounds;
        self
    }

    /// Set the axis scale.
    ///
    /// Automatically updates the tick locator and formatter for log scales.
    pub fn scale(mut self, scale: Scale) -> Self {
        if let Scale::Log(base) = &scale {
            self.locator = Box::new(LogLocator::new(*base));
            self.formatter = Box::new(LogFormatter::new(*base));
        }
        self.scale = scale;
        self
    }

    /// Set the tick locator.
    pub fn locator(mut self, locator: impl TickLocator + 'static) -> Self {
        self.locator = Box::new(locator);
        self
    }

    /// Set the tick formatter.
    pub fn formatter(mut self, formatter: impl TickFormatter + 'static) -> Self {
        self.formatter = Box::new(formatter);
        self
    }

    /// Enable or disable grid lines.
    pub fn grid(mut self, show: bool) -> Self {
        self.grid = show;
        self
    }

    /// Set visibility.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Invert the axis direction.
    pub fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }

    /// Enable or disable minor grid lines.
    pub fn minor_grid(mut self, show: bool) -> Self {
        self.minor_grid = show;
        self
    }

    /// Set the number of minor tick subdivisions between major ticks.
    pub fn minor_tick_count(mut self, count: usize) -> Self {
        self.minor_tick_count = count;
        self
    }

    /// Resolve bounds: if Auto, use the provided data range with padding.
    /// If inverted, swaps min and max.
    pub fn resolve_bounds(&self, data_min: f64, data_max: f64) -> (f64, f64) {
        let (lo, hi) = match &self.bounds {
            Bounds::Manual(min, max) => (*min, *max),
            Bounds::Auto => {
                if data_min == data_max {
                    (data_min - 1.0, data_max + 1.0)
                } else {
                    let padding = (data_max - data_min) * 0.05;
                    (data_min - padding, data_max + padding)
                }
            }
        };
        if self.inverted { (hi, lo) } else { (lo, hi) }
    }

    /// Compute tick positions for the given resolved bounds.
    pub fn tick_positions(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        let (lo, hi) = if vmin < vmax {
            (vmin, vmax)
        } else {
            (vmax, vmin)
        };
        self.locator.tick_values(lo, hi)
    }

    /// Compute minor tick positions between major ticks.
    pub fn minor_tick_positions(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        let major = self.tick_positions(vmin, vmax);
        if major.len() < 2 || self.minor_tick_count == 0 {
            return Vec::new();
        }
        let mut minor = Vec::new();
        for i in 0..major.len() - 1 {
            let step = (major[i + 1] - major[i]) / (self.minor_tick_count + 1) as f64;
            for j in 1..=self.minor_tick_count {
                let v = major[i] + step * j as f64;
                let (lo, hi) = if vmin < vmax {
                    (vmin, vmax)
                } else {
                    (vmax, vmin)
                };
                if v > lo && v < hi {
                    minor.push(v);
                }
            }
        }
        minor
    }

    /// Format a tick value.
    pub fn format_tick(&self, value: f64) -> String {
        self.formatter.format(value)
    }
}
