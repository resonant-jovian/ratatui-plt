//! Raw plotters escape hatch widget.
//!
//! Gives the user direct access to a plotters `DrawingArea` for
//! full control over chart rendering while still routing through
//! the ratatui-plt output pipeline.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::raw_plotters::RawPlotters;
//! use ratatui_plt::output::OutputMode;
//!
//! let raw = RawPlotters::new(|root| {
//!     use plotters::prelude::*;
//!     let Ok(mut chart) = ChartBuilder::on(root)
//!         .build_cartesian_2d(0.0..10.0, 0.0..10.0)
//!     else { return; };
//!     let _ = chart.configure_mesh().draw();
//! });
//! // frame.render_widget(&raw, area);
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::helpers;
use crate::output::{self, OutputMode, UnicodeMode};
use crate::theme::Theme;

/// A widget that gives the user direct access to a plotters DrawingArea.
pub struct RawPlotters<F> {
    draw_fn: F,
    theme: Theme,
    backend: Option<OutputMode>,
    unicode_mode: UnicodeMode,
}

impl<F> RawPlotters<F>
where
    F: Fn(
        &plotters::prelude::DrawingArea<
            crate::backend::TinySkiaDrawingBackend,
            plotters::coord::Shift,
        >,
    ),
{
    /// Create a new raw plotters widget with the given draw function.
    pub fn new(draw_fn: F) -> Self {
        Self {
            draw_fn,
            theme: Theme::get_default(),
            backend: None,
            unicode_mode: UnicodeMode::HalfBlock,
        }
    }

    /// Set the theme (used for background color).
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Override the rendering backend.
    pub fn backend(mut self, mode: OutputMode) -> Self {
        self.backend = Some(mode);
        self
    }

    /// Set the Unicode rendering sub-mode.
    pub fn unicode_mode(mut self, mode: UnicodeMode) -> Self {
        self.unicode_mode = mode;
        self
    }
}

impl<F> Widget for &RawPlotters<F>
where
    F: Fn(
        &plotters::prelude::DrawingArea<
            crate::backend::TinySkiaDrawingBackend,
            plotters::coord::Shift,
        >,
    ),
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode = self.backend.unwrap_or_default();
        let bg = helpers::theme_bg_rgb(&self.theme);
        let unicode_mode = self.unicode_mode;
        output::render_chart(area, buf, bg, mode, unicode_mode, |root| {
            (self.draw_fn)(root);
        });
    }
}
