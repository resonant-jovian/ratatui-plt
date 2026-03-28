//! Crosshair cursor overlay widget for coordinate readout.
//!
//! Draws horizontal and vertical dashed lines at a data coordinate position,
//! with an optional formatted coordinate label near the intersection.

use ratatui::buffer::Buffer;
use ratatui::style::Color;

use crate::frame::PlotArea;
use crate::theme::Theme;

/// A crosshair cursor overlay widget.
///
/// Given data coordinates, draws intersecting dashed lines across the plot
/// area with an optional coordinate readout label. This is designed to be
/// rendered *on top of* an existing plot by calling [`Crosshair::render_on`]
/// with a [`PlotArea`] reference obtained from [`PlotFrame::render`](crate::frame::PlotFrame::render).
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::crosshair::Crosshair;
/// use ratatui::style::Color;
///
/// let cursor = Crosshair::new(2.5, 3.7)
///     .color(Color::Yellow)
///     .show_labels(true)
///     .format(|x, y| format!("({:.2}, {:.2})", x, y));
/// ```
pub struct Crosshair {
    /// Cursor position in data coordinates (x).
    pub data_x: f64,
    /// Cursor position in data coordinates (y).
    pub data_y: f64,
    /// Crosshair line color.
    pub color: Color,
    /// Whether to show coordinate labels near the cursor.
    pub show_labels: bool,
    /// Custom format function for coordinate display.
    pub format: Option<Box<dyn Fn(f64, f64) -> String>>,
}

impl Crosshair {
    /// Create a new crosshair at the given data coordinates.
    pub fn new(data_x: f64, data_y: f64) -> Self {
        Self {
            data_x,
            data_y,
            color: Theme::get_default().highlight,
            show_labels: true,
            format: None,
        }
    }

    /// Set the crosshair color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set whether to show coordinate labels.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set a custom format function for coordinate display.
    pub fn format(mut self, f: impl Fn(f64, f64) -> String + 'static) -> Self {
        self.format = Some(Box::new(f));
        self
    }

    /// Render the crosshair overlay on a plot area.
    ///
    /// This should be called after the main widget has been rendered,
    /// using the `PlotArea` returned by `PlotFrame::render`.
    pub fn render_on(&self, pa: &PlotArea, buf: &mut Buffer) {
        #[cfg(feature = "plotters-render")]
        {
            if crate::plotters_render::should_use_plotters() {
                use crate::plotters_render::PlottersRenderable;
                let area = ratatui::layout::Rect::new(pa.x, pa.y, pa.width, pa.height);
                self.render_plotters(area, buf, &crate::theme::Theme::get_default());
                return;
            }
        }
        let theme = Theme::get_default();
        let sx = pa.screen_x(self.data_x);
        let sy = pa.screen_y(self.data_y);
        let xi = sx.round() as u16;
        let yi = sy.round() as u16;

        // Draw horizontal dashed line across the plot at the y position
        if yi >= pa.y && yi < pa.y + pa.height {
            for x in pa.x..pa.x + pa.width {
                // Dashed pattern: draw 2, skip 2
                if ((x - pa.x) % 4) < 2 {
                    // Skip the intersection point itself (drawn separately)
                    if x != xi {
                        buf[(x, yi)].set_char(theme.chars.dash.h).set_fg(self.color);
                    }
                }
            }
        }

        // Draw vertical dashed line down the plot at the x position
        if xi >= pa.x && xi < pa.x + pa.width {
            for y in pa.y..pa.y + pa.height {
                // Dashed pattern: draw 1, skip 1
                if (y - pa.y).is_multiple_of(2) {
                    // Skip the intersection point itself
                    if y != yi {
                        buf[(xi, y)].set_char(theme.chars.dash.v).set_fg(self.color);
                    }
                }
            }
        }

        // Draw intersection marker
        if pa.contains(xi, yi) {
            buf[(xi, yi)]
                .set_char(theme.chars.marker.default_point)
                .set_fg(self.color);
        }

        // Draw coordinate labels if enabled
        if self.show_labels && pa.contains(xi, yi) {
            let label = if let Some(ref fmt) = self.format {
                fmt(self.data_x, self.data_y)
            } else {
                format!("({:.2}, {:.2})", self.data_x, self.data_y)
            };

            // Position the label to the right and above the cursor if possible,
            // otherwise adjust to stay within the plot area.
            let label_len = label.len() as u16;

            // Try placing to the right and one row above
            let mut lx = xi + 2;
            let mut ly = yi.saturating_sub(1);

            // If it would overflow right, place to the left instead
            if lx + label_len >= pa.x + pa.width {
                lx = xi.saturating_sub(label_len + 1);
            }

            // If it would overflow top, place below instead
            if ly < pa.y {
                ly = yi + 1;
            }

            // Clamp to plot area
            if ly >= pa.y && ly < pa.y + pa.height {
                for (j, ch) in label.chars().enumerate() {
                    let cx = lx + j as u16;
                    if cx >= pa.x && cx < pa.x + pa.width {
                        buf[(cx, ly)].set_char(ch).set_fg(self.color);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for Crosshair {
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
