//! Global plot configuration (rcParams-like).
//!
//! Provides a thread-local default configuration that widgets read during
//! construction, similar to matplotlib's `rcParams` system.

use crate::legend::LegendPosition;
use crate::style::MarkerShape;
use crate::theme::Theme;

/// Global plot configuration controlling default widget behavior.
///
/// # Example
///
/// ```
/// use ratatui_plt::config::PlotConfig;
///
/// // Modify global defaults
/// let mut cfg = PlotConfig::get_default();
/// cfg.grid_visible = true;
/// cfg.legend_visible = false;
/// PlotConfig::set_default(cfg);
/// ```
#[derive(Clone, Debug)]
pub struct PlotConfig {
    /// Default theme for all widgets.
    pub theme: Theme,
    /// Default line width (maps to Thickness).
    pub default_line_width: u16,
    /// Default marker shape.
    pub default_marker: Option<MarkerShape>,
    /// Whether grid is visible by default.
    pub grid_visible: bool,
    /// Whether legend is visible by default.
    pub legend_visible: bool,
    /// Default legend position.
    pub legend_position: LegendPosition,
    /// Default colormap name.
    pub colormap: String,
}

impl Default for PlotConfig {
    fn default() -> Self {
        Self {
            theme: Theme::get_default(),
            default_line_width: 1,
            default_marker: None,
            grid_visible: false,
            legend_visible: true,
            legend_position: LegendPosition::TopRight,
            colormap: "viridis".to_string(),
        }
    }
}

impl PlotConfig {
    /// Set the global default configuration.
    pub fn set_default(config: PlotConfig) {
        DEFAULT_CONFIG.with(|c| {
            *c.borrow_mut() = config;
        });
    }

    /// Get a clone of the current global default configuration.
    pub fn get_default() -> PlotConfig {
        DEFAULT_CONFIG.with(|c| c.borrow().clone())
    }
}

std::thread_local! {
    static DEFAULT_CONFIG: std::cell::RefCell<PlotConfig> = std::cell::RefCell::new(PlotConfig::default());
}

/// RAII guard that restores the previous [`PlotConfig`] when dropped.
///
/// Created by [`PlotConfig::activate`].
pub struct ConfigGuard {
    previous: PlotConfig,
}

impl Drop for ConfigGuard {
    fn drop(&mut self) {
        PlotConfig::set_default(self.previous.clone());
    }
}

impl PlotConfig {
    /// Activate this configuration as the global default, returning a guard
    /// that restores the previous configuration when dropped.
    pub fn activate(self) -> ConfigGuard {
        let previous = PlotConfig::get_default();
        PlotConfig::set_default(self);
        ConfigGuard { previous }
    }
}
