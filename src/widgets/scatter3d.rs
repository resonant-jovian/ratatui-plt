//! 3D scatter plot widget with depth cuing.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{StatefulWidget, Widget};

use crate::colormap::{Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::series::Series3D;
use crate::style::MarkerShape;
use crate::theme::Theme;
use crate::transform::{Camera3D, Camera3DState, data_to_screen, depth_sort};

/// A 3D scatter plot widget.
///
/// Renders 3D points with depth cuing (dimmer/smaller for farther points).
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let data = Series3D::new("particles")
///     .data(vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)]);
/// let plot = Scatter3D::new()
///     .series(data)
///     .camera(Camera3D::new().azimuth(-45.0));
/// ```
pub struct Scatter3D {
    series: Vec<Series3D>,
    camera: Camera3D,
    title: Option<String>,
    marker: MarkerShape,
    color_by_value: bool,
    colormap: Box<dyn Colormap>,
    theme: Theme,
}

impl Default for Scatter3D {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            camera: Camera3D::default(),
            title: None,
            marker: MarkerShape::Dot,
            color_by_value: false,
            colormap: Box::new(Viridis),
            theme: Theme::get_default(),
        }
    }
}

impl Scatter3D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn series(mut self, s: Series3D) -> Self {
        self.series.push(s);
        self
    }
    pub fn camera(mut self, cam: Camera3D) -> Self {
        self.camera = cam;
        self
    }
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }
    pub fn marker(mut self, m: MarkerShape) -> Self {
        self.marker = m;
        self
    }

    /// Color points by their z-value (or custom values) using the colormap.
    pub fn color_by_value(mut self, enable: bool) -> Self {
        self.color_by_value = enable;
        self
    }

    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
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

        // Collect all points with projected coords
        let mut all_points: Vec<(f64, f64, f64, Color)> = Vec::new();

        // Find global bounds for normalization
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut z_min = f64::INFINITY;
        let mut z_max = f64::NEG_INFINITY;

        for s in &self.series {
            for &(x, y, z) in &s.data {
                x_min = x_min.min(x);
                x_max = x_max.max(x);
                y_min = y_min.min(y);
                y_max = y_max.max(y);
                z_min = z_min.min(z);
                z_max = z_max.max(z);
            }
        }

        let x_range = if x_max == x_min { 1.0 } else { x_max - x_min };
        let y_range = if y_max == y_min { 1.0 } else { y_max - y_min };
        let z_range = if z_max == z_min { 1.0 } else { z_max - z_min };

        let z_norm = LinearNorm::new(z_min, z_max);

        for s in &self.series {
            for (idx, &(x, y, z)) in s.data.iter().enumerate() {
                let nx = 2.0 * (x - x_min) / x_range - 1.0;
                let ny = 2.0 * (y - y_min) / y_range - 1.0;
                let nz = (2.0 * (z - z_min) / z_range - 1.0) * 0.5;

                let (sx, sy, depth) = camera.project(nx, ny, nz);

                let color = if self.color_by_value {
                    let val = s.values.as_ref().map_or(z, |v| v[idx]);
                    let t = z_norm.normalize(val);
                    self.colormap.color_at(t)
                } else {
                    s.color.unwrap_or(self.theme.primary)
                };

                all_points.push((sx, sy, depth, color));
            }
        }

        if all_points.is_empty() {
            return;
        }

        // Screen bounds
        let sx_min = all_points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let sx_max = all_points
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let sy_min = all_points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let sy_max = all_points
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max);

        // Apply zoom to viewport
        let zoom = 5.0 / camera.distance;
        let (sx_min, sx_max, sy_min, sy_max) = {
            let cx = (sx_min + sx_max) / 2.0;
            let cy = (sy_min + sy_max) / 2.0;
            let hx = (sx_max - sx_min) / 2.0 / zoom;
            let hy = (sy_max - sy_min) / 2.0 / zoom;
            (cx - hx, cx + hx, cy - hy, cy + hy)
        };

        let depth_min = all_points.iter().map(|p| p.2).fold(f64::INFINITY, f64::min);
        let depth_max = all_points
            .iter()
            .map(|p| p.2)
            .fold(f64::NEG_INFINITY, f64::max);
        let depth_range = if depth_max == depth_min {
            1.0
        } else {
            depth_max - depth_min
        };

        // Sort by depth (back to front)
        let depths: Vec<f64> = all_points.iter().map(|p| p.2).collect();
        let sorted = depth_sort(&depths);

        for &idx in &sorted {
            let (sx, sy, depth, color) = all_points[idx];

            let scx =
                data_to_screen(sx, sx_min, sx_max, px as f64, (px + pw - 1) as f64).round() as u16;
            let scy =
                data_to_screen(sy, sy_min, sy_max, py as f64, (py + ph - 1) as f64).round() as u16;

            if scx >= px && scx < px + pw && scy >= py && scy < py + ph {
                // Depth cuing: dim far points
                let brightness: f64 =
                    ((depth - depth_min) / depth_range * 0.7 + 0.3).clamp(0.3, 1.0);
                let dimmed = dim_color(color, brightness);
                buf[(scx, scy)].set_char(self.marker.char()).set_fg(dimmed);
            }
        }

        // Draw 3D axis lines and labels
        let clip = ClipRect {
            x_min: px,
            y_min: py,
            x_max: px + pw,
            y_max: py + ph,
        };
        let origin = camera.project(0.0, 0.0, 0.0);
        let x_tip = camera.project(0.5, 0.0, 0.0);
        let y_tip = camera.project(0.0, 0.5, 0.0);
        let z_tip = camera.project(0.0, 0.0, 0.3);

        let ox = data_to_screen(origin.0, sx_min, sx_max, px as f64, (px + pw - 1) as f64);
        let oy = data_to_screen(origin.1, sy_min, sy_max, py as f64, (py + ph - 1) as f64);

        // X axis line and label
        let xx = data_to_screen(x_tip.0, sx_min, sx_max, px as f64, (px + pw - 1) as f64);
        let xy = data_to_screen(x_tip.1, sy_min, sy_max, py as f64, (py + ph - 1) as f64);
        draw_braille_line(buf, ox, oy, xx, xy, self.theme.x_axis_3d_color, &clip);
        let xxi = xx.round() as u16;
        let xyi = xy.round() as u16;
        if xxi >= px && xxi < px + pw && xyi >= py && xyi < py + ph {
            buf[(xxi, xyi)]
                .set_char('X')
                .set_fg(self.theme.x_axis_3d_color);
        }

        // Y axis line and label
        let yx = data_to_screen(y_tip.0, sx_min, sx_max, px as f64, (px + pw - 1) as f64);
        let yy = data_to_screen(y_tip.1, sy_min, sy_max, py as f64, (py + ph - 1) as f64);
        draw_braille_line(buf, ox, oy, yx, yy, self.theme.y_axis_3d_color, &clip);
        let yxi = yx.round() as u16;
        let yyi = yy.round() as u16;
        if yxi >= px && yxi < px + pw && yyi >= py && yyi < py + ph {
            buf[(yxi, yyi)]
                .set_char('Y')
                .set_fg(self.theme.y_axis_3d_color);
        }

        // Z axis line and label
        let zx = data_to_screen(z_tip.0, sx_min, sx_max, px as f64, (px + pw - 1) as f64);
        let zy = data_to_screen(z_tip.1, sy_min, sy_max, py as f64, (py + ph - 1) as f64);
        draw_braille_line(buf, ox, oy, zx, zy, self.theme.z_axis_3d_color, &clip);
        let zxi = zx.round() as u16;
        let zyi = zy.round() as u16;
        if zxi >= px && zxi < px + pw && zyi >= py && zyi < py + ph {
            buf[(zxi, zyi)]
                .set_char('Z')
                .set_fg(self.theme.z_axis_3d_color);
        }
    }
}

fn dim_color(color: Color, brightness: f64) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f64 * brightness) as u8,
            (g as f64 * brightness) as u8,
            (b as f64 * brightness) as u8,
        ),
        _ => color,
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


impl Widget for &Scatter3D {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_with_camera(&self.camera, area, buf);
    }
}

impl StatefulWidget for &Scatter3D {
    type State = Camera3DState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let camera = state.to_camera();
        self.render_with_camera(&camera, area, buf);
    }
}
