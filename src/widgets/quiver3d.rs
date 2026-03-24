//! 3D vector field (quiver) plot widget with interactive camera support.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{StatefulWidget, Widget};

use crate::drawing::draw_braille_line;
use crate::frame::PlotArea;
use crate::theme::Theme;
use crate::transform::{Camera3D, Camera3DState, data_to_screen};

/// A single 3D arrow in a vector field.
///
/// Represents a vector `(dx, dy, dz)` originating at position `(x, y, z)`.
#[derive(Clone, Debug)]
pub struct Arrow3D {
    /// X position of the arrow origin.
    pub x: f64,
    /// Y position of the arrow origin.
    pub y: f64,
    /// Z position of the arrow origin.
    pub z: f64,
    /// X component of the direction vector.
    pub dx: f64,
    /// Y component of the direction vector.
    pub dy: f64,
    /// Z component of the direction vector.
    pub dz: f64,
    /// Color of this arrow (None = use theme primary).
    pub color: Option<Color>,
}

impl Arrow3D {
    /// Create a new arrow at position `(x, y, z)` with direction `(dx, dy, dz)`.
    pub fn new(x: f64, y: f64, z: f64, dx: f64, dy: f64, dz: f64) -> Self {
        Self {
            x,
            y,
            z,
            dx,
            dy,
            dz,
            color: None,
        }
    }

    /// Set the arrow color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A 3D vector field plot widget.
///
/// Renders arrows in 3D space using Camera3D projection. Supports both static
/// (`Widget`) and interactive (`StatefulWidget`) usage with `Camera3DState`.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::widgets::quiver3d::{Quiver3D, Arrow3D};
///
/// let arrows: Vec<Arrow3D> = (0..5).flat_map(|i| {
///     (0..5).map(move |j| {
///         let x = i as f64 - 2.0;
///         let y = j as f64 - 2.0;
///         Arrow3D::new(x, y, 0.0, -y * 0.3, x * 0.3, 0.1)
///     })
/// }).collect();
/// let plot = Quiver3D::new(arrows).title("3D Vector Field");
/// ```
pub struct Quiver3D {
    arrows: Vec<Arrow3D>,
    title: Option<String>,
    scale: f64,
    camera: Camera3D,
    theme: Theme,
}

impl Quiver3D {
    /// Create a new 3D quiver plot from a collection of arrows.
    pub fn new(arrows: Vec<Arrow3D>) -> Self {
        Self {
            arrows,
            title: None,
            scale: 1.0,
            camera: Camera3D::default(),
            theme: Theme::get_default(),
        }
    }

    /// Set the plot title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Set the arrow length scale factor.
    pub fn scale(mut self, s: f64) -> Self {
        self.scale = s;
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
        if area.width < 4 || area.height < 4 || self.arrows.is_empty() {
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

        // Find data bounds for normalization
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut z_min = f64::INFINITY;
        let mut z_max = f64::NEG_INFINITY;

        for a in &self.arrows {
            let ex = a.x + a.dx * self.scale;
            let ey = a.y + a.dy * self.scale;
            let ez = a.z + a.dz * self.scale;
            x_min = x_min.min(a.x).min(ex);
            x_max = x_max.max(a.x).max(ex);
            y_min = y_min.min(a.y).min(ey);
            y_max = y_max.max(a.y).max(ey);
            z_min = z_min.min(a.z).min(ez);
            z_max = z_max.max(a.z).max(ez);
        }

        let x_range = if x_max == x_min { 1.0 } else { x_max - x_min };
        let y_range = if y_max == y_min { 1.0 } else { y_max - y_min };
        let z_range = if z_max == z_min { 1.0 } else { z_max - z_min };

        // Project start and end points for each arrow
        struct ProjectedArrow {
            sx0: f64,
            sy0: f64,
            sx1: f64,
            sy1: f64,
            avg_depth: f64,
            color: Color,
        }

        let normalize_and_project = |x: f64, y: f64, z: f64| -> (f64, f64, f64) {
            let nx = 2.0 * (x - x_min) / x_range - 1.0;
            let ny = 2.0 * (y - y_min) / y_range - 1.0;
            let nz = (2.0 * (z - z_min) / z_range - 1.0) * 0.8;
            camera.project(nx, ny, nz)
        };

        let mut projected: Vec<ProjectedArrow> = Vec::with_capacity(self.arrows.len());
        let mut sx_min = f64::INFINITY;
        let mut sx_max = f64::NEG_INFINITY;
        let mut sy_min = f64::INFINITY;
        let mut sy_max = f64::NEG_INFINITY;

        for a in &self.arrows {
            let (s0x, s0y, d0) = normalize_and_project(a.x, a.y, a.z);
            let ex = a.x + a.dx * self.scale;
            let ey = a.y + a.dy * self.scale;
            let ez = a.z + a.dz * self.scale;
            let (s1x, s1y, d1) = normalize_and_project(ex, ey, ez);

            sx_min = sx_min.min(s0x).min(s1x);
            sx_max = sx_max.max(s0x).max(s1x);
            sy_min = sy_min.min(s0y).min(s1y);
            sy_max = sy_max.max(s0y).max(s1y);

            projected.push(ProjectedArrow {
                sx0: s0x,
                sy0: s0y,
                sx1: s1x,
                sy1: s1y,
                avg_depth: (d0 + d1) / 2.0,
                color: a.color.unwrap_or(self.theme.primary),
            });
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

        // Sort back-to-front by depth
        let mut indices: Vec<usize> = (0..projected.len()).collect();
        indices.sort_by(|&a, &b| {
            projected[a]
                .avg_depth
                .partial_cmp(&projected[b].avg_depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let map_x = |sx: f64| -> f64 {
            data_to_screen(sx, sx_min, sx_max, px as f64, (px + pw - 1) as f64)
        };
        let map_y = |sy: f64| -> f64 {
            data_to_screen(sy, sy_min, sy_max, py as f64, (py + ph - 1) as f64)
        };

        // Build PlotArea for Braille line drawing
        let pa = PlotArea {
            x: px,
            y: py,
            width: pw,
            height: ph,
            x_lo: 0.0,
            x_hi: 0.0,
            y_lo: 0.0,
            y_hi: 0.0,
            area: Rect::new(px, py, pw, ph),
        };

        // Draw arrows back-to-front
        for &idx in &indices {
            let arrow = &projected[idx];
            let scx0 = map_x(arrow.sx0);
            let scy0 = map_y(arrow.sy0);
            let scx1 = map_x(arrow.sx1);
            let scy1 = map_y(arrow.sy1);

            // Draw line using Braille characters for higher resolution
            draw_braille_line(buf, scx0, scy0, scx1, scy1, arrow.color, &pa);

            // Draw arrowhead at end
            let xi = scx1.round() as u16;
            let yi = scy1.round() as u16;
            if xi >= px && xi < px + pw && yi >= py && yi < py + ph {
                let screen_dx = scx1 - scx0;
                let screen_dy = scy1 - scy0;
                let ch = arrow_head_char(screen_dx, screen_dy);
                buf[(xi, yi)].set_char(ch).set_fg(arrow.color);
            }
        }
    }
}

/// Choose an arrowhead character based on screen-space direction.
fn arrow_head_char(dx: f64, dy: f64) -> char {
    if dx.abs() < 0.001 && dy.abs() < 0.001 {
        return '·';
    }
    let angle = dy.atan2(dx);
    let octant = ((angle + std::f64::consts::PI) / (std::f64::consts::PI / 4.0)).round() as i32 % 8;
    match octant {
        0 => '←',
        1 => '↙',
        2 => '↓',
        3 => '↘',
        4 => '→',
        5 => '↗',
        6 => '↑',
        7 => '↖',
        _ => '→',
    }
}

impl Widget for &Quiver3D {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_with_camera(&self.camera, area, buf);
    }
}

impl StatefulWidget for &Quiver3D {
    type State = Camera3DState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let camera = state.to_camera();
        self.render_with_camera(&camera, area, buf);
    }
}
