//! Carpet plot widget for 2D parameterized surfaces.
//!
//! Renders a curvilinear (a, b) coordinate grid where the x and y positions
//! are arbitrary functions of the two parameters. Optionally colors grid cells
//! using a scalar field mapped through a colormap.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::carpet::CarpetPlot;
//!
//! let a = vec![0.0, 1.0, 2.0, 3.0];
//! let b = vec![0.0, 1.0, 2.0];
//! let mut x = vec![vec![0.0; a.len()]; b.len()];
//! let mut y = vec![vec![0.0; a.len()]; b.len()];
//! for (bi, &bv) in b.iter().enumerate() {
//!     for (ai, &av) in a.iter().enumerate() {
//!         x[bi][ai] = av + 0.1 * bv;
//!         y[bi][ai] = bv + 0.2 * av;
//!     }
//! }
//! let plot = CarpetPlot::new(a, b, x, y).title("Carpet");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::colormap::{Colorbar, Colormap, Viridis};
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::norm::{LinearNorm, Normalize};
use crate::plot_buffer::{PlotBackend, Z_DATA, Z_GRID, create_backend};
use crate::spines::Spines;
use crate::theme::Theme;

/// A carpet plot widget for 2D parameterized surfaces.
///
/// The carpet plot displays a curvilinear grid defined by two parameter arrays
/// `a` and `b`, with the grid positions given by `x[b_idx][a_idx]` and
/// `y[b_idx][a_idx]`. An optional scalar field `values[b_idx][a_idx]` can be
/// used to color the grid cells through a colormap.
pub struct CarpetPlot {
    /// Parameter a values.
    a: Vec<f64>,
    /// Parameter b values.
    b: Vec<f64>,
    /// X positions: x[b_idx][a_idx].
    x: Vec<Vec<f64>>,
    /// Y positions: y[b_idx][a_idx].
    y: Vec<Vec<f64>>,
    /// Optional scalar values for coloring: values[b_idx][a_idx].
    values: Option<Vec<Vec<f64>>>,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_grid: bool,
    show_colorbar: bool,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
}

impl CarpetPlot {
    /// Create a new carpet plot from parameter arrays and position grids.
    ///
    /// `a_vals` and `b_vals` are the parameter values along each axis.
    /// `x_grid` and `y_grid` are 2D arrays of shape `[b.len()][a.len()]`
    /// giving the x and y positions in data space.
    pub fn new(
        a_vals: Vec<f64>,
        b_vals: Vec<f64>,
        x_grid: Vec<Vec<f64>>,
        y_grid: Vec<Vec<f64>>,
    ) -> Self {
        Self {
            a: a_vals,
            b: b_vals,
            x: x_grid,
            y: y_grid,
            values: None,
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(0.0, 1.0)),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_grid: true,
            show_colorbar: false,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
        }
    }

    /// Set optional scalar values for coloring grid cells.
    ///
    /// `val_grid` should have shape `[b.len()][a.len()]`.
    pub fn values(mut self, val_grid: Vec<Vec<f64>>) -> Self {
        let (vmin, vmax) = value_bounds_2d(&val_grid);
        self.norm = Box::new(LinearNorm::new(vmin, vmax));
        self.show_colorbar = true;
        self.values = Some(val_grid);
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

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Show or hide the curvilinear grid lines.
    pub fn show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    /// Show or hide the colorbar.
    pub fn show_colorbar(mut self, show: bool) -> Self {
        self.show_colorbar = show;
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
}

/// Compute the min and max of a 2D value array, ignoring non-finite values.
fn value_bounds_2d(values: &[Vec<f64>]) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for row in values {
        for &v in row {
            if v.is_finite() {
                if v < min {
                    min = v;
                }
                if v > max {
                    max = v;
                }
            }
        }
    }
    if min.is_infinite() {
        (0.0, 1.0)
    } else {
        (min, max)
    }
}

/// Compute data bounds from position grids.
fn grid_bounds(x: &[Vec<f64>], y: &[Vec<f64>]) -> (f64, f64, f64, f64) {
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for row in x {
        for &v in row {
            if v.is_finite() {
                x_min = x_min.min(v);
                x_max = x_max.max(v);
            }
        }
    }
    for row in y {
        for &v in row {
            if v.is_finite() {
                y_min = y_min.min(v);
                y_max = y_max.max(v);
            }
        }
    }
    (x_min, x_max, y_min, y_max)
}


impl Widget for &CarpetPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let nb = self.b.len();
        let na = self.a.len();
        if nb < 2 || na < 2 {
            return;
        }

        // Validate grid dimensions
        if self.x.len() != nb || self.y.len() != nb {
            return;
        }
        for row in &self.x {
            if row.len() != na {
                return;
            }
        }
        for row in &self.y {
            if row.len() != na {
                return;
            }
        }

        // Validate values dimensions if present
        if let Some(ref vals) = self.values {
            if vals.len() != nb {
                return;
            }
            for row in vals {
                if row.len() != na {
                    return;
                }
            }
        }

        let (x_min, x_max, y_min, y_max) = grid_bounds(&self.x, &self.y);
        if !x_min.is_finite() || !y_min.is_finite() {
            return;
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let colorbar_width: u16 = if self.show_colorbar { 10 } else { 0 };

        let mut pb = create_backend(area);

        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .colorbar_width(colorbar_width)
            .y_label_width(7)
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

        // If values are provided, fill grid cells with colormap colors
        if let Some(ref vals) = self.values {
            for bi in 0..nb.saturating_sub(1) {
                for ai in 0..na.saturating_sub(1) {
                    // Average the four corner values for cell color
                    let v00 = vals[bi][ai];
                    let v01 = vals[bi][ai + 1];
                    let v10 = vals[bi + 1][ai];
                    let v11 = vals[bi + 1][ai + 1];
                    if !v00.is_finite() || !v01.is_finite() || !v10.is_finite() || !v11.is_finite()
                    {
                        continue;
                    }
                    let avg = (v00 + v01 + v10 + v11) / 4.0;
                    let t = self.norm.normalize(avg);
                    let color = self.colormap.color_at(t);

                    // Get the four corners in screen space
                    let corners = [
                        (pa.screen_x(self.x[bi][ai]), pa.screen_y(self.y[bi][ai])),
                        (
                            pa.screen_x(self.x[bi][ai + 1]),
                            pa.screen_y(self.y[bi][ai + 1]),
                        ),
                        (
                            pa.screen_x(self.x[bi + 1][ai + 1]),
                            pa.screen_y(self.y[bi + 1][ai + 1]),
                        ),
                        (
                            pa.screen_x(self.x[bi + 1][ai]),
                            pa.screen_y(self.y[bi + 1][ai]),
                        ),
                    ];

                    // Compute bounding box
                    let bb_min_x = corners
                        .iter()
                        .map(|c| c.0)
                        .fold(f64::INFINITY, f64::min)
                        .floor() as i32;
                    let bb_max_x = corners
                        .iter()
                        .map(|c| c.0)
                        .fold(f64::NEG_INFINITY, f64::max)
                        .ceil() as i32;
                    let bb_min_y = corners
                        .iter()
                        .map(|c| c.1)
                        .fold(f64::INFINITY, f64::min)
                        .floor() as i32;
                    let bb_max_y = corners
                        .iter()
                        .map(|c| c.1)
                        .fold(f64::NEG_INFINITY, f64::max)
                        .ceil() as i32;

                    let quad = [corners[0], corners[1], corners[2], corners[3]];
                    for sy in bb_min_y..=bb_max_y {
                        for sx in bb_min_x..=bb_max_x {
                            let ux = sx as u16;
                            let uy = sy as u16;
                            if pa.contains(ux, uy) && point_in_quad(sx as f64, sy as f64, &quad) {
                                pb.set_cell(
                                    ux,
                                    uy,
                                    self.theme.chars.fill.solid,
                                    color,
                                    color,
                                    Z_DATA,
                                );
                            }
                        }
                    }
                }
            }
        }

        // Draw curvilinear grid lines using Braille
        if self.show_grid {
            let grid_color = self.theme.grid_color;
            let z = if self.values.is_some() {
                Z_DATA + 1
            } else {
                Z_GRID
            };

            // Constant-a lines: for each a index, connect across all b values
            for ai in 0..na {
                for bi in 0..nb.saturating_sub(1) {
                    let sx0 = pa.screen_x(self.x[bi][ai]);
                    let sy0 = pa.screen_y(self.y[bi][ai]);
                    let sx1 = pa.screen_x(self.x[bi + 1][ai]);
                    let sy1 = pa.screen_y(self.y[bi + 1][ai]);
                    pb.draw_line(sx0, sy0, sx1, sy1, grid_color, &pa, z);
                }
            }

            // Constant-b lines: for each b index, connect across all a values
            for bi in 0..nb {
                for ai in 0..na.saturating_sub(1) {
                    let sx0 = pa.screen_x(self.x[bi][ai]);
                    let sy0 = pa.screen_y(self.y[bi][ai]);
                    let sx1 = pa.screen_x(self.x[bi][ai + 1]);
                    let sy1 = pa.screen_y(self.y[bi][ai + 1]);
                    pb.draw_line(sx0, sy0, sx1, sy1, grid_color, &pa, z);
                }
            }
        }

        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Draw colorbar
        if self.show_colorbar
            && let Some(ref vals) = self.values
        {
            let (vmin, vmax) = value_bounds_2d(vals);
            let cb = Colorbar::new(self.colormap.as_ref(), vmin, vmax)
                .label_color(self.theme.foreground);
            let cb_area = Rect::new(
                pa.x + pa.width + 2,
                pa.y,
                colorbar_width.min(
                    area.x
                        .saturating_add(area.width)
                        .saturating_sub(pa.x + pa.width + 2),
                ),
                pa.height,
            );
            if cb_area.x + cb_area.width <= area.x + area.width {
                (&cb).render(cb_area, buf);
            }
        }
    }
}

/// Test if a point is inside a convex quadrilateral using cross-product winding.
fn point_in_quad(px: f64, py: f64, quad: &[(f64, f64); 4]) -> bool {
    let mut sign = 0i32;
    for i in 0..4 {
        let (x0, y0) = quad[i];
        let (x1, y1) = quad[(i + 1) % 4];
        let cross = (x1 - x0) * (py - y0) - (y1 - y0) * (px - x0);
        if cross.abs() > 1e-12 {
            let s = if cross > 0.0 { 1 } else { -1 };
            if sign == 0 {
                sign = s;
            } else if sign != s {
                return false;
            }
        }
    }
    true
}
