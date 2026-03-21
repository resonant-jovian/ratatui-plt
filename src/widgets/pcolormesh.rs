//! Pseudocolor mesh widget for irregular quadrilateral grids.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis};
use crate::colormap::{Colorbar, Colormap, Viridis};
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::norm::{LinearNorm, Normalize};
use crate::spines::Spines;
use crate::theme::Theme;

/// A pseudocolor mesh plot widget for irregular quadrilateral grids.
///
/// Each cell `(i, j)` is defined by 4 corner vertices and colored according
/// to its value through a colormap. Unlike [`Heatmap`](super::heatmap::Heatmap)
/// which requires a regular grid, `Pcolormesh` supports arbitrary vertex
/// positions (e.g. curvilinear or polar meshes).
///
/// The vertex arrays `x` and `y` have shape `(nrows+1) x (ncols+1)`, while
/// the value array has shape `nrows x ncols`.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::widgets::pcolormesh::Pcolormesh;
/// use std::f64::consts::PI;
///
/// // Polar grid
/// let nr = 10;
/// let nt = 20;
/// let mut x = vec![vec![0.0; nt + 1]; nr + 1];
/// let mut y = vec![vec![0.0; nt + 1]; nr + 1];
/// let mut values = vec![vec![0.0; nt]; nr];
/// for i in 0..=nr {
///     let r = i as f64 / nr as f64;
///     for j in 0..=nt {
///         let theta = 2.0 * PI * j as f64 / nt as f64;
///         x[i][j] = r * theta.cos();
///         y[i][j] = r * theta.sin();
///     }
/// }
/// for i in 0..nr {
///     for j in 0..nt {
///         values[i][j] = (i as f64 + j as f64) / (nr + nt) as f64;
///     }
/// }
/// let plot = Pcolormesh::new(x, y, values).title("Polar Mesh");
/// ```
pub struct Pcolormesh {
    /// X coordinates of vertices (nrows+1 x ncols+1).
    x: Vec<Vec<f64>>,
    /// Y coordinates of vertices (nrows+1 x ncols+1).
    y: Vec<Vec<f64>>,
    /// Cell values (nrows x ncols).
    values: Vec<Vec<f64>>,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_colorbar: bool,
    aspect_ratio: AspectRatio,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Pcolormesh {
    /// Create a pseudocolor mesh from vertex coordinates and cell values.
    ///
    /// `x` and `y` are vertex coordinate arrays of shape `(nrows+1) x (ncols+1)`.
    /// `values` is the cell value array of shape `nrows x ncols`.
    pub fn new(x: Vec<Vec<f64>>, y: Vec<Vec<f64>>, values: Vec<Vec<f64>>) -> Self {
        let (vmin, vmax) = value_bounds(&values);
        Self {
            x,
            y,
            values,
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_colorbar: true,
            aspect_ratio: AspectRatio::Auto,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
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

    /// Show or hide the colorbar.
    pub fn show_colorbar(mut self, show: bool) -> Self {
        self.show_colorbar = show;
        self
    }

    /// Set the aspect ratio.
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
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

    /// Add an annotation.
    pub fn annotation(mut self, ann: Annotation) -> Self {
        self.annotations.push(ann);
        self
    }
}

/// Compute the min and max of a 2D value array.
fn value_bounds(values: &[Vec<f64>]) -> (f64, f64) {
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

impl Widget for &Pcolormesh {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let nrows = self.values.len();
        if nrows == 0 {
            return;
        }
        let ncols = self.values[0].len();
        if ncols == 0 {
            return;
        }

        // Validate vertex array dimensions
        if self.x.len() < nrows + 1 || self.y.len() < nrows + 1 {
            return;
        }
        for row in &self.x {
            if row.len() < ncols + 1 {
                return;
            }
        }
        for row in &self.y {
            if row.len() < ncols + 1 {
                return;
            }
        }

        // Compute data bounds from vertex coordinates
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for row in &self.x {
            for &v in row {
                if v.is_finite() {
                    x_min = x_min.min(v);
                    x_max = x_max.max(v);
                }
            }
        }
        for row in &self.y {
            for &v in row {
                if v.is_finite() {
                    y_min = y_min.min(v);
                    y_max = y_max.max(v);
                }
            }
        }

        if !x_min.is_finite() || !y_min.is_finite() {
            return;
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let colorbar_width: u16 = if self.show_colorbar { 10 } else { 0 };

        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .aspect_ratio(self.aspect_ratio.clone())
            .spines(self.spines.clone())
            .colorbar_width(colorbar_width)
            .y_label_width(7)
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, DataBounds { x_lo, x_hi, y_lo, y_hi }) else {
            return;
        };

        // For each cell, compute the screen-space bounding box of its 4 corners
        // and fill pixels within that bounding box that pass the point-in-quad test
        for i in 0..nrows {
            for j in 0..ncols {
                let val = self.values[i][j];
                if !val.is_finite() {
                    continue;
                }

                let t = self.norm.normalize(val);
                let color = self.colormap.color_at(t);

                // 4 corner vertices of the cell in data coordinates
                let corners_data = [
                    (self.x[i][j], self.y[i][j]),
                    (self.x[i][j + 1], self.y[i][j + 1]),
                    (self.x[i + 1][j + 1], self.y[i + 1][j + 1]),
                    (self.x[i + 1][j], self.y[i + 1][j]),
                ];

                // Convert to screen coordinates
                let corners_screen: [(f64, f64); 4] = [
                    (
                        pa.screen_x(corners_data[0].0),
                        pa.screen_y(corners_data[0].1),
                    ),
                    (
                        pa.screen_x(corners_data[1].0),
                        pa.screen_y(corners_data[1].1),
                    ),
                    (
                        pa.screen_x(corners_data[2].0),
                        pa.screen_y(corners_data[2].1),
                    ),
                    (
                        pa.screen_x(corners_data[3].0),
                        pa.screen_y(corners_data[3].1),
                    ),
                ];

                // Bounding box in screen space
                let bb_min_x = corners_screen
                    .iter()
                    .map(|c| c.0)
                    .fold(f64::INFINITY, f64::min)
                    .floor() as i32;
                let bb_max_x = corners_screen
                    .iter()
                    .map(|c| c.0)
                    .fold(f64::NEG_INFINITY, f64::max)
                    .ceil() as i32;
                let bb_min_y = corners_screen
                    .iter()
                    .map(|c| c.1)
                    .fold(f64::INFINITY, f64::min)
                    .floor() as i32;
                let bb_max_y = corners_screen
                    .iter()
                    .map(|c| c.1)
                    .fold(f64::NEG_INFINITY, f64::max)
                    .ceil() as i32;

                for sy in bb_min_y..=bb_max_y {
                    for sx in bb_min_x..=bb_max_x {
                        let ux = sx as u16;
                        let uy = sy as u16;
                        if pa.contains(ux, uy)
                            && point_in_quad(sx as f64, sy as f64, &corners_screen)
                        {
                            buf[(ux, uy)]
                                .set_char('█')
                                .set_style(Style::default().fg(color));
                        }
                    }
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations(&pa, &self.annotations, buf);

        // Draw colorbar
        if self.show_colorbar {
            let (vmin, vmax) = value_bounds(&self.values);
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
