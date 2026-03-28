//! 3D parametric line/trajectory widget.
//!
//! Renders 3D line segments connecting points in each [`Series3D`],
//! with depth-cued brightness and optional markers at data points.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{StatefulWidget, Widget};

use crate::drawing::draw_braille_line;
use crate::frame::PlotArea;
use crate::series::Series3D;
use crate::style::MarkerShape;
use crate::theme::Theme;
use crate::transform::{Camera3D, Camera3DState, data_to_screen};

/// A 3D parametric line / trajectory widget.
///
/// Connects consecutive points in each [`Series3D`] with Braille line
/// segments, applying depth-cued brightness so that farther segments
/// appear dimmer. An optional marker can be drawn at each data point.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let helix = Series3D::new("helix").data(
///     (0..100)
///         .map(|i| {
///             let t = i as f64 * 0.1;
///             (t.cos(), t.sin(), t * 0.1)
///         })
///         .collect(),
/// );
/// let plot = Line3D::new()
///     .series(helix)
///     .camera(Camera3D::new().azimuth(-45.0));
/// ```
pub struct Line3D {
    series: Vec<Series3D>,
    camera: Camera3D,
    title: Option<String>,
    marker: Option<MarkerShape>,
    theme: Theme,
}

impl Default for Line3D {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            camera: Camera3D::default(),
            title: None,
            marker: None,
            theme: Theme::get_default(),
        }
    }
}

impl Line3D {
    /// Create a new empty `Line3D` widget.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a 3D series (trajectory) to the plot.
    pub fn series(mut self, s: Series3D) -> Self {
        self.series.push(s);
        self
    }

    /// Set the camera configuration.
    pub fn camera(mut self, cam: Camera3D) -> Self {
        self.camera = cam;
        self
    }

    /// Set the plot title (centered above the plot).
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Draw a marker at each data point in addition to lines.
    pub fn marker(mut self, m: MarkerShape) -> Self {
        self.marker = Some(m);
        self
    }

    /// Set the theme.
    pub fn theme(mut self, t: Theme) -> Self {
        self.theme = t;
        self
    }

    // ── core rendering logic ────────────────────────────────────────────

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

        // ── compute data bounds across all series ───────────────────────
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

        // ── project all points ──────────────────────────────────────────
        // Flat list per series: (sx, sy, depth, color, series_end_flag)
        struct Projected {
            sx: f64,
            sy: f64,
            depth: f64,
            color: Color,
            /// true when this is the last point in a series segment
            is_last: bool,
        }

        let mut projected: Vec<Projected> = Vec::new();

        for s in &self.series {
            let color = s.color.unwrap_or(self.theme.primary);
            let len = s.data.len();
            for (idx, &(x, y, z)) in s.data.iter().enumerate() {
                let nx = 2.0 * (x - x_min) / x_range - 1.0;
                let ny = 2.0 * (y - y_min) / y_range - 1.0;
                let nz = (2.0 * (z - z_min) / z_range - 1.0) * 0.5;
                let (sx, sy, depth) = camera.project(nx, ny, nz);
                projected.push(Projected {
                    sx,
                    sy,
                    depth,
                    color,
                    is_last: idx + 1 == len,
                });
            }
        }

        if projected.is_empty() {
            return;
        }

        // ── screen bounds ───────────────────────────────────────────────
        let mut sx_min = f64::INFINITY;
        let mut sx_max = f64::NEG_INFINITY;
        let mut sy_min = f64::INFINITY;
        let mut sy_max = f64::NEG_INFINITY;

        for p in &projected {
            sx_min = sx_min.min(p.sx);
            sx_max = sx_max.max(p.sx);
            sy_min = sy_min.min(p.sy);
            sy_max = sy_max.max(p.sy);
        }

        // Apply zoom
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

        // Depth range for brightness cuing
        let depth_min = projected
            .iter()
            .map(|p| p.depth)
            .fold(f64::INFINITY, f64::min);
        let depth_max = projected
            .iter()
            .map(|p| p.depth)
            .fold(f64::NEG_INFINITY, f64::max);
        let depth_range = if depth_max == depth_min {
            1.0
        } else {
            depth_max - depth_min
        };

        // Build PlotArea for Braille line clipping
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

        let map_x = |v: f64| data_to_screen(v, sx_min, sx_max, px as f64, (px + pw - 1) as f64);
        let map_y = |v: f64| data_to_screen(v, sy_min, sy_max, py as f64, (py + ph - 1) as f64);

        // ── collect line segments with depth for sorting ────────────────
        struct Segment {
            x0: f64,
            y0: f64,
            x1: f64,
            y1: f64,
            color: Color,
            depth: f64,
        }

        let mut segments: Vec<Segment> = Vec::new();

        for i in 0..projected.len().saturating_sub(1) {
            // Do not connect across series boundaries.
            if projected[i].is_last {
                continue;
            }
            let p0 = &projected[i];
            let p1 = &projected[i + 1];
            let avg_depth = (p0.depth + p1.depth) / 2.0;
            segments.push(Segment {
                x0: map_x(p0.sx),
                y0: map_y(p0.sy),
                x1: map_x(p1.sx),
                y1: map_y(p1.sy),
                color: p0.color,
                depth: avg_depth,
            });
        }

        // Sort back-to-front (farthest first so nearer segments overwrite)
        segments.sort_by(|a, b| {
            a.depth
                .partial_cmp(&b.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Draw segments with depth-cued brightness
        for seg in &segments {
            let brightness = ((seg.depth - depth_min) / depth_range * 0.7 + 0.3).clamp(0.3, 1.0);
            let color = dim_color(seg.color, brightness);
            draw_braille_line(buf, seg.x0, seg.y0, seg.x1, seg.y1, color, &pa);
        }

        // ── optional markers at data points ─────────────────────────────
        if let Some(marker) = self.marker {
            // Sort points back-to-front and draw markers
            let mut indices: Vec<usize> = (0..projected.len()).collect();
            indices.sort_by(|&a, &b| {
                projected[a]
                    .depth
                    .partial_cmp(&projected[b].depth)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            for &idx in &indices {
                let p = &projected[idx];
                let scx = map_x(p.sx).round() as u16;
                let scy = map_y(p.sy).round() as u16;
                if scx >= px && scx < px + pw && scy >= py && scy < py + ph {
                    let brightness =
                        ((p.depth - depth_min) / depth_range * 0.7 + 0.3).clamp(0.3, 1.0);
                    let color = dim_color(p.color, brightness);
                    buf[(scx, scy)].set_char(marker.char()).set_fg(color);
                }
            }
        }

        // ── 3D axis lines and labels ────────────────────────────────────
        let sb = ScreenBounds {
            sx_min,
            sx_max,
            sy_min,
            sy_max,
        };
        draw_axis_lines(camera, buf, &pa, &sb, &self.theme);
    }
}

// ── screen bounds ───────────────────────────────────────────────────────────

struct ScreenBounds {
    sx_min: f64,
    sx_max: f64,
    sy_min: f64,
    sy_max: f64,
}

// ── axis helpers ────────────────────────────────────────────────────────────

fn draw_axis_lines(
    camera: &Camera3D,
    buf: &mut Buffer,
    pa: &PlotArea,
    sb: &ScreenBounds,
    theme: &Theme,
) {
    let (px, pw, py, ph) = (pa.x, pa.width, pa.y, pa.height);
    let (sx_min, sx_max, sy_min, sy_max) = (sb.sx_min, sb.sx_max, sb.sy_min, sb.sy_max);

    let to_sx = |v: f64| data_to_screen(v, sx_min, sx_max, px as f64, (px + pw - 1) as f64);
    let to_sy = |v: f64| data_to_screen(v, sy_min, sy_max, py as f64, (py + ph - 1) as f64);

    let origin = camera.project(0.0, 0.0, 0.0);
    let x_tip = camera.project(0.5, 0.0, 0.0);
    let y_tip = camera.project(0.0, 0.5, 0.0);
    let z_tip = camera.project(0.0, 0.0, 0.3);

    let ox = to_sx(origin.0);
    let oy = to_sy(origin.1);

    // X axis
    let xx = to_sx(x_tip.0);
    let xy = to_sy(x_tip.1);
    draw_braille_line(buf, ox, oy, xx, xy, theme.x_axis_3d_color, pa);
    let xxi = xx.round() as u16;
    let xyi = xy.round() as u16;
    if xxi >= px && xxi < px + pw && xyi >= py && xyi < py + ph {
        buf[(xxi, xyi)].set_char('X').set_fg(theme.x_axis_3d_color);
    }

    // Y axis
    let yx = to_sx(y_tip.0);
    let yy = to_sy(y_tip.1);
    draw_braille_line(buf, ox, oy, yx, yy, theme.y_axis_3d_color, pa);
    let yxi = yx.round() as u16;
    let yyi = yy.round() as u16;
    if yxi >= px && yxi < px + pw && yyi >= py && yyi < py + ph {
        buf[(yxi, yyi)].set_char('Y').set_fg(theme.y_axis_3d_color);
    }

    // Z axis
    let zx = to_sx(z_tip.0);
    let zy = to_sy(z_tip.1);
    draw_braille_line(buf, ox, oy, zx, zy, theme.z_axis_3d_color, pa);
    let zxi = zx.round() as u16;
    let zyi = zy.round() as u16;
    if zxi >= px && zxi < px + pw && zyi >= py && zyi < py + ph {
        buf[(zxi, zyi)].set_char('Z').set_fg(theme.z_axis_3d_color);
    }
}

// ── colour helpers ──────────────────────────────────────────────────────────

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

// ── Widget / StatefulWidget impls ───────────────────────────────────────────

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for Line3D {
    fn render_plotters(
        &self,
        area: ratatui::layout::Rect,
        buf: &mut ratatui::buffer::Buffer,
        theme: &crate::theme::Theme,
    ) {
        use crate::plotters_render::{bridge, theme_bridge};
        bridge::render_plotters_to_buf(
            area, buf, theme_bridge::theme_bg_rgb(theme),
            |_root| { /* Minimal stub - full plotters rendering TBD */ },
        );
    }
}

impl Widget for &Line3D {
    fn render(self, area: Rect, buf: &mut Buffer) {
        #[cfg(feature = "plotters-render")]
        {
            if crate::plotters_render::should_use_plotters() {
                use crate::plotters_render::PlottersRenderable;
                self.render_plotters(area, buf, &self.theme);
                return;
            }
        }
        self.render_with_camera(&self.camera, area, buf);
    }
}

impl StatefulWidget for &Line3D {
    type State = Camera3DState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let camera = state.to_camera();
        self.render_with_camera(&camera, area, buf);
    }
}
