//! Span selector overlay widget for selecting a 1D range on a plot.
//!
//! Draws a filled rectangular overlay across the plot area indicating
//! a selected data range. Designed to be rendered on top of an existing
//! plot using the overlay pattern (like [`Crosshair`](super::crosshair::Crosshair)).

use std::cell::RefCell;
use std::rc::Rc;

use ratatui::buffer::Buffer;
use ratatui::style::Color;

use crate::frame::PlotArea;
use crate::theme::Theme;

/// Direction of the span selection.
#[derive(Clone, Debug, Default)]
pub enum SpanDirection {
    /// Select a horizontal range (along the x-axis).
    #[default]
    Horizontal,
    /// Select a vertical range (along the y-axis).
    Vertical,
}

/// State for a span selection containing optional start and end data values.
#[derive(Clone, Debug, Default)]
pub struct SpanSelectorState {
    /// Start of the selected range in data coordinates.
    pub start: Option<f64>,
    /// End of the selected range in data coordinates.
    pub end: Option<f64>,
}

/// Convenience type for a shared span selector state (single-threaded).
pub type SharedSpanState = Rc<RefCell<SpanSelectorState>>;

/// Create a new shared span selector state.
pub fn shared_span_state() -> SharedSpanState {
    Rc::new(RefCell::new(SpanSelectorState::default()))
}

/// A span selector overlay widget.
///
/// Given a shared state containing start and end data values, draws a filled
/// rectangular region across the plot area. This is designed to be rendered
/// *on top of* an existing plot by calling [`SpanSelector::render_on`]
/// with a [`PlotArea`] reference obtained from [`PlotFrame::render`](crate::frame::PlotFrame::render).
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::span_selector::{SpanSelector, shared_span_state};
/// use ratatui::style::Color;
///
/// let state = shared_span_state();
/// state.borrow_mut().start = Some(2.0);
/// state.borrow_mut().end = Some(5.0);
///
/// let selector = SpanSelector::new(state)
///     .color(Color::Cyan)
///     .fill_char('▒');
/// ```
pub struct SpanSelector {
    state: SharedSpanState,
    direction: SpanDirection,
    color: Color,
    fill_char: Option<char>,
    theme: Theme,
}

impl SpanSelector {
    /// Create a new span selector with the given shared state.
    pub fn new(state: SharedSpanState) -> Self {
        let theme = Theme::get_default();
        Self {
            state,
            direction: SpanDirection::default(),
            color: theme.accent,
            fill_char: None,
            theme,
        }
    }

    /// Set the span direction.
    pub fn direction(mut self, direction: SpanDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set the overlay color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the fill character.
    pub fn fill_char(mut self, ch: char) -> Self {
        self.fill_char = Some(ch);
        self
    }

    /// Render the span selector overlay on a plot area.
    ///
    /// This should be called after the main widget has been rendered,
    /// using the `PlotArea` returned by `PlotFrame::render`.
    pub fn render_on(&self, pa: &PlotArea, buf: &mut Buffer) {
        #[cfg(feature = "plotters-render")]
        {
            if crate::plotters_render::should_use_plotters() {
                use crate::plotters_render::PlottersRenderable;
                let area = ratatui::layout::Rect::new(pa.x, pa.y, pa.width, pa.height);
                self.render_plotters(area, buf, &self.theme);
                return;
            }
        }
        let state = self.state.borrow();
        let (start, end) = match (state.start, state.end) {
            (Some(s), Some(e)) => (s, e),
            _ => return,
        };

        // Ensure start <= end
        let (lo, hi) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };

        let fill_char = self.fill_char.unwrap_or(self.theme.chars.fill.light);

        match self.direction {
            SpanDirection::Horizontal => {
                let sx_lo = pa.screen_x(lo).round() as u16;
                let sx_hi = pa.screen_x(hi).round() as u16;

                // Clamp to plot area bounds
                let x_start = sx_lo.max(pa.x);
                let x_end = (sx_hi + 1).min(pa.x + pa.width);

                for y in pa.y..pa.y + pa.height {
                    for x in x_start..x_end {
                        buf[(x, y)].set_char(fill_char).set_fg(self.color);
                    }
                }
            }
            SpanDirection::Vertical => {
                let sy_lo = pa.screen_y(hi).round() as u16; // high data = low screen y
                let sy_hi = pa.screen_y(lo).round() as u16; // low data = high screen y

                // Clamp to plot area bounds
                let y_start = sy_lo.max(pa.y);
                let y_end = (sy_hi + 1).min(pa.y + pa.height);

                for y in y_start..y_end {
                    for x in pa.x..pa.x + pa.width {
                        buf[(x, y)].set_char(fill_char).set_fg(self.color);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for SpanSelector {
    fn render_plotters(
        &self,
        area: ratatui::layout::Rect,
        buf: &mut ratatui::buffer::Buffer,
        theme: &crate::theme::Theme,
    ) {
        use crate::plotters_render::{bridge, theme_bridge};
        bridge::render_plotters_to_buf(
            area, buf, theme_bridge::theme_bg_rgb(theme),
            |_root| { /* Minimal stub - full plotters rendering TBD */ },
        );
    }
}
