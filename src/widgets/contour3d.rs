//! 3D contour plot widget projecting contour lines on a surface.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{StatefulWidget, Widget};

use crate::colormap::{Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::series::GridData;
use crate::theme::Theme;
use crate::transform::{Camera3D, Camera3DState, data_to_screen};

/// A 3D contour plot widget.
///
/// Projects contour lines onto a 3D surface using marching squares for
/// contour extraction and Camera3D for projection. Supports both static
/// (`Widget`) and interactive (`StatefulWidget`) usage.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::widgets::contour3d::Contour3D;
///
/// let data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 30, 30, |x, y| {
///     (-(x * x + y * y) / 4.0).exp()
/// });
/// let plot = Contour3D::new(data).levels(8).title("3D Contours");
/// ```
pub struct Contour3D {
    data: GridData,
    levels: Vec<f64>,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    title: Option<String>,
    camera: Camera3D,
    theme: Theme,
}

impl Contour3D {
    /// Create a 3D contour plot from grid data.
    pub fn new(data: GridData) -> Self {
        let (vmin, vmax) = data.value_bounds();
        Self {
            data,
            levels: Vec::new(),
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            title: None,
            camera: Camera3D::default(),
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
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Set the camera configuration.
    pub fn camera(mut self, cam: Camera3D) -> Self {
        self.camera = cam;
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    fn render_with_camera(&self, camera: &Camera3D, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        let nrows = self.data.nrows();
        let ncols = self.data.ncols();
        if nrows < 2 || ncols < 2 {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let px = area.x + 1;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(2);
        let ph = area.height.saturating_sub(title_height + 1);

        if pw < 2 || ph < 2 {
            return;
        }

        // Title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        let (vmin, vmax) = self.data.value_bounds();
        let x_lo = self.data.x[0];
        let x_hi = self.data.x[ncols - 1];
        let y_lo = self.data.y[0];
        let y_hi = self.data.y[nrows - 1];
        let x_range = if x_hi == x_lo { 1.0 } else { x_hi - x_lo };
        let y_range = if y_hi == y_lo { 1.0 } else { y_hi - y_lo };
        let z_range = if vmax == vmin { 1.0 } else { vmax - vmin };

        let levels = if self.levels.is_empty() {
            (0..8)
                .map(|i| vmin + (vmax - vmin) * (i as f64 + 0.5) / 8.0)
                .collect::<Vec<_>>()
        } else {
            self.levels.clone()
        };

        // Closure to normalize and project a 3D data point
        let normalize_and_project = |x: f64, y: f64, z: f64| -> (f64, f64, f64) {
            let nx = if x_range > 0.0 {
                2.0 * (x - x_lo) / x_range - 1.0
            } else {
                0.0
            };
            let ny = if y_range > 0.0 {
                2.0 * (y - y_lo) / y_range - 1.0
            } else {
                0.0
            };
            let nz = if z_range > 0.0 {
                2.0 * (z - vmin) / z_range - 1.0
            } else {
                0.0
            };
            camera.project(nx, ny, nz * 0.8)
        };

        // Find screen bounds by projecting grid corners and extremes
        let mut sx_min = f64::INFINITY;
        let mut sx_max = f64::NEG_INFINITY;
        let mut sy_min = f64::INFINITY;
        let mut sy_max = f64::NEG_INFINITY;

        for j in 0..nrows {
            for i in 0..ncols {
                let (sx, sy, _) =
                    normalize_and_project(self.data.x[i], self.data.y[j], self.data.values[j][i]);
                sx_min = sx_min.min(sx);
                sx_max = sx_max.max(sx);
                sy_min = sy_min.min(sy);
                sy_max = sy_max.max(sy);
            }
        }

        // Apply zoom
        let zoom = 5.0 / camera.distance;
        let (sx_min, sx_max, sy_min, sy_max) = {
            let cx = (sx_min + sx_max) / 2.0;
            let cy = (sy_min + sy_max) / 2.0;
            let hx = (sx_max - sx_min) / 2.0 / zoom;
            let hy = (sy_max - sy_min) / 2.0 / zoom;
            (cx - hx, cx + hx, cy - hy, cy + hy)
        };

        let map_x = |sx: f64| -> f64 {
            data_to_screen(sx, sx_min, sx_max, px as f64, (px + pw - 1) as f64)
        };
        let map_y = |sy: f64| -> f64 {
            data_to_screen(sy, sy_min, sy_max, py as f64, (py + ph - 1) as f64)
        };

        // For each contour level, extract contour segments using marching squares,
        // then project onto the 3D surface
        for &level in &levels {
            let t = self.norm.normalize(level);
            let color = self.colormap.color_at(t);

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

                    let interp = |va: f64, vb: f64| -> f64 {
                        if (vb - va).abs() < 1e-12 {
                            0.5
                        } else {
                            (level - va) / (vb - va)
                        }
                    };

                    let x0 = self.data.x[i];
                    let x1 = self.data.x[i + 1];
                    let y0 = self.data.y[j];
                    let y1 = self.data.y[j + 1];

                    // Edge crossing points in data coordinates (x, y)
                    // The z value at each crossing is the contour level itself
                    let top_edge = || {
                        let f = interp(v00, v10);
                        (x0 + f * (x1 - x0), y0)
                    };
                    let bottom_edge = || {
                        let f = interp(v01, v11);
                        (x0 + f * (x1 - x0), y1)
                    };
                    let left_edge = || {
                        let f = interp(v00, v01);
                        (x0, y0 + f * (y1 - y0))
                    };
                    let right_edge = || {
                        let f = interp(v10, v11);
                        (x1, y0 + f * (y1 - y0))
                    };

                    let segments: Vec<((f64, f64), (f64, f64))> = match case {
                        1 | 14 => vec![(top_edge(), left_edge())],
                        2 | 13 => vec![(top_edge(), right_edge())],
                        3 | 12 => vec![(left_edge(), right_edge())],
                        4 | 11 => vec![(bottom_edge(), left_edge())],
                        5 => vec![(top_edge(), left_edge()), (bottom_edge(), right_edge())],
                        6 | 9 => vec![(top_edge(), bottom_edge())],
                        7 | 8 => vec![(bottom_edge(), right_edge())],
                        10 => vec![(top_edge(), right_edge()), (bottom_edge(), left_edge())],
                        _ => vec![],
                    };

                    // Project each segment endpoint to 3D (z = contour level) then to screen
                    for ((dx0, dy0), (dx1, dy1)) in segments {
                        let (p0x, p0y, _) = normalize_and_project(dx0, dy0, level);
                        let (p1x, p1y, _) = normalize_and_project(dx1, dy1, level);

                        let scx0 = map_x(p0x);
                        let scy0 = map_y(p0y);
                        let scx1 = map_x(p1x);
                        let scy1 = map_y(p1y);

                        draw_line(buf, scx0, scy0, scx1, scy1, color, px, py, pw, ph);
                    }
                }
            }
        }
    }
}

/// Draw a line between two screen-space points using Bresenham's algorithm.
#[allow(clippy::too_many_arguments)]
fn draw_line(
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
        let cx = ix0 as u16;
        let cy = iy0 as u16;
        if cx >= clip_x && cx < clip_x + clip_w && cy >= clip_y && cy < clip_y + clip_h {
            buf[(cx, cy)].set_char('·').set_fg(color);
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

impl Widget for &Contour3D {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_with_camera(&self.camera, area, buf);
    }
}

impl StatefulWidget for &Contour3D {
    type State = Camera3DState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let camera = state.to_camera();
        self.render_with_camera(&camera, area, buf);
    }
}
