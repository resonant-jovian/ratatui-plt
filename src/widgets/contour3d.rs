//! 3D contour plot widget projecting contour lines on a surface.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
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

        // Project all grid points
        let mut projected: Vec<(f64, f64, f64, f64)> = Vec::new(); // (sx, sy, depth, value)
        for j in 0..nrows {
            for i in 0..ncols {
                let nx = if x_range > 0.0 {
                    2.0 * (self.data.x[i] - x_lo) / x_range - 1.0
                } else {
                    0.0
                };
                let ny = if y_range > 0.0 {
                    2.0 * (self.data.y[j] - y_lo) / y_range - 1.0
                } else {
                    0.0
                };
                let nz = if z_range > 0.0 {
                    2.0 * (self.data.values[j][i] - vmin) / z_range - 1.0
                } else {
                    0.0
                };

                let (sx, sy, depth) = camera.project(nx, ny, nz * 0.8);
                projected.push((sx, sy, depth, self.data.values[j][i]));
            }
        }

        // Find screen bounds of projected points
        let mut sx_min = f64::INFINITY;
        let mut sx_max = f64::NEG_INFINITY;
        let mut sy_min = f64::INFINITY;
        let mut sy_max = f64::NEG_INFINITY;
        for &(sx, sy, _, _) in &projected {
            sx_min = sx_min.min(sx);
            sx_max = sx_max.max(sx);
            sy_min = sy_min.min(sy);
            sy_max = sy_max.max(sy);
        }

        // Apply zoom to viewport
        let zoom = 5.0 / camera.distance;
        {
            let cx = (sx_min + sx_max) / 2.0;
            let cy = (sy_min + sy_max) / 2.0;
            let hx = (sx_max - sx_min) / 2.0 / zoom;
            let hy = (sy_max - sy_min) / 2.0 / zoom;
            sx_min = cx - hx;
            sx_max = cx + hx;
            sy_min = cy - hy;
            sy_max = cy + hy;
        }

        // Collect quad faces with average depth for sorting
        let mut faces: Vec<(usize, usize, f64)> = Vec::new();
        for j in 0..nrows - 1 {
            for i in 0..ncols - 1 {
                let idx00 = j * ncols + i;
                let idx10 = j * ncols + i + 1;
                let idx01 = (j + 1) * ncols + i;
                let idx11 = (j + 1) * ncols + i + 1;
                let avg_depth = (projected[idx00].2
                    + projected[idx10].2
                    + projected[idx01].2
                    + projected[idx11].2)
                    / 4.0;
                faces.push((j, i, avg_depth));
            }
        }

        // Sort back-to-front (painter's algorithm)
        faces.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        // Determine which contour level each cell value falls into.
        // For a cell with average value v, find the highest level <= v.
        let cell_contour_color = |avg_val: f64| -> Color {
            // Find the contour band: the index of the highest level that avg_val >= level
            let mut band_idx: Option<usize> = None;
            for (li, &lv) in levels.iter().enumerate() {
                if avg_val >= lv {
                    band_idx = Some(li);
                }
            }
            // Map band index to colormap
            let t = match band_idx {
                Some(idx) => self.norm.normalize(levels[idx]),
                None => self.norm.normalize(vmin),
            };
            self.colormap.color_at(t)
        };

        // Draw faces with scanline rasterization using half-block shading
        for &(j, i, _) in &faces {
            let idx00 = j * ncols + i;
            let idx10 = j * ncols + i + 1;
            let idx01 = (j + 1) * ncols + i;
            let idx11 = (j + 1) * ncols + i + 1;

            // Compute contour-level color from average cell value
            let avg_val =
                (projected[idx00].3 + projected[idx10].3 + projected[idx01].3 + projected[idx11].3)
                    / 4.0;
            let color = cell_contour_color(avg_val);

            // Quad corners in screen space
            let quad = [
                (projected[idx00].0, projected[idx00].1),
                (projected[idx10].0, projected[idx10].1),
                (projected[idx11].0, projected[idx11].1),
                (projected[idx01].0, projected[idx01].1),
            ];

            // Map to pixel coords
            let screen_quad: Vec<(i32, i32)> = quad
                .iter()
                .map(|&(qx, qy)| {
                    let scx = data_to_screen(qx, sx_min, sx_max, px as f64, (px + pw - 1) as f64)
                        .round() as i32;
                    let scy = data_to_screen(qy, sy_min, sy_max, py as f64, (py + ph - 1) as f64)
                        .round() as i32;
                    (scx, scy)
                })
                .collect();

            // Expand quad slightly to eliminate gaps between adjacent faces
            let centroid_x = screen_quad.iter().map(|c| c.0).sum::<i32>() / 4;
            let centroid_y = screen_quad.iter().map(|c| c.1).sum::<i32>() / 4;
            let expanded_quad: Vec<(i32, i32)> = screen_quad
                .iter()
                .map(|&(x, y)| {
                    let dx = if x > centroid_x {
                        1
                    } else if x < centroid_x {
                        -1
                    } else {
                        0
                    };
                    let dy = if y > centroid_y {
                        1
                    } else if y < centroid_y {
                        -1
                    } else {
                        0
                    };
                    (x + dx, y + dy)
                })
                .collect();

            // Bounding box of the expanded quad
            let Some(bb_min_x) = expanded_quad.iter().map(|c| c.0).min() else {
                continue;
            };
            let Some(bb_max_x) = expanded_quad.iter().map(|c| c.0).max() else {
                continue;
            };
            let Some(bb_min_y) = expanded_quad.iter().map(|c| c.1).min() else {
                continue;
            };
            let Some(bb_max_y) = expanded_quad.iter().map(|c| c.1).max() else {
                continue;
            };

            // Fill using half-block characters for doubled vertical resolution
            let mut row = bb_min_y;
            while row <= bb_max_y {
                let upper_row = row;
                let lower_row = row + 1;
                for sx in bb_min_x..=bb_max_x {
                    let ux = sx as u16;
                    let uy_upper = upper_row as u16;
                    let upper_in = ux >= px
                        && ux < px + pw
                        && uy_upper >= py
                        && uy_upper < py + ph
                        && point_in_quad(sx, upper_row, &expanded_quad);
                    let lower_in = lower_row <= bb_max_y
                        && ux >= px
                        && ux < px + pw
                        && (lower_row as u16) >= py
                        && (lower_row as u16) < py + ph
                        && point_in_quad(sx, lower_row, &expanded_quad);

                    if upper_in && lower_in {
                        let upper_frac = if bb_max_y != bb_min_y {
                            (upper_row - bb_min_y) as f64 / (bb_max_y - bb_min_y) as f64
                        } else {
                            0.5
                        };
                        let lower_frac = if bb_max_y != bb_min_y {
                            (lower_row - bb_min_y) as f64 / (bb_max_y - bb_min_y) as f64
                        } else {
                            0.5
                        };
                        let upper_color = shade_color(color, 1.0 - upper_frac * 0.2);
                        let lower_color = shade_color(color, 1.0 - lower_frac * 0.2);
                        buf[(ux, uy_upper)]
                            .set_char(self.theme.chars.fill.half_upper)
                            .set_style(Style::default().fg(upper_color).bg(lower_color));
                    } else if upper_in {
                        let upper_frac = if bb_max_y != bb_min_y {
                            (upper_row - bb_min_y) as f64 / (bb_max_y - bb_min_y) as f64
                        } else {
                            0.5
                        };
                        let upper_color = shade_color(color, 1.0 - upper_frac * 0.2);
                        buf[(ux, uy_upper)]
                            .set_char(self.theme.chars.fill.half_upper)
                            .set_style(Style::default().fg(upper_color));
                    } else if lower_in {
                        let lower_frac = if bb_max_y != bb_min_y {
                            (lower_row - bb_min_y) as f64 / (bb_max_y - bb_min_y) as f64
                        } else {
                            0.5
                        };
                        let lower_color = shade_color(color, 1.0 - lower_frac * 0.2);
                        buf[(ux, uy_upper)]
                            .set_char(self.theme.chars.fill.half_lower)
                            .set_style(Style::default().fg(lower_color));
                    }
                }
                row += 2;
            }
        }
    }
}

/// Test if a point is inside a convex quad using cross-product winding.
fn point_in_quad(px: i32, py: i32, quad: &[(i32, i32)]) -> bool {
    let n = quad.len();
    let mut sign = 0i32;
    for i in 0..n {
        let (x0, y0) = quad[i];
        let (x1, y1) = quad[(i + 1) % n];
        let cross = (x1 - x0) as i64 * (py - y0) as i64 - (y1 - y0) as i64 * (px - x0) as i64;
        if cross != 0 {
            let s = if cross > 0 { 1 } else { -1 };
            if sign == 0 {
                sign = s;
            } else if sign != s {
                return false;
            }
        }
    }
    true
}

/// Apply a shading factor to a color for half-block rendering.
fn shade_color(color: Color, factor: f64) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f64 * factor).round().clamp(0.0, 255.0) as u8,
            (g as f64 * factor).round().clamp(0.0, 255.0) as u8,
            (b as f64 * factor).round().clamp(0.0, 255.0) as u8,
        ),
        other => other,
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
