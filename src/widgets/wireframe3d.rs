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

        // Apply zoom to viewport
        let zoom = 5.0 / camera.distance;
        let (sx_min, sx_max, sy_min, sy_max) = {
            let cx = (sx_min + sx_max) / 2.0;
            let cy = (sy_min + sy_max) / 2.0;
            let hx = (sx_max - sx_min) / 2.0 / zoom;
            let hy = (sy_max - sy_min) / 2.0 / zoom;
            (cx - hx, cx + hx, cy - hy, cy + hy)
        };

        let depth_min = points.iter().map(|p| p.2).fold(f64::INFINITY, f64::min);
        let depth_max = points.iter().map(|p| p.2).fold(f64::NEG_INFINITY, f64::max);
        let depth_range = if depth_max == depth_min {
            1.0
        } else {
            depth_max - depth_min
        };

        // Collect line segments
        struct Segment {
            sx0: f64,
            sy0: f64,
            sx1: f64,
            sy1: f64,
            depth: f64,
        }

        let mut segments: Vec<Segment> = Vec::new();

        let map_x = |sx: f64| -> f64 {
            data_to_screen(sx, sx_min, sx_max, px as f64, (px + pw - 1) as f64)
        };
        let map_y = |sy: f64| -> f64 {
            data_to_screen(sy, sy_min, sy_max, py as f64, (py + ph - 1) as f64)
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
        let clip = ClipRect {
            x_min: px,
            y_min: py,
            x_max: px + pw,
            y_max: py + ph,
        };
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

            draw_braille_line(buf, seg.sx0, seg.sy0, seg.sx1, seg.sy1, color, &clip);
        }
    }
}

struct ClipRect {
    x_min: u16,
    y_min: u16,
    x_max: u16,
    y_max: u16,
}

const BRAILLE_BITS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
const BRAILLE_BASE: u32 = 0x2800;

fn write_braille(buf: &mut Buffer, x: u16, y: u16, bits: u8, color: Color) {
    let (existing_bits, existing_bg) = {
        let cell = &buf[(x, y)];
        let ch = cell.symbol().chars().next().unwrap_or(' ');
        let bg = cell.bg;
        let code = ch as u32;
        if code == 0x2580 || code == 0x2584 {
            return;
        }
        let bits = if (BRAILLE_BASE..=0x28FF).contains(&code) {
            (code - BRAILLE_BASE) as u8
        } else {
            0
        };
        (bits, bg)
    };
    let combined = existing_bits | bits;
    if let Some(ch) = char::from_u32(BRAILLE_BASE + combined as u32) {
        let fg = if crate::drawing::colors_match(color, existing_bg) {
            crate::drawing::contrasting_color(color)
        } else {
            color
        };
        buf[(x, y)].set_char(ch).set_fg(fg).set_bg(existing_bg);
    }
}

fn draw_braille_line(
    buf: &mut Buffer,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
    clip: &ClipRect,
) {
    let mut ix0 = (x0 * 2.0).round() as i32;
    let mut iy0 = (y0 * 4.0).round() as i32;
    let ix1 = (x1 * 2.0).round() as i32;
    let iy1 = (y1 * 4.0).round() as i32;

    let dx = (ix1 - ix0).abs();
    let dy = -(iy1 - iy0).abs();
    let sx = if ix0 < ix1 { 1 } else { -1 };
    let sy = if iy0 < iy1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if ix0 >= 0 && iy0 >= 0 {
            let cell_x = (ix0 / 2) as u16;
            let cell_y = (iy0 / 4) as u16;
            if cell_x >= clip.x_min
                && cell_x < clip.x_max
                && cell_y >= clip.y_min
                && cell_y < clip.y_max
            {
                let dot_col = (ix0 % 2) as usize;
                let dot_row = (iy0 % 4) as usize;
                let bit = BRAILLE_BITS[dot_col][dot_row];
                write_braille(buf, cell_x, cell_y, bit, color);
            }
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
