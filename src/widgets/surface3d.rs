//! 3D surface plot widget with interactive camera support.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{StatefulWidget, Widget};

use crate::colormap::{Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::series::GridData;
use crate::transform::{Camera3D, Camera3DState, data_to_screen};

/// A 3D surface plot widget.
///
/// Renders z = f(x, y) as a colored surface using isometric or perspective projection.
/// Supports both static (`Widget`) and interactive (`StatefulWidget`) usage.
///
/// # Example
///
/// ```
/// use ratatui_sim::prelude::*;
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
                    buf[(x, area.y)].set_char(ch).set_fg(Color::White);
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
        let mut projected: Vec<(f64, f64, f64, f64)> = Vec::new(); // (sx, sy, depth, value)
        for j in 0..nrows {
            for i in 0..ncols {
                let nx = if x_range > 0.0 { 2.0 * (self.data.x[i] - x_lo) / x_range - 1.0 } else { 0.0 };
                let ny = if y_range > 0.0 { 2.0 * (self.data.y[j] - y_lo) / y_range - 1.0 } else { 0.0 };
                let nz = if z_range > 0.0 { 2.0 * (self.data.values[j][i] - vmin) / z_range - 1.0 } else { 0.0 };

                let (sx, sy, depth) = camera.project(nx, ny, nz * 0.5);
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

        // Draw faces
        for &(j, i, _) in &faces {
            let idx00 = j * ncols + i;
            let idx10 = j * ncols + i + 1;
            let idx01 = (j + 1) * ncols + i;
            let idx11 = (j + 1) * ncols + i + 1;

            let avg_val = (projected[idx00].3
                + projected[idx10].3
                + projected[idx01].3
                + projected[idx11].3)
                / 4.0;
            let t = self.norm.normalize(avg_val);
            let color = self.colormap.color_at(t);

            // Draw filled quad by filling the bounding box
            let corners = [
                (projected[idx00].0, projected[idx00].1),
                (projected[idx10].0, projected[idx10].1),
                (projected[idx01].0, projected[idx01].1),
                (projected[idx11].0, projected[idx11].1),
            ];

            let min_x = corners.iter().map(|c| c.0).fold(f64::INFINITY, f64::min);
            let max_x = corners.iter().map(|c| c.0).fold(f64::NEG_INFINITY, f64::max);
            let min_y = corners.iter().map(|c| c.1).fold(f64::INFINITY, f64::min);
            let max_y = corners.iter().map(|c| c.1).fold(f64::NEG_INFINITY, f64::max);

            let sx_s = data_to_screen(min_x, sx_min, sx_max, px as f64, (px + pw - 1) as f64).round() as u16;
            let sx_e = data_to_screen(max_x, sx_min, sx_max, px as f64, (px + pw - 1) as f64).round() as u16;
            let sy_s = data_to_screen(max_y, sy_min, sy_max, py as f64, (py + ph - 1) as f64).round() as u16;
            let sy_e = data_to_screen(min_y, sy_min, sy_max, py as f64, (py + ph - 1) as f64).round() as u16;

            for sy in sy_s..=sy_e {
                for sx in sx_s..=sx_e {
                    if sx >= px && sx < px + pw && sy >= py && sy < py + ph {
                        buf[(sx, sy)]
                            .set_char('█')
                            .set_style(Style::default().fg(color));
                    }
                }
            }

            // Draw wireframe edges
            if self.show_wireframe {
                let wire_color = Color::DarkGray;
                // Just draw corner points as wireframe indication
                for &(cx, cy) in &corners {
                    let scx = data_to_screen(cx, sx_min, sx_max, px as f64, (px + pw - 1) as f64).round() as u16;
                    let scy = data_to_screen(cy, sy_min, sy_max, py as f64, (py + ph - 1) as f64).round() as u16;
                    if scx >= px && scx < px + pw && scy >= py && scy < py + ph {
                        buf[(scx, scy)].set_char('·').set_fg(wire_color);
                    }
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
