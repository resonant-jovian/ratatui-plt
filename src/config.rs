//! Global plot configuration (rcParams-like).
//!
//! Provides a thread-local default configuration that widgets read during
//! construction, similar to matplotlib's `rcParams` system.
//!
//! # Rendering Backends
//!
//! The [`RenderBackend`] enum selects the rendering strategy:
//! - [`RenderBackend::Unicode`] — Braille sub-pixel dots and half-block characters (default, works everywhere)
//! - [`RenderBackend::Kitty`] — Pixel-level rendering via Kitty graphics protocol (requires `kitty` feature)
//! - [`RenderBackend::Sixel`] — Pixel-level rendering via Sixel protocol (requires `sixel` feature)
//! - [`RenderBackend::Auto`] — Auto-detect best available backend from terminal environment

use crate::legend::LegendPosition;
use crate::style::MarkerShape;
use crate::theme::Theme;

/// Rendering backend selection for plot widgets.
///
/// Controls how plot data is rendered to the terminal. The default is
/// [`RenderBackend::Unicode`] which uses Braille characters for lines and
/// half-block characters for fills, working in all terminals.
///
/// When `kitty` or `sixel` features are enabled, pixel-level rendering
/// backends are available for higher-fidelity output.
///
/// # Example
///
/// ```
/// use ratatui_plt::config::{PlotConfig, RenderBackend};
///
/// let mut cfg = PlotConfig::get_default();
/// cfg.render_backend = RenderBackend::Auto;
/// PlotConfig::set_default(cfg);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum RenderBackend {
    /// Auto-detect best available backend from terminal environment variables.
    /// Falls back to Unicode if no graphics protocol is detected.
    Auto,
    /// Braille sub-pixel dots and half-block characters. Works in all terminals.
    #[default]
    Unicode,
    /// Pixel-level rendering via Kitty graphics protocol.
    /// Requires the `kitty` feature. Falls back to Unicode if feature is not enabled.
    Kitty,
    /// Pixel-level rendering via Sixel protocol.
    /// Requires the `sixel` feature. Falls back to Unicode if feature is not enabled.
    Sixel,
}

/// Detect if the terminal supports Kitty graphics protocol.
///
/// Checks the `TERM_PROGRAM` environment variable for known Kitty-compatible terminals.
pub fn detect_kitty() -> bool {
    matches!(
        std::env::var("TERM_PROGRAM").ok().as_deref(),
        Some("kitty") | Some("WezTerm") | Some("Ghostty")
    )
}

/// Detect if the terminal supports the Sixel protocol.
///
/// Checks for the `SIXEL_SUPPORT` environment variable or known Sixel terminals.
pub fn detect_sixel() -> bool {
    if std::env::var("SIXEL_SUPPORT").is_ok() {
        return true;
    }
    matches!(
        std::env::var("TERM_PROGRAM").ok().as_deref(),
        Some("foot") | Some("mlterm") | Some("contour")
    )
}

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
    /// Rendering backend for plot widgets.
    pub render_backend: RenderBackend,
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
            render_backend: RenderBackend::default(),
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
