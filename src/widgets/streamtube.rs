//! 3D streamtube widget — streamlines rendered as thick depth-cued tubes.
//!
//! Each path is a sequence of 3D points representing a streamline through a
//! vector field. Tubes are drawn as Braille lines with depth-cued brightness
//! and optional per-vertex coloring via a colormap.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{StatefulWidget, Widget};

use crate::colormap::{Colormap, Viridis};
use crate::drawing::draw_braille_line;
use crate::frame::PlotArea;
use crate::norm::{LinearNorm, Normalize};
use crate::theme::Theme;
use crate::transform::{Camera3D, Camera3DState, data_to_screen};

/// A 3D streamtube widget.
///
/// Renders streamline paths as depth-cued Braille tubes with optional
/// per-vertex scalar coloring.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let path = vec![(0.0, 0.0, 0.0), (1.0, 0.5, 0.2), (2.0, 1.0, 0.5)];
/// let plot = Streamtube::new(vec![path])
///     .camera(Camera3D::new().azimuth(-45.0))
///     .title("Flow");
/// ```
pub struct Streamtube {
    paths: Vec<Vec<(f64, f64, f64)>>,
    values: Option<Vec<Vec<f64>>>,
    camera: Camera3D,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    title: Option<String>,
    tube_radius: f64,
    theme: Theme,
}

impl Default for Streamtube {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            values: None,
            camera: Camera3D::default(),
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(0.0, 1.0)),
            title: None,
            tube_radius: 0.05,
            theme: Theme::get_default(),
        }
    }
}

impl Streamtube {
    pub fn new(paths: Vec<Vec<(f64, f64, f64)>>) -> Self {
        Self {
            paths,
            ..Self::default()
        }
    }

    pub fn values(mut self, v: Vec<Vec<f64>>) -> Self {
        // Compute norm bounds
        let (mut vmin, mut vmax) = (f64::INFINITY, f64::NEG_INFINITY);
        for row in &v {
            for &val in row {
                if val.is_finite() {
                    vmin = vmin.min(val);
                    vmax = vmax.max(val);
                }
            }
        }
        if vmin >= vmax {
            vmax = vmin + 1.0;
        }
        self.norm = Box::new(LinearNorm::new(vmin, vmax));
        self.values = Some(v);
        self
    }

    pub fn camera(mut self, c: Camera3D) -> Self {
        self.camera = c;
        self
    }

    pub fn colormap(mut self, c: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(c);
        self
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn tube_radius(mut self, r: f64) -> Self {
        self.tube_radius = r;
        self
    }

    pub fn theme(mut self, t: Theme) -> Self {
        self.theme = t;
        self
    }

    fn render_with_camera(&self, area: Rect, buf: &mut Buffer, camera: &Camera3D) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        // Render title
        let plot_area = if let Some(ref t) = self.title {
            let tx = area.x + area.width.saturating_sub(t.len() as u16) / 2;
            for (i, ch) in t.chars().enumerate() {
                let x = tx + i as u16;
                if x < area.x + area.width
                    && let Some(cell) = buf.cell_mut((x, area.y)) {
                        cell.set_char(ch);
                        cell.set_fg(self.theme.foreground);
                    }
            }
            Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1))
        } else {
            area
        };

        // Collect all 3D points for bounds
        let mut all_pts: Vec<(f64, f64, f64)> = Vec::new();
        for path in &self.paths {
            all_pts.extend(path.iter().copied());
        }
        if all_pts.is_empty() {
            return;
        }

        // Project all points
        let projected: Vec<(f64, f64, f64)> = all_pts
            .iter()
            .map(|&(x, y, z)| camera.project(x, y, z))
            .collect();

        let (sx_min, sx_max, sy_min, sy_max) = projected.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY),
            |(sxn, sxx, syn, syx), &(sx, sy, _)| {
                (sxn.min(sx), sxx.max(sx), syn.min(sy), syx.max(sy))
            },
        );
        if sx_min >= sx_max || sy_min >= sy_max {
            return;
        }

        let pa = PlotArea {
            x: plot_area.x,
            y: plot_area.y,
            width: plot_area.width,
            height: plot_area.height,
            x_lo: sx_min,
            x_hi: sx_max,
            y_lo: sy_min,
            y_hi: sy_max,
            area: plot_area,
        };

        // Find depth range for brightness
        let (depth_min, depth_max) = projected.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(dmin, dmax), &(_, _, d)| (dmin.min(d), dmax.max(d)),
        );
        let depth_range = if (depth_max - depth_min).abs() < 1e-10 {
            1.0
        } else {
            depth_max - depth_min
        };

        // Collect segments with depth for sorting
        struct Seg {
            sx0: f64,
            sy0: f64,
            sx1: f64,
            sy1: f64,
            depth: f64,
            color: Color,
        }

        let mut segments: Vec<Seg> = Vec::new();
        let mut pt_idx = 0;

        for (pi, path) in self.paths.iter().enumerate() {
            let path_color = self.theme.color_cycle.at(pi);
            for i in 0..path.len().saturating_sub(1) {
                let idx0 = pt_idx + i;
                let idx1 = pt_idx + i + 1;
                if idx0 >= projected.len() || idx1 >= projected.len() {
                    break;
                }
                let p0 = projected[idx0];
                let p1 = projected[idx1];
                let avg_depth = (p0.2 + p1.2) / 2.0;

                let color = if let Some(ref vals) = self.values {
                    if pi < vals.len() && i < vals[pi].len() {
                        let t = self.norm.normalize(vals[pi][i]);
                        self.colormap.color_at(t)
                    } else {
                        path_color
                    }
                } else {
                    path_color
                };

                let map_x = |sx: f64| {
                    data_to_screen(sx, sx_min, sx_max, pa.x as f64, (pa.x + pa.width - 1) as f64)
                };
                let map_y = |sy: f64| {
                    data_to_screen(sy, sy_min, sy_max, pa.y as f64, (pa.y + pa.height - 1) as f64)
                };

                segments.push(Seg {
                    sx0: map_x(p0.0),
                    sy0: map_y(p0.1),
                    sx1: map_x(p1.0),
                    sy1: map_y(p1.1),
                    depth: avg_depth,
                    color,
                });
            }
            pt_idx += path.len();
        }

        // Sort back-to-front
        segments.sort_by(|a, b| {
            a.depth
                .partial_cmp(&b.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Draw segments with depth-cued brightness and tube offset
        for seg in &segments {
            let brightness = ((seg.depth - depth_min) / depth_range * 200.0 + 55.0)
                .clamp(55.0, 255.0) as u8;
            let (r, g, b_val) = match seg.color {
                Color::Rgb(r, g, b) => (r, g, b),
                _ => (200, 200, 200),
            };
            let color = Color::Rgb(
                (r as f64 * brightness as f64 / 255.0) as u8,
                (g as f64 * brightness as f64 / 255.0) as u8,
                (b_val as f64 * brightness as f64 / 255.0) as u8,
            );

            // Draw main line
            draw_braille_line(buf, seg.sx0, seg.sy0, seg.sx1, seg.sy1, color, &pa);

            // Draw offset lines for "tube" thickness
            let dx = seg.sx1 - seg.sx0;
            let dy = seg.sy1 - seg.sy0;
            let len = (dx * dx + dy * dy).sqrt().max(1e-10);
            let nx = -dy / len;
            let ny = dx / len;
            let r_px = self.tube_radius * pa.width as f64 * 0.1;

            if r_px > 0.5 {
                let dim = Color::Rgb(
                    (r as f64 * brightness as f64 / 400.0) as u8,
                    (g as f64 * brightness as f64 / 400.0) as u8,
                    (b_val as f64 * brightness as f64 / 400.0) as u8,
                );
                draw_braille_line(
                    buf,
                    seg.sx0 + nx * r_px,
                    seg.sy0 + ny * r_px,
                    seg.sx1 + nx * r_px,
                    seg.sy1 + ny * r_px,
                    dim,
                    &pa,
                );
                draw_braille_line(
                    buf,
                    seg.sx0 - nx * r_px,
                    seg.sy0 - ny * r_px,
                    seg.sx1 - nx * r_px,
                    seg.sy1 - ny * r_px,
                    dim,
                    &pa,
                );
            }
        }
    }
}

impl Widget for &Streamtube {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_with_camera(area, buf, &self.camera);
    }
}

impl StatefulWidget for &Streamtube {
    type State = Camera3DState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let camera = state.to_camera();
        self.render_with_camera(area, buf, &camera);
    }
}
