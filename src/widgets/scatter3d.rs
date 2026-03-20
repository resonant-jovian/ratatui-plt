//! 3D scatter plot widget with depth cuing.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{StatefulWidget, Widget};

use crate::colormap::{Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::series::Series3D;
use crate::style::MarkerShape;
use crate::transform::{Camera3D, Camera3DState, data_to_screen, depth_sort};

/// A 3D scatter plot widget.
///
/// Renders 3D points with depth cuing (dimmer/smaller for farther points).
///
/// # Example
///
/// ```
/// use ratatui_sim::prelude::*;
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
        }
    }
}

impl Scatter3D {
    pub fn new() -> Self { Self::default() }

    pub fn series(mut self, s: Series3D) -> Self { self.series.push(s); self }
    pub fn camera(mut self, cam: Camera3D) -> Self { self.camera = cam; self }
    pub fn title(mut self, t: impl Into<String>) -> Self { self.title = Some(t.into()); self }
    pub fn marker(mut self, m: MarkerShape) -> Self { self.marker = m; self }

    /// Color points by their z-value (or custom values) using the colormap.
    pub fn color_by_value(mut self, enable: bool) -> Self {
        self.color_by_value = enable;
        self
    }

    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
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
                    buf[(x, area.y)].set_char(ch).set_fg(Color::White);
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
                x_min = x_min.min(x); x_max = x_max.max(x);
                y_min = y_min.min(y); y_max = y_max.max(y);
                z_min = z_min.min(z); z_max = z_max.max(z);
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
                    s.color
                };

                all_points.push((sx, sy, depth, color));
            }
        }

        if all_points.is_empty() {
            return;
        }

        // Screen bounds
        let sx_min = all_points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let sx_max = all_points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let sy_min = all_points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let sy_max = all_points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        let depth_min = all_points.iter().map(|p| p.2).fold(f64::INFINITY, f64::min);
        let depth_max = all_points.iter().map(|p| p.2).fold(f64::NEG_INFINITY, f64::max);
        let depth_range = if depth_max == depth_min { 1.0 } else { depth_max - depth_min };

        // Sort by depth (back to front)
        let depths: Vec<f64> = all_points.iter().map(|p| p.2).collect();
        let sorted = depth_sort(&depths);

        for &idx in &sorted {
            let (sx, sy, depth, color) = all_points[idx];

            let scx = data_to_screen(sx, sx_min, sx_max, px as f64, (px + pw - 1) as f64).round() as u16;
            let scy = data_to_screen(sy, sy_min, sy_max, py as f64, (py + ph - 1) as f64).round() as u16;

            if scx >= px && scx < px + pw && scy >= py && scy < py + ph {
                // Depth cuing: dim far points
                let brightness: f64 = ((depth - depth_min) / depth_range * 0.7 + 0.3).clamp(0.3, 1.0);
                let dimmed = dim_color(color, brightness);
                buf[(scx, scy)].set_char(self.marker.char()).set_fg(dimmed);
            }
        }

        // Draw simple 3D axis indicators
        let origin = camera.project(0.0, 0.0, 0.0);
        let x_tip = camera.project(0.5, 0.0, 0.0);
        let y_tip = camera.project(0.0, 0.5, 0.0);
        let z_tip = camera.project(0.0, 0.0, 0.3);

        let _ox = data_to_screen(origin.0, sx_min, sx_max, px as f64, (px + pw - 1) as f64).round() as u16;
        let _oy = data_to_screen(origin.1, sy_min, sy_max, py as f64, (py + ph - 1) as f64).round() as u16;

        // X axis label
        let xx = data_to_screen(x_tip.0, sx_min, sx_max, px as f64, (px + pw - 1) as f64).round() as u16;
        let xy = data_to_screen(x_tip.1, sy_min, sy_max, py as f64, (py + ph - 1) as f64).round() as u16;
        if xx >= px && xx < px + pw && xy >= py && xy < py + ph {
            buf[(xx, xy)].set_char('X').set_fg(Color::Red);
        }

        // Y axis label
        let yx = data_to_screen(y_tip.0, sx_min, sx_max, px as f64, (px + pw - 1) as f64).round() as u16;
        let yy = data_to_screen(y_tip.1, sy_min, sy_max, py as f64, (py + ph - 1) as f64).round() as u16;
        if yx >= px && yx < px + pw && yy >= py && yy < py + ph {
            buf[(yx, yy)].set_char('Y').set_fg(Color::Green);
        }

        // Z axis label
        let zx = data_to_screen(z_tip.0, sx_min, sx_max, px as f64, (px + pw - 1) as f64).round() as u16;
        let zy = data_to_screen(z_tip.1, sy_min, sy_max, py as f64, (py + ph - 1) as f64).round() as u16;
        if zx >= px && zx < px + pw && zy >= py && zy < py + ph {
            buf[(zx, zy)].set_char('Z').set_fg(Color::Blue);
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
