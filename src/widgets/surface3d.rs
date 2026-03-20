//! 3D surface plot widget with interactive camera support.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{StatefulWidget, Widget};

use crate::colormap::{Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::series::GridData;
use crate::theme::Theme;
use crate::transform::{Camera3D, Camera3DState, data_to_screen};

/// A 3D surface plot widget.
///
/// Renders z = f(x, y) as a colored surface using isometric or perspective projection.
/// Supports both static (`Widget`) and interactive (`StatefulWidget`) usage.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 30, 30, |x, y| {
///     (x * x + y * y).sqrt().sin()
/// });
/// let plot = Surface3D::new(data)
///     .camera(Camera3D::new().azimuth(-60.0).elevation(30.0))
///     .title("3D Surface");
/// ```
pub struct Surface3D {
    data: GridData,
    camera: Camera3D,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    title: Option<String>,
    show_wireframe: bool,
    theme: Theme,
}

impl Surface3D {
    pub fn new(data: GridData) -> Self {
        let (vmin, vmax) = data.value_bounds();
        Self {
            data,
            camera: Camera3D::default(),
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            title: None,
            show_wireframe: true,
            theme: Theme::get_default(),
        }
    }

    pub fn camera(mut self, cam: Camera3D) -> Self {
        self.camera = cam;
        self
    }

    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    pub fn norm(mut self, norm: impl Normalize + 'static) -> Self {
        self.norm = Box::new(norm);
        self
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn show_wireframe(mut self, show: bool) -> Self {
        self.show_wireframe = show;
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

        let nrows = self.data.nrows();
        let ncols = self.data.ncols();
        if nrows < 2 || ncols < 2 {
            return;
        }

        let (vmin, vmax) = self.data.value_bounds();

        // Normalize coordinates to [-1, 1]
        let x_lo = self.data.x[0];
        let x_hi = self.data.x[ncols - 1];
        let y_lo = self.data.y[0];
        let y_hi = self.data.y[nrows - 1];
        let x_range = x_hi - x_lo;
        let y_range = y_hi - y_lo;
        let z_range = if vmax == vmin { 1.0 } else { vmax - vmin };

        // Project all grid points
        let mut projected: Vec<(f64, f64, f64, f64, f64, f64, f64)> = Vec::new(); // (sx, sy, depth, value, nx, ny, nz)
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
                projected.push((sx, sy, depth, self.data.values[j][i], nx, ny, nz));
            }
        }

        // Find screen bounds of projected points
        let mut sx_min = f64::INFINITY;
        let mut sx_max = f64::NEG_INFINITY;
        let mut sy_min = f64::INFINITY;
        let mut sy_max = f64::NEG_INFINITY;
        for &(sx, sy, _, _, _, _, _) in &projected {
            sx_min = sx_min.min(sx);
            sx_max = sx_max.max(sx);
            sy_min = sy_min.min(sy);
            sy_max = sy_max.max(sy);
        }

        // Collect quad faces with average depth for sorting
        let mut faces: Vec<(usize, usize, f64)> = Vec::new(); // (row, col, avg_depth)
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

        // Draw faces with scanline rasterization
        for &(j, i, _) in &faces {
            let idx00 = j * ncols + i;
            let idx10 = j * ncols + i + 1;
            let idx01 = (j + 1) * ncols + i;
            let idx11 = (j + 1) * ncols + i + 1;

            let avg_val =
                (projected[idx00].3 + projected[idx10].3 + projected[idx01].3 + projected[idx11].3)
                    / 4.0;
            let t = self.norm.normalize(avg_val);

            // Lambertian shading: compute face normal from normalized 3D coords
            let (_, _, _, _, nx00, ny00, nz00) = projected[idx00];
            let (_, _, _, _, nx10, ny10, nz10) = projected[idx10];
            let (_, _, _, _, nx01, ny01, nz01) = projected[idx01];

            // Two edge vectors
            let e1 = (nx10 - nx00, ny10 - ny00, nz10 - nz00);
            let e2 = (nx01 - nx00, ny01 - ny00, nz01 - nz00);

            // Cross product for face normal
            let normal = (
                e1.1 * e2.2 - e1.2 * e2.1,
                e1.2 * e2.0 - e1.0 * e2.2,
                e1.0 * e2.1 - e1.1 * e2.0,
            );
            let len = (normal.0 * normal.0 + normal.1 * normal.1 + normal.2 * normal.2).sqrt();
            let normal = if len > 1e-10 {
                (normal.0 / len, normal.1 / len, normal.2 / len)
            } else {
                (0.0, 0.0, 1.0)
            };

            // Light direction (normalized (0.3, -0.5, 0.8))
            let light = (0.302, -0.503, 0.809);
            let dot = (normal.0 * light.0 + normal.1 * light.1 + normal.2 * light.2).abs();
            let shade = dot.clamp(0.4, 1.0);

            let color = crate::colormap::scale_color(self.colormap.color_at(t), shade);

            // Quad corners in screen space: order as a proper quad (not Z-order)
            // v00--v10
            //  |    |
            // v01--v11
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
            let expanded_quad: Vec<(i32, i32)> = screen_quad.iter().map(|&(x, y)| {
                let dx = if x > centroid_x { 1 } else if x < centroid_x { -1 } else { 0 };
                let dy = if y > centroid_y { 1 } else if y < centroid_y { -1 } else { 0 };
                (x + dx, y + dy)
            }).collect();

            // Bounding box of the expanded quad
            let bb_min_x = expanded_quad.iter().map(|c| c.0).min().unwrap();
            let bb_max_x = expanded_quad.iter().map(|c| c.0).max().unwrap();
            let bb_min_y = expanded_quad.iter().map(|c| c.1).min().unwrap();
            let bb_max_y = expanded_quad.iter().map(|c| c.1).max().unwrap();

            // Fill using point-in-quad test (winding number) on expanded quad
            for sy in bb_min_y..=bb_max_y {
                for sx in bb_min_x..=bb_max_x {
                    let ux = sx as u16;
                    let uy = sy as u16;
                    if ux >= px
                        && ux < px + pw
                        && uy >= py
                        && uy < py + ph
                        && point_in_quad(sx, sy, &expanded_quad)
                    {
                        buf[(ux, uy)]
                            .set_char('█')
                            .set_style(Style::default().fg(color));
                    }
                }
            }

            // Draw wireframe edges between adjacent corners
            if self.show_wireframe {
                let wire_color = self.theme.axis_color;
                let edges = [(0, 1), (1, 2), (2, 3), (3, 0)];
                for &(a, b) in &edges {
                    draw_surface_line(
                        buf,
                        screen_quad[a],
                        screen_quad[b],
                        wire_color,
                        px,
                        py,
                        pw,
                        ph,
                    );
                }
            }
        }
    }
}

impl Widget for &Surface3D {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_with_camera(&self.camera, area, buf);
    }
}

impl StatefulWidget for &Surface3D {
    type State = Camera3DState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let camera = state.to_camera();
        self.render_with_camera(&camera, area, buf);
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

/// Draw a line between two pixel positions using Bresenham's algorithm.
#[allow(clippy::too_many_arguments)]
fn draw_surface_line(
    buf: &mut Buffer,
    p0: (i32, i32),
    p1: (i32, i32),
    color: Color,
    clip_x: u16,
    clip_y: u16,
    clip_w: u16,
    clip_h: u16,
) {
    let (mut ix0, mut iy0) = p0;
    let (ix1, iy1) = p1;
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
