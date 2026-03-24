//! Inset axes — render a sub-widget within a fractional sub-area of the parent.
//!
//! Useful for zoomed views, mini-maps, or auxiliary plots placed inside a
//! larger plot area.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::inset::InsetAxes;
//!
//! // Place an inset in the top-right quadrant (60%-95% x, 5%-40% y)
//! let inset = InsetAxes::new(0.6, 0.05, 0.35, 0.35);
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::theme::Theme;

/// Configuration for an inset axes area within a parent widget.
///
/// Bounds are fractional (0.0–1.0) relative to the parent area.
#[derive(Clone, Debug)]
pub struct InsetAxes {
    /// Fractional x position of the inset's left edge.
    pub x: f64,
    /// Fractional y position of the inset's top edge.
    pub y: f64,
    /// Fractional width of the inset.
    pub width: f64,
    /// Fractional height of the inset.
    pub height: f64,
    /// Whether to draw a border around the inset.
    pub border: bool,
    /// Border color.
    pub border_color: Color,
    /// Theme for character set lookup.
    theme: Theme,
}

impl InsetAxes {
    /// Create an inset with fractional bounds (0.0–1.0) within the parent area.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        let theme = Theme::get_default();
        Self {
            x: x.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
            width: width.clamp(0.0, 1.0),
            height: height.clamp(0.0, 1.0),
            border: true,
            border_color: theme.muted,
            theme,
        }
    }

    /// Enable or disable the inset border.
    pub fn border(mut self, show: bool) -> Self {
        self.border = show;
        self
    }

    /// Set the border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// Compute the screen `Rect` for this inset within the given parent area.
    pub fn rect(&self, parent: Rect) -> Rect {
        let x = parent.x + (self.x * parent.width as f64).round() as u16;
        let y = parent.y + (self.y * parent.height as f64).round() as u16;
        let w = (self.width * parent.width as f64).round() as u16;
        let h = (self.height * parent.height as f64).round() as u16;
        Rect::new(
            x.min(parent.x + parent.width),
            y.min(parent.y + parent.height),
            w.min(parent.x + parent.width - x),
            h.min(parent.y + parent.height - y),
        )
    }

    /// Render a widget inside this inset area.
    ///
    /// Clears the inset background and optionally draws a border, then
    /// delegates to the provided render function.
    pub fn render_with(
        &self,
        parent: Rect,
        buf: &mut Buffer,
        render: impl FnOnce(Rect, &mut Buffer),
    ) {
        let rect = self.rect(parent);
        if rect.width < 2 || rect.height < 2 {
            return;
        }

        // Clear background
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                if x < parent.x + parent.width && y < parent.y + parent.height {
                    buf[(x, y)].set_char(' ');
                }
            }
        }

        // Draw border
        if self.border {
            let bc = self.border_color;
            let border = &self.theme.chars.border;
            let r = rect;
            if r.width >= 2 && r.height >= 2 {
                buf[(r.x, r.y)].set_char(border.top_left).set_fg(bc);
                buf[(r.x + r.width - 1, r.y)]
                    .set_char(border.top_right)
                    .set_fg(bc);
                buf[(r.x, r.y + r.height - 1)]
                    .set_char(border.bottom_left)
                    .set_fg(bc);
                buf[(r.x + r.width - 1, r.y + r.height - 1)]
                    .set_char(border.bottom_right)
                    .set_fg(bc);
                for x in r.x + 1..r.x + r.width - 1 {
                    buf[(x, r.y)].set_char(border.horizontal).set_fg(bc);
                    buf[(x, r.y + r.height - 1)]
                        .set_char(border.horizontal)
                        .set_fg(bc);
                }
                for y in r.y + 1..r.y + r.height - 1 {
                    buf[(r.x, y)].set_char(border.vertical).set_fg(bc);
                    buf[(r.x + r.width - 1, y)]
                        .set_char(border.vertical)
                        .set_fg(bc);
                }

                // Render content inside the border
                let inner = Rect::new(r.x + 1, r.y + 1, r.width - 2, r.height - 2);
                render(inner, buf);
            }
        } else {
            render(rect, buf);
        }
    }
}
