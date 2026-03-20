//! 2D histogram widget (matplotlib's hist2d).
//!
//! Bins (x, y) point data into a 2D grid and renders as a heatmap.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::colormap::{Colorbar, Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::transform::data_to_screen;

/// A 2D histogram widget.
///
/// # Example
///
/// ```
/// use ratatui_sim::widgets::hist2d::Hist2D;
///
/// let data: Vec<(f64, f64)> = (0..1000).map(|i| {
///     let x = (i as f64 * 0.01).sin();
///     let y = (i as f64 * 0.01).cos();
///     (x, y)
/// }).collect();
/// let hist = Hist2D::new(data).bins_x(20).bins_y(20);
/// ```
pub struct Hist2D {
    data: Vec<(f64, f64)>,
    bins_x: usize,
    bins_y: usize,
    colormap: Box<dyn Colormap>,
    norm: Option<Box<dyn Normalize>>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_colorbar: bool,
}

impl Hist2D {
    pub fn new(data: Vec<(f64, f64)>) -> Self {
        Self {
            data,
            bins_x: 20,
            bins_y: 20,
            colormap: Box::new(Viridis),
            norm: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_colorbar: true,
        }
    }

    pub fn bins_x(mut self, n: usize) -> Self {
        self.bins_x = n.max(1);
        self
    }

    pub fn bins_y(mut self, n: usize) -> Self {
        self.bins_y = n.max(1);
        self
    }

    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    pub fn norm(mut self, norm: impl Normalize + 'static) -> Self {
        self.norm = Some(Box::new(norm));
        self
    }

    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn show_colorbar(mut self, show: bool) -> Self {
        self.show_colorbar = show;
        self
    }

    fn compute_bins(&self) -> (Vec<Vec<f64>>, f64, f64, f64, f64) {
        // Filter NaN
        let valid: Vec<(f64, f64)> = self.data.iter()
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .copied()
            .collect();

        if valid.is_empty() {
            return (vec![vec![0.0; self.bins_x]; self.bins_y], 0.0, 1.0, 0.0, 1.0);
        }

        let x_min = valid.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x_max = valid.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let y_min = valid.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let y_max = valid.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);

        let x_range = if (x_max - x_min).abs() < 1e-15 { 1.0 } else { x_max - x_min };
        let y_range = if (y_max - y_min).abs() < 1e-15 { 1.0 } else { y_max - y_min };

        let mut grid = vec![vec![0.0f64; self.bins_x]; self.bins_y];

        for &(x, y) in &valid {
            let xi = ((x - x_min) / x_range * self.bins_x as f64).floor() as usize;
            let yi = ((y - y_min) / y_range * self.bins_y as f64).floor() as usize;
            let xi = xi.min(self.bins_x - 1);
            let yi = yi.min(self.bins_y - 1);
            grid[yi][xi] += 1.0;
        }

        (grid, x_min, x_max, y_min, y_max)
    }
}

impl Widget for &Hist2D {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 3 {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let colorbar_width: u16 = if self.show_colorbar { 10 } else { 0 };
        let y_label_width: u16 = 7;
        let tick_height: u16 = 1;

        let px = area.x + y_label_width;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(y_label_width + colorbar_width + 1);
        let ph = area.height.saturating_sub(title_height + tick_height);

        if pw < 2 || ph < 2 {
            return;
        }

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

        let (grid, x_min, x_max, y_min, y_max) = self.compute_bins();
        let nrows = grid.len();
        let ncols = if nrows > 0 { grid[0].len() } else { 0 };

        if nrows == 0 || ncols == 0 {
            return;
        }

        // Determine value range for normalization
        let vmax = grid.iter().flat_map(|r| r.iter()).cloned().fold(0.0f64, f64::max);
        let norm: Box<dyn Normalize> = match &self.norm {
            Some(n) => n.box_clone(),
            None => Box::new(LinearNorm::new(0.0, vmax.max(1.0))),
        };

        // Render using half-block characters
        let effective_height = ph as usize * 2;
        for cy in 0..ph {
            for cx in 0..pw {
                let screen_x = px + cx;
                let screen_y = py + cy;
                if screen_x >= area.x + area.width || screen_y >= area.y + area.height {
                    continue;
                }

                // Top half
                let top_row_f = (cy as usize * 2) as f64 / effective_height as f64;
                let top_row = ((1.0 - top_row_f) * nrows as f64).min((nrows - 1) as f64) as usize;
                let top_col = (cx as f64 / pw as f64 * ncols as f64).min((ncols - 1) as f64) as usize;
                let top_val = grid[top_row][top_col];
                let top_t = norm.normalize(top_val);
                let top_color = self.colormap.color_at(top_t);

                // Bottom half
                let bot_row_f = (cy as usize * 2 + 1) as f64 / effective_height as f64;
                let bot_row = ((1.0 - bot_row_f) * nrows as f64).min((nrows - 1) as f64) as usize;
                let bot_val = grid[bot_row][top_col];
                let bot_t = norm.normalize(bot_val);
                let bot_color = self.colormap.color_at(bot_t);

                buf[(screen_x, screen_y)]
                    .set_char('▀')
                    .set_style(Style::default().fg(top_color).bg(bot_color));
            }
        }

        // Draw tick labels
        let x_ticks = self.x_axis.tick_positions(x_min, x_max);
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_min, x_max, px as f64, (px + pw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let start = xi.saturating_sub(label.len() as u16 / 2);
            let y = py + ph;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        }

        let y_ticks = self.y_axis.tick_positions(y_min, y_max);
        for &tv in &y_ticks {
            let sy = data_to_screen(tv, y_min, y_max, (py + ph - 1) as f64, py as f64);
            let label = self.y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ph {
                let start = px.saturating_sub(label.len() as u16 + 1);
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < px {
                        buf[(lx, yi)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        }

        // Colorbar
        if self.show_colorbar {
            let cb = Colorbar::new(self.colormap.as_ref(), 0.0, vmax);
            let cb_area = Rect::new(
                px + pw + 2,
                py,
                colorbar_width.min(area.x + area.width - px - pw - 2),
                ph,
            );
            if cb_area.x + cb_area.width <= area.x + area.width {
                (&cb).render(cb_area, buf);
            }
        }
    }
}
