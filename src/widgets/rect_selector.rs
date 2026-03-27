//! Rectangle selector overlay widget for 2D region selection on a plot.
//!
//! Draws a filled rectangle with an optional box-drawing border on top of
//! an existing plot, using a shared [`BrushState`](crate::brushing::BrushState)
//! for the selection region.

use ratatui::buffer::Buffer;
use ratatui::style::Color;

use crate::brushing::SharedBrush;
use crate::frame::PlotArea;
use crate::theme::Theme;

/// A rectangle selector overlay widget.
///
/// Given a shared brush state containing a selection rectangle, draws a filled
/// region with an optional border. This is designed to be rendered *on top of*
/// an existing plot by calling [`RectangleSelector::render_on`] with a
/// [`PlotArea`] reference obtained from [`PlotFrame::render`](crate::frame::PlotFrame::render).
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::rect_selector::RectangleSelector;
/// use ratatui_plt::brushing::shared_brush;
/// use ratatui::style::Color;
///
/// let brush = shared_brush();
/// brush.borrow_mut().set_selection(1.0, 2.0, 5.0, 8.0);
///
/// let selector = RectangleSelector::new(brush)
///     .color(Color::Yellow)
///     .border(true);
/// ```
pub struct RectangleSelector {
    brush: SharedBrush,
    color: Color,
    fill_char: Option<char>,
    border: bool,
    theme: Theme,
}

impl RectangleSelector {
    /// Create a new rectangle selector with the given shared brush state.
    pub fn new(brush: SharedBrush) -> Self {
        let theme = Theme::get_default();
        Self {
            brush,
            color: theme.accent,
            fill_char: None,
            border: true,
            theme,
        }
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

    /// Enable or disable the box-drawing border.
    pub fn border(mut self, show: bool) -> Self {
        self.border = show;
        self
    }

    /// Render the rectangle selector overlay on a plot area.
    ///
    /// This should be called after the main widget has been rendered,
    /// using the `PlotArea` returned by `PlotFrame::render`.
    pub fn render_on(&self, pa: &PlotArea, buf: &mut Buffer) {
        let state = self.brush.borrow();
        let (x_min, y_min, x_max, y_max) = match state.selection {
            Some(sel) => sel,
            None => return,
        };

        // Map data coordinates to screen coordinates
        let sx_lo = pa.screen_x(x_min).round() as u16;
        let sx_hi = pa.screen_x(x_max).round() as u16;
        let sy_lo = pa.screen_y(y_max).round() as u16; // high data = low screen y
        let sy_hi = pa.screen_y(y_min).round() as u16; // low data = high screen y

        // Clamp to plot area bounds
        let x_start = sx_lo.max(pa.x);
        let x_end = (sx_hi + 1).min(pa.x + pa.width);
        let y_start = sy_lo.max(pa.y);
        let y_end = (sy_hi + 1).min(pa.y + pa.height);

        if x_start >= x_end || y_start >= y_end {
            return;
        }

        let fill_char = self.fill_char.unwrap_or(self.theme.chars.fill.light);
        let border = &self.theme.chars.border;

        // Fill the rectangle interior
        for y in y_start..y_end {
            for x in x_start..x_end {
                buf[(x, y)].set_char(fill_char).set_fg(self.color);
            }
        }

        // Draw border if enabled
        if self.border && x_end > x_start && y_end > y_start {
            let left = x_start;
            let right = x_end - 1;
            let top = y_start;
            let bottom = y_end - 1;

            // Top and bottom edges
            for x in left + 1..right {
                if x < pa.x + pa.width {
                    buf[(x, top)].set_char(border.horizontal).set_fg(self.color);
                    buf[(x, bottom)]
                        .set_char(border.horizontal)
                        .set_fg(self.color);
                }
            }

            // Left and right edges
            for y in top + 1..bottom {
                if y < pa.y + pa.height {
                    buf[(left, y)].set_char(border.vertical).set_fg(self.color);
                    buf[(right, y)].set_char(border.vertical).set_fg(self.color);
                }
            }

            // Corners
            buf[(left, top)]
                .set_char(border.top_left)
                .set_fg(self.color);
            if right < pa.x + pa.width {
                buf[(right, top)]
                    .set_char(border.top_right)
                    .set_fg(self.color);
            }
            if bottom < pa.y + pa.height {
                buf[(left, bottom)]
                    .set_char(border.bottom_left)
                    .set_fg(self.color);
            }
            if right < pa.x + pa.width && bottom < pa.y + pa.height {
                buf[(right, bottom)]
                    .set_char(border.bottom_right)
                    .set_fg(self.color);
            }
        }
    }
}
