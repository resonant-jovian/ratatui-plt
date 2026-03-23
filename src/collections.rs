//! Primitive shape collections for efficient batch rendering.
//!
//! Similar to matplotlib's `LineCollection` and `PathCollection`, these allow
//! rendering many line segments or paths in a single pass with per-element styling.

use ratatui::buffer::Buffer;
use ratatui::style::Color;

use crate::drawing::{draw_braille_line, draw_braille_line_pb};
use crate::frame::PlotArea;
use crate::plot_buffer::PlotBuffer;
use crate::style::LineStyle;

/// A single line segment with color.
pub type LineSegment = ((f64, f64), (f64, f64), Color);

/// A single path with color and closed flag.
pub type PathEntry = (Vec<(f64, f64)>, Color, bool);

/// A collection of line segments, each with its own color.
///
/// Useful for rendering many independent line segments efficiently (e.g. colored
/// by a scalar value) without creating a full `Series` for each.
///
/// # Example
///
/// ```
/// use ratatui_plt::collections::LineCollection;
/// use ratatui::style::Color;
///
/// let lc = LineCollection::new()
///     .segment((0.0, 0.0), (1.0, 1.0), Color::Red)
///     .segment((1.0, 0.0), (0.0, 1.0), Color::Blue);
/// ```
#[derive(Clone, Debug)]
pub struct LineCollection {
    /// Line segments: ((x0, y0), (x1, y1), color).
    pub segments: Vec<LineSegment>,
    /// Default line style for all segments.
    pub line_style: LineStyle,
}

impl Default for LineCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl LineCollection {
    /// Create an empty line collection.
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            line_style: LineStyle::default(),
        }
    }

    /// Add a line segment with a specific color.
    pub fn segment(mut self, from: (f64, f64), to: (f64, f64), color: Color) -> Self {
        self.segments.push((from, to, color));
        self
    }

    /// Set the line style for all segments.
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.line_style = style;
        self
    }

    /// Render the line collection onto a buffer using Braille sub-pixel line drawing.
    pub fn render(&self, pa: &PlotArea, buf: &mut Buffer) {
        for &((x0, y0), (x1, y1), color) in &self.segments {
            let sx0 = pa.screen_x(x0);
            let sy0 = pa.screen_y(y0);
            let sx1 = pa.screen_x(x1);
            let sy1 = pa.screen_y(y1);
            draw_braille_line(buf, sx0, sy0, sx1, sy1, color, pa);
        }
    }

    /// Render the line collection into a [`PlotBuffer`] using Braille sub-pixel line drawing.
    pub fn render_to_pb(&self, pa: &PlotArea, pb: &mut PlotBuffer) {
        for &((x0, y0), (x1, y1), color) in &self.segments {
            let sx0 = pa.screen_x(x0);
            let sy0 = pa.screen_y(y0);
            let sx1 = pa.screen_x(x1);
            let sy1 = pa.screen_y(y1);
            draw_braille_line_pb(pb, sx0, sy0, sx1, sy1, color, pa);
        }
    }
}

/// A collection of paths (polylines or closed polygons) with per-path colors.
///
/// # Example
///
/// ```
/// use ratatui_plt::collections::PathCollection;
/// use ratatui::style::Color;
///
/// let pc = PathCollection::new()
///     .path(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)], Color::Green, true);
/// ```
#[derive(Clone, Debug)]
pub struct PathCollection {
    /// Paths: (vertices, color, closed).
    pub paths: Vec<PathEntry>,
}

impl Default for PathCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl PathCollection {
    /// Create an empty path collection.
    pub fn new() -> Self {
        Self { paths: Vec::new() }
    }

    /// Add a path with a color. If `closed`, the last vertex connects back to the first.
    pub fn path(mut self, vertices: Vec<(f64, f64)>, color: Color, closed: bool) -> Self {
        self.paths.push((vertices, color, closed));
        self
    }

    /// Render all paths onto a buffer using Braille sub-pixel line drawing.
    pub fn render(&self, pa: &PlotArea, buf: &mut Buffer) {
        for (vertices, color, closed) in &self.paths {
            if vertices.len() < 2 {
                continue;
            }
            for i in 0..vertices.len() - 1 {
                let (x0, y0) = vertices[i];
                let (x1, y1) = vertices[i + 1];
                let sx0 = pa.screen_x(x0);
                let sy0 = pa.screen_y(y0);
                let sx1 = pa.screen_x(x1);
                let sy1 = pa.screen_y(y1);
                draw_braille_line(buf, sx0, sy0, sx1, sy1, *color, pa);
            }
            if *closed && vertices.len() > 2 {
                let (x0, y0) = vertices[vertices.len() - 1];
                let (x1, y1) = vertices[0];
                let sx0 = pa.screen_x(x0);
                let sy0 = pa.screen_y(y0);
                let sx1 = pa.screen_x(x1);
                let sy1 = pa.screen_y(y1);
                draw_braille_line(buf, sx0, sy0, sx1, sy1, *color, pa);
            }
        }
    }

    /// Render all paths into a [`PlotBuffer`] using Braille sub-pixel line drawing.
    pub fn render_to_pb(&self, pa: &PlotArea, pb: &mut PlotBuffer) {
        for (vertices, color, closed) in &self.paths {
            if vertices.len() < 2 {
                continue;
            }
            for i in 0..vertices.len() - 1 {
                let (x0, y0) = vertices[i];
                let (x1, y1) = vertices[i + 1];
                let sx0 = pa.screen_x(x0);
                let sy0 = pa.screen_y(y0);
                let sx1 = pa.screen_x(x1);
                let sy1 = pa.screen_y(y1);
                draw_braille_line_pb(pb, sx0, sy0, sx1, sy1, *color, pa);
            }
            if *closed && vertices.len() > 2 {
                let (x0, y0) = vertices[vertices.len() - 1];
                let (x1, y1) = vertices[0];
                let sx0 = pa.screen_x(x0);
                let sy0 = pa.screen_y(y0);
                let sx1 = pa.screen_x(x1);
                let sy1 = pa.screen_y(y1);
                draw_braille_line_pb(pb, sx0, sy0, sx1, sy1, *color, pa);
            }
        }
    }
}
