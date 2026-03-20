//! Triangle mesh edge plot widget.
//!
//! Renders the edges of a triangulation as lines using Bresenham's algorithm.
//! Not feature-gated — works with explicit triangulations.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::frame::{PlotFrame, ReferenceLine};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::triangulation::Triangulation;

/// A triangle mesh edge plot widget.
///
/// Renders the edges of a [`Triangulation`] as lines.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::triangulation::Triangulation;
/// use ratatui_plt::widgets::triplot::TriPlot;
///
/// let tri = Triangulation::from_explicit(
///     vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0), (1.5, 1.0)],
///     vec![(0, 1, 2), (1, 3, 2)],
/// );
/// let plot = TriPlot::new(tri)
///     .title("Triangle Mesh")
///     .edge_color(Color::Cyan);
/// ```
pub struct TriPlot {
    triangulation: Triangulation,
    edge_color: Color,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
}

impl TriPlot {
    /// Create a triangle plot from a triangulation.
    pub fn new(triangulation: Triangulation) -> Self {
        Self {
            triangulation,
            edge_color: Color::White,
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
        }
    }

    /// Set the edge color.
    pub fn edge_color(mut self, color: Color) -> Self {
        self.edge_color = color;
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

impl Widget for &TriPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.triangulation.vertices.is_empty() || self.triangulation.triangles.is_empty() {
            return;
        }

        let (x_min, x_max, y_min, y_max) = self.triangulation.bounds();
        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, x_lo, x_hi, y_lo, y_hi) else {
            return;
        };

        // Draw unique edges
        let edges = self.triangulation.edges();
        for (i, j) in edges {
            let (x0, y0) = self.triangulation.vertices[i];
            let (x1, y1) = self.triangulation.vertices[j];

            let sx0 = pa.screen_x(x0);
            let sy0 = pa.screen_y(y0);
            let sx1 = pa.screen_x(x1);
            let sy1 = pa.screen_y(y1);

            draw_bresenham_line(buf, sx0, sy0, sx1, sy1, self.edge_color, &pa);
        }
    }
}

/// Draw a line between two screen-space points using Bresenham's algorithm.
fn draw_bresenham_line(
    buf: &mut Buffer,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
    pa: &crate::frame::PlotArea,
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
