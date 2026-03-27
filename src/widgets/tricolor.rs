//! Filled triangle mesh plot widget with per-face colors.
//!
//! Renders triangles using scanline fill, mapping face values through a
//! colormap and normalization.
//! Not feature-gated — works with explicit triangulations.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::colormap::{Colormap, Viridis};
use crate::frame::{DataBounds, PlotArea, PlotFrame, ReferenceLine};
use crate::norm::{LinearNorm, Normalize};
use crate::plot_buffer::{PlotBackend, Z_DATA, create_backend};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::triangulation::Triangulation;

/// A filled triangle mesh plot widget.
///
/// Renders triangles with per-face colors mapped through a colormap.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::triangulation::Triangulation;
/// use ratatui_plt::widgets::tricolor::TriColor;
///
/// let tri = Triangulation::from_explicit(
///     vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0), (1.5, 1.0)],
///     vec![(0, 1, 2), (1, 3, 2)],
/// );
/// let plot = TriColor::new(tri)
///     .face_values(vec![0.3, 0.7])
///     .title("Filled Triangles");
/// ```
pub struct TriColor {
    triangulation: Triangulation,
    face_values: Vec<f64>,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
}

impl TriColor {
    /// Create a filled triangle plot from a triangulation.
    pub fn new(triangulation: Triangulation) -> Self {
        Self {
            triangulation,
            face_values: Vec::new(),
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(0.0, 1.0)),
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
        }
    }

    /// Set per-face values for color mapping.
    ///
    /// Each value corresponds to a triangle in the triangulation.
    /// The normalization range is updated to match the data.
    pub fn face_values(mut self, values: Vec<f64>) -> Self {
        if !values.is_empty() {
            let mut vmin = f64::INFINITY;
            let mut vmax = f64::NEG_INFINITY;
            for &v in &values {
                if v.is_finite() {
                    if v < vmin {
                        vmin = v;
                    }
                    if v > vmax {
                        vmax = v;
                    }
                }
            }
            if vmin.is_finite() && vmax.is_finite() {
                self.norm = Box::new(LinearNorm::new(vmin, vmax));
            }
        }
        self.face_values = values;
        self
    }

    /// Set the colormap.
    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    /// Set the normalization.
    pub fn norm(mut self, norm: impl Normalize + 'static) -> Self {
        self.norm = Box::new(norm);
        self
    }

    /// Set the plot title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the X axis.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y axis.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set spine visibility.
    pub fn spines(mut self, spines: Spines) -> Self {
        self.spines = spines;
        self
    }

    /// Add a reference line.
    pub fn reference_line(mut self, line: ReferenceLine) -> Self {
        self.reference_lines.push(line);
        self
    }

    /// Set all reference lines.
    pub fn reference_lines(mut self, lines: Vec<ReferenceLine>) -> Self {
        self.reference_lines = lines;
        self
    }
}

impl Widget for &TriColor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.triangulation.vertices.is_empty() || self.triangulation.triangles.is_empty() {
            return;
        }

        let (x_min, x_max, y_min, y_max) = self.triangulation.bounds();
        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let mut pb = create_backend(area);

        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render_to_pb(
            &mut pb,
            area,
            DataBounds {
                x_lo,
                x_hi,
                y_lo,
                y_hi,
            },
        ) else {
            return;
        };

        // Render each filled triangle using scanline fill
        for (tri_idx, &(a, b, c)) in self.triangulation.triangles.iter().enumerate() {
            let val = self.face_values.get(tri_idx).copied().unwrap_or(0.0);
            let t = self.norm.normalize(val);
            let color = self.colormap.color_at(t);

            let (ax, ay) = self.triangulation.vertices[a];
            let (bx, by) = self.triangulation.vertices[b];
            let (cx, cy) = self.triangulation.vertices[c];

            // Convert to screen coordinates
            let sax = pa.screen_x(ax);
            let say = pa.screen_y(ay);
            let sbx = pa.screen_x(bx);
            let sby = pa.screen_y(by);
            let scx = pa.screen_x(cx);
            let scy = pa.screen_y(cy);

            scanline_fill_triangle(
                &mut pb,
                [(sax, say), (sbx, sby), (scx, scy)],
                color,
                &pa,
                self.theme.chars.fill.solid,
            );
        }

        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);
    }
}

/// Fill a triangle using scanline algorithm.
fn scanline_fill_triangle(
    pb: &mut dyn PlotBackend,
    vertices: [(f64, f64); 3],
    color: Color,
    pa: &PlotArea,
    fill_char: char,
) {
    let [(x0, y0), (x1, y1), (x2, y2)] = vertices;
    // Sort vertices by y coordinate (top to bottom on screen)
    let mut verts = [(x0, y0), (x1, y1), (x2, y2)];
    verts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let [(vx0, vy0), (vx1, vy1), (vx2, vy2)] = verts;

    let iy_min = vy0.round() as i32;
    let iy_max = vy2.round() as i32;

    if iy_min == iy_max {
        // Degenerate horizontal triangle -- draw a horizontal line
        let min_x = x0.min(x1).min(x2).round() as u16;
        let max_x = x0.max(x1).max(x2).round() as u16;
        let sy = iy_min as u16;
        for sx in min_x..=max_x {
            if pa.contains(sx, sy) {
                pb.set_cell(sx, sy, fill_char, color, color, Z_DATA);
            }
        }
        return;
    }

    for iy in iy_min..=iy_max {
        let y = iy as f64;
        let mut x_intercepts = Vec::new();

        // Intersect scanline with each edge
        let edges = [
            (vx0, vy0, vx1, vy1),
            (vx1, vy1, vx2, vy2),
            (vx0, vy0, vx2, vy2),
        ];

        for (ex0, ey0, ex1, ey1) in &edges {
            let (ey_min, ey_max) = if ey0 < ey1 {
                (*ey0, *ey1)
            } else {
                (*ey1, *ey0)
            };
            if y >= ey_min && y <= ey_max && (ey1 - ey0).abs() > 1e-10 {
                let t = (y - ey0) / (ey1 - ey0);
                let x = ex0 + t * (ex1 - ex0);
                x_intercepts.push(x);
            }
        }

        x_intercepts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        x_intercepts.dedup_by(|a, b| (*a - *b).abs() < 0.5);

        if x_intercepts.len() >= 2 {
            let left = x_intercepts[0].round() as u16;
            let right = x_intercepts[x_intercepts.len() - 1].round() as u16;
            let sy = iy as u16;
            for sx in left..=right {
                if pa.contains(sx, sy) {
                    pb.set_cell(sx, sy, fill_char, color, color, Z_DATA);
                }
            }
        }
    }
}
