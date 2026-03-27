//! Freehand polygon (lasso) selection overlay widget.
//!
//! Provides a lasso selection tool for interactive data selection on 2D plots.
//! The user builds a polygon path through mouse events (managed externally),
//! and this widget renders the polygon outline using Braille sub-pixel lines.
//!
//! Uses `StatefulWidget` with [`LassoSelectorState`] holding the polygon points.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::lasso_selector::{LassoSelector, LassoSelectorState};
//! use ratatui_plt::prelude::*;
//!
//! let lasso = LassoSelector::new()
//!     .x_axis(Axis::new().bounds(Bounds::Manual(0.0, 10.0)))
//!     .y_axis(Axis::new().bounds(Bounds::Manual(0.0, 10.0)));
//!
//! let mut state = LassoSelectorState::new();
//! state.add_point(1.0, 2.0);
//! state.add_point(3.0, 5.0);
//! state.add_point(2.0, 4.0);
//! state.close();
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::StatefulWidget;

use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame};
use crate::plot_buffer::{PlotBuffer, Z_DATA, Z_FILL, Z_MARKER};
use crate::spines::Spines;
use crate::theme::Theme;

/// Mutable state for the lasso selector, holding the polygon vertices.
///
/// Points are stored in data coordinates. The user is responsible for
/// adding points (e.g., from mouse events) and closing the polygon.
#[derive(Clone, Debug, Default)]
pub struct LassoSelectorState {
    /// Points in the lasso polygon (data coordinates).
    pub points: Vec<(f64, f64)>,
    /// Whether the lasso is currently being drawn.
    pub active: bool,
    /// Whether the lasso is closed (selection complete).
    pub closed: bool,
}

impl LassoSelectorState {
    /// Create a new empty lasso selector state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a point to the polygon in data coordinates.
    pub fn add_point(&mut self, x: f64, y: f64) {
        self.points.push((x, y));
        self.active = true;
    }

    /// Close the polygon to complete the selection.
    pub fn close(&mut self) {
        if self.points.len() >= 3 {
            self.closed = true;
            self.active = false;
        }
    }

    /// Reset the selection, clearing all points.
    pub fn reset(&mut self) {
        self.points.clear();
        self.active = false;
        self.closed = false;
    }

    /// Test whether a point (in data coordinates) is inside the closed polygon.
    ///
    /// Uses the ray-casting algorithm. Returns false if the polygon is not closed
    /// or has fewer than 3 points.
    pub fn contains(&self, px: f64, py: f64) -> bool {
        if !self.closed || self.points.len() < 3 {
            return false;
        }

        let mut inside = false;
        let n = self.points.len();
        let mut j = n - 1;

        for i in 0..n {
            let (xi, yi) = self.points[i];
            let (xj, yj) = self.points[j];

            if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
            j = i;
        }

        inside
    }
}

/// A freehand polygon (lasso) selection overlay widget.
///
/// Renders a polygon outline on top of a plot frame. When the polygon is
/// closed, the interior is lightly filled to indicate the selected region.
///
/// This widget implements `StatefulWidget` with [`LassoSelectorState`].
pub struct LassoSelector {
    x_axis: Axis,
    y_axis: Axis,
    line_color: Option<Color>,
    fill_closed: bool,
    show_vertices: bool,
    theme: Theme,
    spines: Spines,
}

impl Default for LassoSelector {
    fn default() -> Self {
        Self {
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            line_color: None,
            fill_closed: true,
            show_vertices: true,
            theme: Theme::get_default(),
            spines: Spines::default(),
        }
    }
}

impl LassoSelector {
    /// Create a new lasso selector with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the x-axis configuration.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the y-axis configuration.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the polygon line color.
    pub fn line_color(mut self, color: Color) -> Self {
        self.line_color = Some(color);
        self
    }

    /// Whether to fill the interior when the polygon is closed. Default: true.
    pub fn fill_closed(mut self, fill: bool) -> Self {
        self.fill_closed = fill;
        self
    }

    /// Whether to show vertex markers at polygon points. Default: true.
    pub fn show_vertices(mut self, show: bool) -> Self {
        self.show_vertices = show;
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set spine visibility.
    pub fn spines(mut self, spines: Spines) -> Self {
        self.spines = spines;
        self
    }
}

impl StatefulWidget for &LassoSelector {
    type State = LassoSelectorState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if state.points.is_empty() {
            return;
        }

        // Compute data bounds from the polygon points plus some padding
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for &(x, y) in &state.points {
            if x.is_finite() && y.is_finite() {
                x_min = x_min.min(x);
                x_max = x_max.max(x);
                y_min = y_min.min(y);
                y_max = y_max.max(y);
            }
        }

        if x_min.is_infinite() {
            return;
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let mut pb = PlotBuffer::new(area);

        let frame =
            PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme).spines(self.spines.clone());

        let bounds = DataBounds {
            x_lo,
            x_hi,
            y_lo,
            y_hi,
        };

        let Some(pa) = frame.render_to_pb(&mut pb, area, bounds) else {
            return;
        };

        let line_color = self.line_color.unwrap_or(self.theme.accent);

        // If closed and fill_closed, fill interior using scanline
        if state.closed && self.fill_closed && state.points.len() >= 3 {
            let fill_char = self.theme.chars.fill.light;

            // Scanline fill: for each screen row, find intersections with polygon edges
            for sy in pa.y..pa.y + pa.height {
                // Convert screen y to data y
                // screen_y maps y_hi -> pa.y and y_lo -> pa.y+pa.height-1
                let data_y = if pa.height > 1 {
                    y_hi - (sy - pa.y) as f64 * (y_hi - y_lo) / (pa.height - 1) as f64
                } else {
                    (y_lo + y_hi) / 2.0
                };

                // Find x intersections with all polygon edges
                let mut x_intersections: Vec<f64> = Vec::new();
                let n = state.points.len();
                let mut j = n - 1;
                for i in 0..n {
                    let (xi, yi) = state.points[i];
                    let (xj, yj) = state.points[j];

                    if (yi > data_y) != (yj > data_y) {
                        let x_int = (xj - xi) * (data_y - yi) / (yj - yi) + xi;
                        x_intersections.push(x_int);
                    }
                    j = i;
                }

                x_intersections
                    .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                // Fill between pairs of intersections
                let mut k = 0;
                while k + 1 < x_intersections.len() {
                    let sx_start = pa.screen_x(x_intersections[k]).round() as u16;
                    let sx_end = pa.screen_x(x_intersections[k + 1]).round() as u16;

                    let fill_start = sx_start.max(pa.x);
                    let fill_end = (sx_end + 1).min(pa.x + pa.width);

                    for sx in fill_start..fill_end {
                        if pa.contains(sx, sy) {
                            pb.set_char(sx, sy, fill_char, line_color, Z_FILL);
                        }
                    }

                    k += 2;
                }
            }
        }

        // Draw polygon edges using Braille lines
        let n = state.points.len();
        if n >= 2 {
            for i in 0..n - 1 {
                let (x0, y0) = state.points[i];
                let (x1, y1) = state.points[i + 1];

                if x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite() {
                    let sx0 = pa.screen_x(x0);
                    let sy0 = pa.screen_y(y0);
                    let sx1 = pa.screen_x(x1);
                    let sy1 = pa.screen_y(y1);

                    pb.draw_line(sx0, sy0, sx1, sy1, line_color, &pa, Z_DATA);
                }
            }

            // Close the polygon if it is closed
            if state.closed && n >= 3 {
                let (x0, y0) = state.points[n - 1];
                let (x1, y1) = state.points[0];

                if x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite() {
                    let sx0 = pa.screen_x(x0);
                    let sy0 = pa.screen_y(y0);
                    let sx1 = pa.screen_x(x1);
                    let sy1 = pa.screen_y(y1);

                    pb.draw_line(sx0, sy0, sx1, sy1, line_color, &pa, Z_DATA);
                }
            }
        }

        // Draw vertex markers
        if self.show_vertices {
            for &(x, y) in &state.points {
                if x.is_finite() && y.is_finite() {
                    let sx = pa.screen_x(x).round() as u16;
                    let sy = pa.screen_y(y).round() as u16;
                    if pa.contains(sx, sy) {
                        pb.set_char(sx, sy, '+', line_color, Z_MARKER);
                    }
                }
            }
        }

        pb.composite(buf);
    }
}
