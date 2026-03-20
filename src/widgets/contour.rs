//! Contour plot widget using marching squares.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::axis::{AspectRatio, Axis};
use crate::colormap::{Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::series::GridData;
use crate::theme::Theme;
use crate::transform::{apply_aspect_ratio, data_to_screen};

/// A contour plot widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 40, 40, |x, y| {
///     (-(x*x + y*y) / 2.0).exp()
/// });
/// let plot = ContourPlot::new(data).levels(10).title("Gaussian");
/// ```
pub struct ContourPlot {
    data: GridData,
    levels: Vec<f64>,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    aspect_ratio: AspectRatio,
    filled: bool,
    show_labels: bool,
    theme: Theme,
}

impl ContourPlot {
    /// Create a contour plot from grid data with auto-computed levels.
    pub fn new(data: GridData) -> Self {
        let (vmin, vmax) = data.value_bounds();
        Self {
            data,
            levels: Vec::new(),
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            aspect_ratio: AspectRatio::Auto,
            filled: false,
            show_labels: false,
            theme: Theme::get_default(),
        }
    }

    /// Set the number of contour levels (auto-spaced).
    pub fn levels(mut self, n: usize) -> Self {
        let (vmin, vmax) = self.data.value_bounds();
        self.levels = (0..n)
            .map(|i| vmin + (vmax - vmin) * (i as f64 + 0.5) / n as f64)
            .collect();
        self
    }

    /// Set explicit contour level values.
    pub fn level_values(mut self, levels: Vec<f64>) -> Self {
        self.levels = levels;
        self
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

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
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

    /// Set the aspect ratio.
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
        self
    }

    /// Enable filled contours (contourf-style).
    pub fn filled(mut self, f: bool) -> Self {
        self.filled = f;
        self
    }

    /// Show level labels on contour lines.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for &ContourPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let y_label_width: u16 = 8;
        let tick_height: u16 = 1;

        let px = area.x + y_label_width;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(y_label_width + 1);
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
                    buf[(x, area.y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        let nrows = self.data.nrows();
        let ncols = self.data.ncols();
        if nrows < 2 || ncols < 2 {
            return;
        }

        let x_lo = self.data.x[0];
        let x_hi = self.data.x[ncols - 1];
        let y_lo = self.data.y[0];
        let y_hi = self.data.y[nrows - 1];

        let (ax_off, ay_off, aw, ah) =
            apply_aspect_ratio(&self.aspect_ratio, x_hi - x_lo, y_hi - y_lo, pw, ph);
        let px = px + ax_off;
        let py = py + ay_off;

        let levels = if self.levels.is_empty() {
            let (vmin, vmax) = self.data.value_bounds();
            (0..8)
                .map(|i| vmin + (vmax - vmin) * (i as f64 + 0.5) / 8.0)
                .collect::<Vec<_>>()
        } else {
            self.levels.clone()
        };

        let (_vmin, _vmax) = self.data.value_bounds();

        // Draw grid
        let x_grid = self.x_axis.grid || self.theme.grid_visible;
        let y_grid = self.y_axis.grid || self.theme.grid_visible;
        if x_grid {
            let gx_ticks = self.x_axis.tick_positions(x_lo, x_hi);
            for &tv in &gx_ticks {
                let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                let xi = sx.round() as u16;
                if xi >= px && xi < px + aw {
                    for y in py..py + ah {
                        buf[(xi, y)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }
        if y_grid {
            let gy_ticks = self.y_axis.tick_positions(y_lo, y_hi);
            for &tv in &gy_ticks {
                let sy = data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ah {
                    for x in px..px + aw {
                        buf[(x, yi)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }

        // Filled contours: color each cell by value band
        if self.filled {
            for cy in 0..ah {
                for cx in 0..aw {
                    let data_x = x_lo + (cx as f64 / aw as f64) * (x_hi - x_lo);
                    let data_y = y_hi - (cy as f64 / ah as f64) * (y_hi - y_lo);

                    let col = ((data_x - x_lo) / (x_hi - x_lo) * (ncols - 1) as f64)
                        .round()
                        .clamp(0.0, (ncols - 1) as f64) as usize;
                    let row = ((data_y - y_lo) / (y_hi - y_lo) * (nrows - 1) as f64)
                        .round()
                        .clamp(0.0, (nrows - 1) as f64) as usize;

                    let val = self.data.values[row][col];
                    let band = levels.partition_point(|&l| l <= val);
                    let t = band as f64 / levels.len() as f64;
                    let color = self.colormap.color_at(t);

                    let sx = px + cx;
                    let sy = py + cy;
                    if sx < area.x + area.width && sy < area.y + area.height {
                        buf[(sx, sy)]
                            .set_char('█')
                            .set_style(Style::default().fg(color));
                    }
                }
            }
        }

        // Draw contour lines
        {
        // Edge naming: top=v00-v10, right=v10-v11, bottom=v01-v11, left=v00-v01
        // Corners: v00=top-left(j,i), v10=top-right(j,i+1), v01=bottom-left(j+1,i), v11=bottom-right(j+1,i+1)
        for &level in &levels {
            let t = self.norm.normalize(level);
            let color = if self.filled {
                // Use contrasting color for lines on filled contours
                self.theme.foreground
            } else {
                self.colormap.color_at(t)
            };

            for j in 0..nrows - 1 {
                for i in 0..ncols - 1 {
                    let v00 = self.data.values[j][i];
                    let v10 = self.data.values[j][i + 1];
                    let v01 = self.data.values[j + 1][i];
                    let v11 = self.data.values[j + 1][i + 1];

                    let case = ((v00 >= level) as u8)
                        | (((v10 >= level) as u8) << 1)
                        | (((v01 >= level) as u8) << 2)
                        | (((v11 >= level) as u8) << 3);

                    if case == 0 || case == 15 {
                        continue;
                    }

                    // Interpolation fraction along an edge
                    let interp = |va: f64, vb: f64| -> f64 {
                        if (vb - va).abs() < 1e-12 {
                            0.5
                        } else {
                            (level - va) / (vb - va)
                        }
                    };

                    // Data coordinates of the four corners
                    let x0 = self.data.x[i];
                    let x1 = self.data.x[i + 1];
                    let y0 = self.data.y[j];
                    let y1 = self.data.y[j + 1];

                    // Edge crossing points in data coordinates
                    let top = || {
                        let f = interp(v00, v10);
                        (x0 + f * (x1 - x0), y0)
                    };
                    let bottom = || {
                        let f = interp(v01, v11);
                        (x0 + f * (x1 - x0), y1)
                    };
                    let left = || {
                        let f = interp(v00, v01);
                        (x0, y0 + f * (y1 - y0))
                    };
                    let right = || {
                        let f = interp(v10, v11);
                        (x1, y0 + f * (y1 - y0))
                    };

                    // Collect line segments for this cell
                    let segments: Vec<((f64, f64), (f64, f64))> = match case {
                        1 | 14 => vec![(top(), left())],
                        2 | 13 => vec![(top(), right())],
                        3 | 12 => vec![(left(), right())],
                        4 | 11 => vec![(bottom(), left())],
                        5 => vec![(top(), left()), (bottom(), right())],
                        6 | 9 => vec![(top(), bottom())],
                        7 | 8 => vec![(bottom(), right())],
                        10 => vec![(top(), right()), (bottom(), left())],
                        _ => vec![],
                    };

                    // Draw each segment using Bresenham
                    for ((dx0, dy0), (dx1, dy1)) in segments {
                        let sx0 = data_to_screen(dx0, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                        let sy0 = data_to_screen(dy0, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
                        let sx1 = data_to_screen(dx1, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                        let sy1 = data_to_screen(dy1, y_lo, y_hi, (py + ah - 1) as f64, py as f64);

                        draw_contour_line(buf, sx0, sy0, sx1, sy1, color, px, py, aw, ah);
                    }
                }
            }
        }
        }

        // Draw axes
        for x in px..px + aw {
            if x < area.x + area.width {
                buf[(x, py + ah)]
                    .set_char('─')
                    .set_fg(self.theme.axis_color);
            }
        }
        for y in py..py + ah {
            buf[(px.saturating_sub(1), y)]
                .set_char('│')
                .set_fg(self.theme.axis_color);
        }

        // Tick labels
        let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let start = xi.saturating_sub(label.len() as u16 / 2);
            let y = py + ah;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }

        let y_ticks = self.y_axis.tick_positions(y_lo, y_hi);
        for &tv in &y_ticks {
            let sy = data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
            let label = self.y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ah {
                let start = px.saturating_sub(label.len() as u16 + 1);
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < px {
                        buf[(lx, yi)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }
    }
}

/// Draw a line between two screen-space points using Bresenham's algorithm.
#[allow(clippy::too_many_arguments)]
fn draw_contour_line(
    buf: &mut Buffer,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
    clip_x: u16,
    clip_y: u16,
    clip_w: u16,
    clip_h: u16,
) {
    let mut ix0 = x0.round() as i32;
    let mut iy0 = y0.round() as i32;
    let ix1 = x1.round() as i32;
    let iy1 = y1.round() as i32;

    let dx = (ix1 - ix0).abs();
    let dy = -(iy1 - iy0).abs();
    let sx = if ix0 < ix1 { 1 } else { -1 };
    let sy = if iy0 < iy1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        let px = ix0 as u16;
        let py = iy0 as u16;
        if px >= clip_x && px < clip_x + clip_w && py >= clip_y && py < clip_y + clip_h {
            buf[(px, py)].set_char('·').set_fg(color);
        }
        if ix0 == ix1 && iy0 == iy1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            ix0 += sx;
        }
        if e2 <= dx {
            err += dx;
            iy0 += sy;
        }
    }
}
