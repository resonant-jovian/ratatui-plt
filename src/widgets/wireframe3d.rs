//! 3D wireframe mesh widget.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{StatefulWidget, Widget};

use crate::series::GridData;
use crate::theme::Theme;
use crate::transform::{Camera3D, Camera3DState, data_to_screen};

/// A 3D wireframe plot widget.
///
/// Renders a grid mesh in 3D space with depth-cued line brightness.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let data = GridData::from_fn((-2.0, 2.0), (-2.0, 2.0), 20, 20, |x, y| {
///     (-(x*x + y*y)).exp()
/// });
/// let plot = Wireframe3D::new(data).title("Wireframe");
/// ```
pub struct Wireframe3D {
    data: GridData,
    camera: Camera3D,
    color: Color,
    title: Option<String>,
    theme: Theme,
}

impl Wireframe3D {
    pub fn new(data: GridData) -> Self {
        Self {
            data,
            camera: Camera3D::default(),
            color: Color::Cyan,
            title: None,
            theme: Theme::get_default(),
        }
    }

    pub fn camera(mut self, cam: Camera3D) -> Self {
        self.camera = cam;
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }
    pub fn theme(mut self, t: Theme) -> Self {
        self.theme = t;
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
        let x_lo = self.data.x[0];
        let x_hi = self.data.x[ncols - 1];
        let y_lo = self.data.y[0];
        let y_hi = self.data.y[nrows - 1];
        let x_range = if x_hi == x_lo { 1.0 } else { x_hi - x_lo };
        let y_range = if y_hi == y_lo { 1.0 } else { y_hi - y_lo };
        let z_range = if vmax == vmin { 1.0 } else { vmax - vmin };

        // Project all points
        let mut points: Vec<(f64, f64, f64)> = Vec::with_capacity(nrows * ncols);
        for j in 0..nrows {
            for i in 0..ncols {
                let nx = 2.0 * (self.data.x[i] - x_lo) / x_range - 1.0;
                let ny = 2.0 * (self.data.y[j] - y_lo) / y_range - 1.0;
                let nz = (2.0 * (self.data.values[j][i] - vmin) / z_range - 1.0) * 0.5;
                let (sx, sy, depth) = camera.project(nx, ny, nz);
                points.push((sx, sy, depth));
            }
        }

        let sx_min = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let sx_max = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let sy_min = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let sy_max = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        let depth_min = points.iter().map(|p| p.2).fold(f64::INFINITY, f64::min);
        let depth_max = points.iter().map(|p| p.2).fold(f64::NEG_INFINITY, f64::max);
        let depth_range = if depth_max == depth_min {
            1.0
        } else {
            depth_max - depth_min
        };

        // Collect line segments
        struct Segment {
            sx0: u16,
            sy0: u16,
            sx1: u16,
            sy1: u16,
            depth: f64,
        }

        let mut segments: Vec<Segment> = Vec::new();

        let map_x = |sx: f64| -> u16 {
            data_to_screen(sx, sx_min, sx_max, px as f64, (px + pw - 1) as f64).round() as u16
        };
        let map_y = |sy: f64| -> u16 {
            data_to_screen(sy, sy_min, sy_max, py as f64, (py + ph - 1) as f64).round() as u16
        };

        // Row lines
        for j in 0..nrows {
            for i in 0..ncols - 1 {
                let idx0 = j * ncols + i;
                let idx1 = j * ncols + i + 1;
                segments.push(Segment {
                    sx0: map_x(points[idx0].0),
                    sy0: map_y(points[idx0].1),
                    sx1: map_x(points[idx1].0),
                    sy1: map_y(points[idx1].1),
                    depth: (points[idx0].2 + points[idx1].2) / 2.0,
                });
            }
        }
        // Column lines
        for j in 0..nrows - 1 {
            for i in 0..ncols {
                let idx0 = j * ncols + i;
                let idx1 = (j + 1) * ncols + i;
                segments.push(Segment {
                    sx0: map_x(points[idx0].0),
                    sy0: map_y(points[idx0].1),
                    sx1: map_x(points[idx1].0),
                    sy1: map_y(points[idx1].1),
                    depth: (points[idx0].2 + points[idx1].2) / 2.0,
                });
            }
        }

        // Sort by depth (back to front)
        segments.sort_by(|a, b| {
            a.depth
                .partial_cmp(&b.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Draw segments with depth-cued brightness
        for seg in &segments {
            let brightness =
                ((seg.depth - depth_min) / depth_range * 200.0 + 55.0).clamp(55.0, 255.0) as u8;
            let (r, g, b) = match self.color {
                Color::Rgb(r, g, b) => (r, g, b),
                Color::Cyan => (0, 255, 255),
                Color::Green => (0, 255, 0),
                Color::Yellow => (255, 255, 0),
                Color::Red => (255, 0, 0),
                Color::Blue => (0, 0, 255),
                Color::Magenta => (255, 0, 255),
                _ => (255, 255, 255),
            };
            let color = Color::Rgb(
                (r as f64 * brightness as f64 / 255.0) as u8,
                (g as f64 * brightness as f64 / 255.0) as u8,
                (b as f64 * brightness as f64 / 255.0) as u8,
            );

            draw_line_simple(
                buf,
                seg.sx0,
                seg.sy0,
                seg.sx1,
                seg.sy1,
                color,
                [px, py, px + pw, py + ph],
            );
        }
    }
}

fn draw_line_simple(
    buf: &mut Buffer,
    x0: u16,
    y0: u16,
    x1: u16,
    y1: u16,
    color: Color,
    clip: [u16; 4],
) {
    let [clip_x_min, clip_y_min, clip_x_max, clip_y_max] = clip;
    let mut ix = x0 as i32;
    let mut iy = y0 as i32;
    let ix1 = x1 as i32;
    let iy1 = y1 as i32;
    let dx = (ix1 - ix).abs();
    let dy = -(iy1 - iy).abs();
    let sx = if ix < ix1 { 1 } else { -1 };
    let sy = if iy < iy1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        let px = ix as u16;
        let py = iy as u16;
        if px >= clip_x_min && px < clip_x_max && py >= clip_y_min && py < clip_y_max {
            buf[(px, py)].set_char('·').set_fg(color);
        }
        if ix == ix1 && iy == iy1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            ix += sx;
        }
        if e2 <= dx {
            err += dx;
            iy += sy;
        }
    }
}

impl Widget for &Wireframe3D {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_with_camera(&self.camera, area, buf);
    }
}

impl StatefulWidget for &Wireframe3D {
    type State = Camera3DState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let camera = state.to_camera();
        self.render_with_camera(&camera, area, buf);
    }
}
