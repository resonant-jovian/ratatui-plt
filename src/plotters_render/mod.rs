//! High-quality pixel rendering via plotters + tiny-skia.
//!
//! This module provides an alternative rendering pipeline that uses
//! the `plotters` crate for chart layout and `tiny-skia` for
//! anti-aliased pixel rendering. The result is displayed in-terminal
//! via the Kitty graphics protocol.
//!
//! # Feature
//!
//! Requires the `plotters-render` cargo feature:
//!
//! ```toml
//! ratatui-plt = { version = "0.1", features = ["plotters-render"] }
//! ```
//!
//! # Architecture
//!
//! Each widget implements [`PlottersRenderable`] to provide a
//! plotters-based rendering path. When the `plotters-render` feature
//! is enabled and a Kitty-capable terminal is detected, widgets
//! automatically use this path for pixel-perfect output.
//!
//! The fallback is the existing character-based rendering via
//! [`PlotBackend`](crate::plot_buffer::PlotBackend).

pub mod backend;
pub mod bridge;
pub mod helpers;
pub mod theme_bridge;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::theme::Theme;

/// Trait for widgets that support high-quality plotters rendering.
///
/// Widgets implement this alongside the standard ratatui `Widget`
/// trait. When a Kitty-capable terminal is detected, the dispatch
/// macro calls `render_plotters()` instead of the character-based
/// path.
///
/// # Example
///
/// ```ignore
/// impl PlottersRenderable for LinePlot {
///     fn render_plotters(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
///         bridge::render_plotters_to_buf(area, buf, theme_bridge::theme_bg_rgb(theme), |root| {
///             let mut chart = helpers::build_cartesian_2d(
///                 root, &self.x_axis, &self.y_axis,
///                 self.title.as_deref(), theme,
///                 x_range, y_range,
///             ).unwrap();
///             // Draw data series...
///         });
///     }
/// }
/// ```
pub trait PlottersRenderable {
    /// Render the widget using plotters + tiny-skia into a ratatui
    /// Buffer via Kitty graphics protocol.
    fn render_plotters(
        &self,
        area: Rect,
        buf: &mut Buffer,
        theme: &Theme,
    );
}

/// Check whether plotters rendering should be used.
///
/// Returns `true` when:
/// 1. The `plotters-render` feature is enabled (compile-time)
/// 2. The detected backend is Kitty (runtime)
/// 3. stdout is a terminal OR `RATATUI_PLT_PLOTTERS=1` is set
///    (allows headless export with plotters quality)
pub fn should_use_plotters() -> bool {
    use std::io::IsTerminal;
    use crate::config::{PlotConfig, RenderBackend, detect_backend};

    // Allow forcing plotters mode for headless export.
    let force_plotters =
        std::env::var("RATATUI_PLT_PLOTTERS").is_ok();

    // Kitty protocol only works in a real terminal, but for
    // headless export with plotters, we render to pixmap and the
    // buffer_to_png/svg path captures the result.
    if !force_plotters && !std::io::stdout().is_terminal() {
        return false;
    }

    let cfg = PlotConfig::get_default();
    let backend = match cfg.render_backend {
        RenderBackend::Auto => {
            if force_plotters {
                RenderBackend::Kitty
            } else {
                detect_backend()
            }
        }
        other => other,
    };
    matches!(backend, RenderBackend::Kitty)
}

/// Dispatch macro for widget rendering.
///
/// When `plotters-render` is enabled and Kitty is detected, calls
/// `render_plotters()`. Otherwise falls through to the existing
/// character-based rendering.
///
/// # Usage
///
/// ```ignore
/// impl Widget for &MyWidget {
///     fn render(self, area: Rect, buf: &mut Buffer) {
///         plotters_dispatch!(self, area, buf, &self.theme);
///         // ... existing character-based rendering below ...
///     }
/// }
/// ```
#[macro_export]
macro_rules! plotters_dispatch {
    ($self:expr, $area:expr, $buf:expr, $theme:expr) => {
        #[cfg(feature = "plotters-render")]
        {
            if $crate::plotters_render::should_use_plotters() {
                use $crate::plotters_render::PlottersRenderable;
                $self.render_plotters($area, $buf, $theme);
                return;
            }
        }
    };
}
