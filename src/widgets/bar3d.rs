//! 3D bar chart widget with interactive camera support.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{StatefulWidget, Widget};

use crate::drawing::draw_braille_line;
use crate::frame::PlotArea;
use crate::theme::Theme;
use crate::transform::{Camera3D, Camera3DState, data_to_screen};

/// A single bar in a 3D bar chart.
#[derive(Clone, Debug)]
pub struct Bar3DData {
    /// X position of the bar center.
    pub x: f64,
    /// Y position of the bar center.
    pub y: f64,
    /// Height of the bar (z extent from 0).
    pub height: f64,
    /// Bar color (top face). `None` uses `theme.primary`.
    pub color: Option<Color>,
    /// Bar width in data units (default 0.8).
    pub width: f64,
}

impl Bar3DData {
    /// Create a new bar with the given position and height.
    pub fn new(x: f64, y: f64, height: f64) -> Self {
        Self {
            x,
            y,
            height,
            color: None,
            width: 0.8,
        }
    }

    /// Set the bar color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the bar width in data units.
    pub fn width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }
}

/// A 3D bar chart widget.
///
/// Renders vertical bars in 3D space using isometric or perspective projection.
/// Supports both static (`Widget`) and interactive (`StatefulWidget`) usage.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::widgets::bar3d::{Bar3D, Bar3DData};
///
/// let bars = vec![
///     Bar3DData::new(0.0, 0.0, 3.0).color(Color::Cyan),
///     Bar3DData::new(1.0, 0.0, 5.0).color(Color::Yellow),
///     Bar3DData::new(2.0, 0.0, 2.0).color(Color::Green),
/// ];
/// let plot = Bar3D::new(bars)
///     .camera(Camera3D::new().azimuth(-60.0).elevation(30.0))
///     .title("3D Bar Chart");
/// ```
pub struct Bar3D {
    bars: Vec<Bar3DData>,
    camera: Camera3D,
    title: Option<String>,
    theme: Theme,
}

impl Bar3D {
    /// Create a new 3D bar chart with the given bars.
    pub fn new(bars: Vec<Bar3DData>) -> Self {
        Self {
            bars,
            camera: Camera3D::default(),
            title: None,
            theme: Theme::get_default(),
        }
    }

    /// Set the camera configuration.
    pub fn camera(mut self, cam: Camera3D) -> Self {
        self.camera = cam;
        self
    }

    /// Set the plot title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    fn render_with_camera(&self, camera: &Camera3D, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.bars.is_empty() {
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

        // Compute data bounds for normalization
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut z_max = f64::NEG_INFINITY;
        for bar in &self.bars {
            let hw = bar.width / 2.0;
            x_min = x_min.min(bar.x - hw);
            x_max = x_max.max(bar.x + hw);
            y_min = y_min.min(bar.y - hw);
            y_max = y_max.max(bar.y + hw);
            z_max = z_max.max(bar.height.abs());
        }
        // Add some padding
        let x_range = (x_max - x_min).max(1.0);
        let y_range = (y_max - y_min).max(1.0);
        let z_range = z_max.max(1.0);

        // Build face list: each bar has up to 3 visible faces (top, front, side).
        // We collect all faces with their depth for painter's algorithm.
        struct Face {
            corners: [(f64, f64); 4], // screen-space corners
            depth: f64,
            color: Color,
            char_fill: char,
        }

        let mut faces: Vec<Face> = Vec::new();

        for bar in &self.bars {
            let hw = bar.width / 2.0;
            let h = bar.height;

            // 8 corners of the bar in normalized [-1, 1] space
            let corners_data = [
                // bottom face (z=0)
                (bar.x - hw, bar.y - hw, 0.0), // 0: front-left-bottom
                (bar.x + hw, bar.y - hw, 0.0), // 1: front-right-bottom
                (bar.x + hw, bar.y + hw, 0.0), // 2: back-right-bottom
                (bar.x - hw, bar.y + hw, 0.0), // 3: back-left-bottom
                // top face (z=h)
                (bar.x - hw, bar.y - hw, h), // 4: front-left-top
                (bar.x + hw, bar.y - hw, h), // 5: front-right-top
                (bar.x + hw, bar.y + hw, h), // 6: back-right-top
                (bar.x - hw, bar.y + hw, h), // 7: back-left-top
            ];

            // Normalize to [-1, 1]
            let normalize = |cx: f64, cy: f64, cz: f64| -> (f64, f64, f64) {
                let nx = if x_range > 0.0 {
                    2.0 * (cx - x_min) / x_range - 1.0
                } else {
                    0.0
                };
                let ny = if y_range > 0.0 {
                    2.0 * (cy - y_min) / y_range - 1.0
                } else {
                    0.0
                };
                let nz = if z_range > 0.0 {
                    2.0 * cz / z_range - 1.0
                } else {
                    0.0
                };
                (nx, ny, nz * 0.8)
            };

            // Project all 8 corners
            let projected: Vec<(f64, f64, f64)> = corners_data
                .iter()
                .map(|&(cx, cy, cz)| {
                    let (nx, ny, nz) = normalize(cx, cy, cz);
                    camera.project(nx, ny, nz)
                })
                .collect();

            let resolved_color = bar.color.unwrap_or(self.theme.primary);
            let top_color = resolved_color;
            let front_color = scale_color(resolved_color, 0.7);
            let side_color = scale_color(resolved_color, 0.5);

            // Top face: corners 4, 5, 6, 7
            let top_depth =
                (projected[4].2 + projected[5].2 + projected[6].2 + projected[7].2) / 4.0;
            faces.push(Face {
                corners: [
                    (projected[4].0, projected[4].1),
                    (projected[5].0, projected[5].1),
                    (projected[6].0, projected[6].1),
                    (projected[7].0, projected[7].1),
                ],
                depth: top_depth,
                color: top_color,
                char_fill: self.theme.chars.depth.front,
            });

            // Front face: corners 0, 1, 5, 4
            let front_depth =
                (projected[0].2 + projected[1].2 + projected[5].2 + projected[4].2) / 4.0;
            faces.push(Face {
                corners: [
                    (projected[0].0, projected[0].1),
                    (projected[1].0, projected[1].1),
                    (projected[5].0, projected[5].1),
                    (projected[4].0, projected[4].1),
                ],
                depth: front_depth,
                color: front_color,
                char_fill: self.theme.chars.depth.side_near,
            });

            // Right side face: corners 1, 2, 6, 5
            let right_depth =
                (projected[1].2 + projected[2].2 + projected[6].2 + projected[5].2) / 4.0;
            faces.push(Face {
                corners: [
                    (projected[1].0, projected[1].1),
                    (projected[2].0, projected[2].1),
                    (projected[6].0, projected[6].1),
                    (projected[5].0, projected[5].1),
                ],
                depth: right_depth,
                color: side_color,
                char_fill: self.theme.chars.depth.side_far,
            });

            // Back face: corners 2, 3, 7, 6
            let back_depth =
                (projected[2].2 + projected[3].2 + projected[7].2 + projected[6].2) / 4.0;
            faces.push(Face {
                corners: [
                    (projected[2].0, projected[2].1),
                    (projected[3].0, projected[3].1),
                    (projected[7].0, projected[7].1),
                    (projected[6].0, projected[6].1),
                ],
                depth: back_depth,
                color: front_color,
                char_fill: self.theme.chars.depth.top_near,
            });

            // Left side face: corners 3, 0, 4, 7
            let left_depth =
                (projected[3].2 + projected[0].2 + projected[4].2 + projected[7].2) / 4.0;
            faces.push(Face {
                corners: [
                    (projected[3].0, projected[3].1),
                    (projected[0].0, projected[0].1),
                    (projected[4].0, projected[4].1),
                    (projected[7].0, projected[7].1),
                ],
                depth: left_depth,
                color: side_color,
                char_fill: self.theme.chars.depth.top_far,
            });
        }

        // Sort faces back-to-front (painter's algorithm)
        faces.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Find screen bounds of all projected corners for viewport mapping
        let mut sx_min = f64::INFINITY;
        let mut sx_max = f64::NEG_INFINITY;
        let mut sy_min = f64::INFINITY;
        let mut sy_max = f64::NEG_INFINITY;
        for face in &faces {
            for &(fx, fy) in &face.corners {
                sx_min = sx_min.min(fx);
                sx_max = sx_max.max(fx);
                sy_min = sy_min.min(fy);
                sy_max = sy_max.max(fy);
            }
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

        // Draw 3D axis lines BEFORE bars so the painter's algorithm occludes them
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

        // Project axis origin and tips from normalized [-1,1] space
        let origin = camera.project(-1.0, -1.0, -0.8);
        let x_tip = camera.project(1.0, -1.0, -0.8);
        let y_tip = camera.project(-1.0, 1.0, -0.8);
        let z_tip = camera.project(-1.0, -1.0, 0.8);

        let to_sx = |v: f64| data_to_screen(v, sx_min, sx_max, px as f64, (px + pw - 1) as f64);
        let to_sy = |v: f64| data_to_screen(v, sy_min, sy_max, py as f64, (py + ph - 1) as f64);

        let ox = to_sx(origin.0);
        let oy = to_sy(origin.1);

        // X axis line and label
        let xx = to_sx(x_tip.0);
        let xy = to_sy(x_tip.1);
        draw_braille_line(buf, ox, oy, xx, xy, self.theme.x_axis_3d_color, &pa);
        let xxi = xx.round() as u16;
        let xyi = xy.round() as u16;
        if xxi >= px && xxi < px + pw && xyi >= py && xyi < py + ph {
            buf[(xxi, xyi)]
                .set_char('X')
                .set_fg(self.theme.x_axis_3d_color);
        }

        // Y axis line and label
        let yx = to_sx(y_tip.0);
        let yy = to_sy(y_tip.1);
        draw_braille_line(buf, ox, oy, yx, yy, self.theme.y_axis_3d_color, &pa);
        let yxi = yx.round() as u16;
        let yyi = yy.round() as u16;
        if yxi >= px && yxi < px + pw && yyi >= py && yyi < py + ph {
            buf[(yxi, yyi)]
                .set_char('Y')
                .set_fg(self.theme.y_axis_3d_color);
        }

        // Z axis line and label
        let zx = to_sx(z_tip.0);
        let zy = to_sy(z_tip.1);
        draw_braille_line(buf, ox, oy, zx, zy, self.theme.z_axis_3d_color, &pa);
        let zxi = zx.round() as u16;
        let zyi = zy.round() as u16;
        if zxi >= px && zxi < px + pw && zyi >= py && zyi < py + ph {
            buf[(zxi, zyi)]
                .set_char('Z')
                .set_fg(self.theme.z_axis_3d_color);
        }

        // Rasterize each face (after axis lines, so bars occlude axes)
        for face in &faces {
            let screen_quad: Vec<(i32, i32)> = face
                .corners
                .iter()
                .map(|&(qx, qy)| {
                    let scx = data_to_screen(qx, sx_min, sx_max, px as f64, (px + pw - 1) as f64)
                        .round() as i32;
                    let scy = data_to_screen(qy, sy_min, sy_max, py as f64, (py + ph - 1) as f64)
                        .round() as i32;
                    (scx, scy)
                })
                .collect();

            // Bounding box
            let Some(bb_min_x) = screen_quad.iter().map(|c| c.0).min() else {
                continue;
            };
            let Some(bb_max_x) = screen_quad.iter().map(|c| c.0).max() else {
                continue;
            };
            let Some(bb_min_y) = screen_quad.iter().map(|c| c.1).min() else {
                continue;
            };
            let Some(bb_max_y) = screen_quad.iter().map(|c| c.1).max() else {
                continue;
            };

            // Fill using point-in-quad test
            for sy in bb_min_y..=bb_max_y {
                for sx in bb_min_x..=bb_max_x {
                    let ux = sx as u16;
                    let uy = sy as u16;
                    if ux >= px
                        && ux < px + pw
                        && uy >= py
                        && uy < py + ph
                        && point_in_quad(sx, sy, &screen_quad)
                    {
                        buf[(ux, uy)]
                            .set_char(face.char_fill)
                            .set_style(Style::default().fg(face.color));
                    }
                }
            }
        }
    }
}


impl Widget for &Bar3D {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_with_camera(&self.camera, area, buf);
    }
}

impl StatefulWidget for &Bar3D {
    type State = Camera3DState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let camera = state.to_camera();
        self.render_with_camera(&camera, area, buf);
    }
}

/// Darken or lighten a color by a factor (0.0 = black, 1.0 = unchanged).
fn scale_color(color: Color, factor: f64) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f64 * factor).round().clamp(0.0, 255.0) as u8,
            (g as f64 * factor).round().clamp(0.0, 255.0) as u8,
            (b as f64 * factor).round().clamp(0.0, 255.0) as u8,
        ),
        // For named colors, convert to approximate RGB, scale, then return RGB
        Color::Cyan => Color::Rgb(
            (0.0 * factor).round() as u8,
            (255.0 * factor).round().clamp(0.0, 255.0) as u8,
            (255.0 * factor).round().clamp(0.0, 255.0) as u8,
        ),
        Color::Yellow => Color::Rgb(
            (255.0 * factor).round().clamp(0.0, 255.0) as u8,
            (255.0 * factor).round().clamp(0.0, 255.0) as u8,
            (0.0 * factor).round() as u8,
        ),
        Color::Green => Color::Rgb(
            (0.0 * factor).round() as u8,
            (128.0 * factor).round().clamp(0.0, 255.0) as u8,
            (0.0 * factor).round() as u8,
        ),
        Color::Red => Color::Rgb(
            (255.0 * factor).round().clamp(0.0, 255.0) as u8,
            (0.0 * factor).round() as u8,
            (0.0 * factor).round() as u8,
        ),
        Color::Blue => Color::Rgb(
            (0.0 * factor).round() as u8,
            (0.0 * factor).round() as u8,
            (255.0 * factor).round().clamp(0.0, 255.0) as u8,
        ),
        Color::Magenta => Color::Rgb(
            (255.0 * factor).round().clamp(0.0, 255.0) as u8,
            (0.0 * factor).round() as u8,
            (255.0 * factor).round().clamp(0.0, 255.0) as u8,
        ),
        Color::White => Color::Rgb(
            (255.0 * factor).round().clamp(0.0, 255.0) as u8,
            (255.0 * factor).round().clamp(0.0, 255.0) as u8,
            (255.0 * factor).round().clamp(0.0, 255.0) as u8,
        ),
        // For other colors, just return them unmodified
        other => other,
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
