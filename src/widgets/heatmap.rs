//! Heatmap widget with colormap and normalization support.
//!
//! Renders a 2D grid using half-block characters for doubled vertical resolution.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::axis::{AspectRatio, Axis};
use crate::colormap::{Colorbar, Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::series::GridData;
use crate::transform::apply_aspect_ratio;

/// A 2D heatmap widget.
///
/// # Example
///
/// ```
/// use ratatui_sim::prelude::*;
///
/// let data = GridData::from_fn((-2.0, 2.0), (-2.0, 2.0), 50, 50, |x, y| {
///     (-(x * x + y * y)).exp()
/// });
/// let heatmap = Heatmap::new(data)
///     .colormap(Viridis)
///     .title("2D Gaussian")
///     .aspect_ratio(AspectRatio::Equal);
/// ```
pub struct Heatmap {
    data: GridData,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_colorbar: bool,
    aspect_ratio: AspectRatio,
    show_values: bool,
    /// Color used for NaN/invalid cells.
    bad_color: Color,
}

impl Heatmap {
    /// Create a heatmap from grid data.
    pub fn new(data: GridData) -> Self {
        let (vmin, vmax) = data.value_bounds();
        Self {
            data,
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_colorbar: true,
            aspect_ratio: AspectRatio::Auto,
            show_values: false,
            bad_color: Color::DarkGray,
        }
    }

    /// Set the colormap.
    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    /// Set the normalization.
    pub fn norm(mut self, norm: impl Normalize + 'static) -> Self {
        self.norm = Box::new(norm);
        self
    }

    /// Set the X axis.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y axis.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Show or hide the colorbar.
    pub fn show_colorbar(mut self, show: bool) -> Self {
        self.show_colorbar = show;
        self
    }

    /// Set the aspect ratio.
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
        self
    }

    /// Show values in cells (only useful for small grids).
    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Set the color used for NaN/invalid cells.
    pub fn bad_color(mut self, color: Color) -> Self {
        self.bad_color = color;
        self
    }
}

impl Widget for &Heatmap {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 3 {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let colorbar_width: u16 = if self.show_colorbar { 10 } else { 0 };
        let y_label_width: u16 = 7;
        let tick_height: u16 = 1;

        let plot_x = area.x + y_label_width;
        let plot_y = area.y + title_height;
        let plot_width = area.width.saturating_sub(y_label_width + colorbar_width + 1);
        let plot_height = area.height.saturating_sub(title_height + tick_height);

        if plot_width < 2 || plot_height < 2 {
            return;
        }

        // Apply aspect ratio
        let data_x_range = if self.data.x.len() >= 2 {
            self.data.x.last().unwrap() - self.data.x.first().unwrap()
        } else {
            1.0
        };
        let data_y_range = if self.data.y.len() >= 2 {
            self.data.y.last().unwrap() - self.data.y.first().unwrap()
        } else {
            1.0
        };

        let (ax_off, ay_off, aw, ah) =
            apply_aspect_ratio(&self.aspect_ratio, data_x_range, data_y_range, plot_width, plot_height);
        let px = plot_x + ax_off;
        let py = plot_y + ay_off;

        // Draw title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(Color::White);
                }
            }
        }

        let nrows = self.data.nrows();
        let ncols = self.data.ncols();
        if nrows == 0 || ncols == 0 {
            return;
        }

        // Use half-block rendering: each character cell encodes two vertical pixels
        // ▀ = top half, ▄ = bottom half, █ = both same color
        // We double the effective vertical resolution.
        let effective_height = ah as usize * 2;

        for cy in 0..ah {
            for cx in 0..aw {
                let screen_x = px + cx;
                let screen_y = py + cy;
                if screen_x >= area.x + area.width || screen_y >= area.y + area.height {
                    continue;
                }

                // Top half-pixel
                let top_row_f = (cy as usize * 2) as f64 / effective_height as f64;
                let top_data_row = ((1.0 - top_row_f) * nrows as f64).min((nrows - 1) as f64) as usize;
                let top_data_col = (cx as f64 / aw as f64 * ncols as f64).min((ncols - 1) as f64) as usize;
                let top_val = self.data.values[top_data_row][top_data_col];
                let top_color = if top_val.is_finite() {
                    let top_t = self.norm.normalize(top_val);
                    self.colormap.color_at(top_t)
                } else {
                    self.bad_color
                };

                // Bottom half-pixel
                let bot_row_f = (cy as usize * 2 + 1) as f64 / effective_height as f64;
                let bot_data_row = ((1.0 - bot_row_f) * nrows as f64).min((nrows - 1) as f64) as usize;
                let bot_data_col = top_data_col;
                let bot_val = self.data.values[bot_data_row][bot_data_col];
                let bot_color = if bot_val.is_finite() {
                    let bot_t = self.norm.normalize(bot_val);
                    self.colormap.color_at(bot_t)
                } else {
                    self.bad_color
                };

                // Use ▀ (upper half block): fg = top color, bg = bottom color
                buf[(screen_x, screen_y)]
                    .set_char('▀')
                    .set_style(Style::default().fg(top_color).bg(bot_color));
            }
        }

        // Draw axis tick labels
        let x_ticks = self.x_axis.tick_positions(
            *self.data.x.first().unwrap_or(&0.0),
            *self.data.x.last().unwrap_or(&1.0),
        );
        let x_lo = *self.data.x.first().unwrap_or(&0.0);
        let x_hi = *self.data.x.last().unwrap_or(&1.0);
        for &tv in &x_ticks {
            let sx = crate::transform::data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let label_start = xi.saturating_sub(label.len() as u16 / 2);
            let y = py + ah;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        }

        let y_lo = *self.data.y.first().unwrap_or(&0.0);
        let y_hi = *self.data.y.last().unwrap_or(&1.0);
        let y_ticks = self.y_axis.tick_positions(y_lo, y_hi);
        for &tv in &y_ticks {
            let sy = crate::transform::data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
            let label = self.y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ah {
                let label_start = px.saturating_sub(label.len() as u16 + 1);
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < px {
                        buf[(lx, yi)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        }

        // Draw colorbar
        if self.show_colorbar {
            let (vmin, vmax) = self.data.value_bounds();
            let cb = Colorbar::new(self.colormap.as_ref(), vmin, vmax);
            let cb_area = Rect::new(
                px + aw + 2,
                py,
                colorbar_width.min(area.x + area.width - px - aw - 2),
                ah,
            );
            if cb_area.x + cb_area.width <= area.x + area.width {
                (&cb).render(cb_area, buf);
            }
        }
    }
}
