//! Contour plot on triangulated data using the marching triangles algorithm.
//!
//! Renders contour lines by linearly interpolating along triangle edges
//! for each contour level.
//! Not feature-gated — works with explicit triangulations.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::colormap::{Colormap, Viridis};
use crate::frame::{DataBounds, PlotArea, PlotFrame, ReferenceLine};
use crate::norm::{LinearNorm, Normalize};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::triangulation::Triangulation;

/// A contour plot on triangulated data.
///
/// Uses the marching triangles algorithm to draw contour lines at
/// specified levels through triangulated scalar data.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::triangulation::Triangulation;
/// use ratatui_plt::widgets::tricontour::TriContour;
///
/// let tri = Triangulation::from_explicit(
///     vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0), (1.5, 1.0)],
///     vec![(0, 1, 2), (1, 3, 2)],
/// );
/// let plot = TriContour::new(tri)
///     .vertex_values(vec![0.0, 1.0, 0.5, 1.5])
///     .levels_auto(5)
///     .title("Contour on Triangulation");
/// ```
pub struct TriContour {
    triangulation: Triangulation,
    vertex_values: Vec<f64>,
    levels: Vec<f64>,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
}

impl TriContour {
    /// Create a contour plot from a triangulation.
    pub fn new(triangulation: Triangulation) -> Self {
        Self {
            triangulation,
            vertex_values: Vec::new(),
            levels: Vec::new(),
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

    /// Set per-vertex scalar values.
    ///
    /// Each value corresponds to a vertex in the triangulation.
    /// The normalization range is updated to match the data.
    pub fn vertex_values(mut self, values: Vec<f64>) -> Self {
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
        self.vertex_values = values;
        self
    }

    /// Set explicit contour level values.
    pub fn levels(mut self, levels: Vec<f64>) -> Self {
        self.levels = levels;
        self
    }

    /// Set the number of auto-spaced contour levels.
    pub fn levels_auto(mut self, n: usize) -> Self {
        // Levels will be computed at render time using vertex value range
        self.levels = vec![f64::NAN; n]; // sentinel; replaced at render
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

impl Widget for &TriContour {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.triangulation.vertices.is_empty()
            || self.triangulation.triangles.is_empty()
            || self.vertex_values.is_empty()
        {
            return;
        }

        let (x_min, x_max, y_min, y_max) = self.triangulation.bounds();
        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, DataBounds { x_lo, x_hi, y_lo, y_hi }) else {
            return;
        };

        // Resolve levels
        let levels = if self.levels.is_empty() {
            // Default: 8 levels
            compute_auto_levels(&self.vertex_values, 8)
        } else if self.levels.iter().any(|v| v.is_nan()) {
            // levels_auto sentinel: use the count
            compute_auto_levels(&self.vertex_values, self.levels.len())
        } else {
            self.levels.clone()
        };

        // Marching triangles: for each level, for each triangle, find edge crossings
        for &level in &levels {
            let t = self.norm.normalize(level);
            let color = self.colormap.color_at(t);

            for &(ia, ib, ic) in &self.triangulation.triangles {
                let va = *self.vertex_values.get(ia).unwrap_or(&0.0);
                let vb = *self.vertex_values.get(ib).unwrap_or(&0.0);
                let vc = *self.vertex_values.get(ic).unwrap_or(&0.0);

                let (xa, ya) = self.triangulation.vertices[ia];
                let (xb, yb) = self.triangulation.vertices[ib];
                let (xc, yc) = self.triangulation.vertices[ic];

                // Classify each vertex as above or below the level
                let above_a = va >= level;
                let above_b = vb >= level;
                let above_c = vc >= level;

                let case = (above_a as u8) | ((above_b as u8) << 1) | ((above_c as u8) << 2);

                // All same side: no contour in this triangle
                if case == 0 || case == 7 {
                    continue;
                }

                // Interpolate crossing point along an edge
                let interp_edge =
                    |v0: f64, x0: f64, y0: f64, v1: f64, x1: f64, y1: f64| -> (f64, f64) {
                        let dv = v1 - v0;
                        let t_edge = if dv.abs() < 1e-12 {
                            0.5
                        } else {
                            (level - v0) / dv
                        };
                        (x0 + t_edge * (x1 - x0), y0 + t_edge * (y1 - y0))
                    };

                // Find the two crossing points
                let crossings: Vec<(f64, f64)> = match case {
                    // One vertex above: crossings on the two edges from that vertex
                    1 => vec![
                        interp_edge(va, xa, ya, vb, xb, yb),
                        interp_edge(va, xa, ya, vc, xc, yc),
                    ],
                    2 => vec![
                        interp_edge(vb, xb, yb, va, xa, ya),
                        interp_edge(vb, xb, yb, vc, xc, yc),
                    ],
                    4 => vec![
                        interp_edge(vc, xc, yc, va, xa, ya),
                        interp_edge(vc, xc, yc, vb, xb, yb),
                    ],
                    // Two vertices above: crossings on the two edges from the below vertex
                    3 => vec![
                        interp_edge(vc, xc, yc, va, xa, ya),
                        interp_edge(vc, xc, yc, vb, xb, yb),
                    ],
                    5 => vec![
                        interp_edge(vb, xb, yb, va, xa, ya),
                        interp_edge(vb, xb, yb, vc, xc, yc),
                    ],
                    6 => vec![
                        interp_edge(va, xa, ya, vb, xb, yb),
                        interp_edge(va, xa, ya, vc, xc, yc),
                    ],
                    _ => vec![],
                };

                if crossings.len() == 2 {
                    let (dx0, dy0) = crossings[0];
                    let (dx1, dy1) = crossings[1];

                    let sx0 = pa.screen_x(dx0);
                    let sy0 = pa.screen_y(dy0);
                    let sx1 = pa.screen_x(dx1);
                    let sy1 = pa.screen_y(dy1);

                    draw_contour_line(buf, sx0, sy0, sx1, sy1, color, &pa);
                }
            }
        }
    }
}

/// Compute auto-spaced contour levels from vertex values.
fn compute_auto_levels(values: &[f64], n: usize) -> Vec<f64> {
    let mut vmin = f64::INFINITY;
    let mut vmax = f64::NEG_INFINITY;
    for &v in values {
        if v.is_finite() {
            if v < vmin {
                vmin = v;
            }
            if v > vmax {
                vmax = v;
            }
        }
    }
    if !vmin.is_finite() || !vmax.is_finite() || n == 0 {
        return Vec::new();
    }
    (0..n)
        .map(|i| vmin + (vmax - vmin) * (i as f64 + 0.5) / n as f64)
        .collect()
}

/// Draw a line between two screen-space points using Bresenham's algorithm.
fn draw_contour_line(
    buf: &mut Buffer,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
    pa: &PlotArea,
) {
    let mut ix0 = x0.round() as i32;
    let mut iy0 = y0.round() as i32;
    let ix1 = x1.round() as i32;
    let iy1 = y1.round() as i32;

    let dx = (ix1 - ix0).abs();
    let dy = -(iy1 - iy0).abs();
    let sx = if ix0 < ix1 { 1 } else { -1 };
    let sy = if iy0 < iy1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        let px = ix0 as u16;
        let py = iy0 as u16;
        if pa.contains(px, py) {
            buf[(px, py)].set_char('\u{00b7}').set_fg(color);
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
