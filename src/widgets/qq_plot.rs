//! Quantile-quantile (Q-Q) plot widget for distribution comparison.
//!
//! Compares sample data against a theoretical distribution by plotting
//! theoretical quantiles on the x-axis against sample quantiles on the y-axis.
//! Points falling on the diagonal indicate a good fit.
//!
//! Requires the `statistics` feature.
//!
//! # Example
//!
//! ```ignore
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::qq_plot::QQPlot;
//!
//! let data = vec![0.5, 1.2, -0.3, 0.8, 2.1, -1.0, 0.1, 1.5, -0.5, 0.9];
//! let plot = QQPlot::new(data)
//!     .distribution(QQDistribution::Normal)
//!     .title("Q-Q Plot")
//!     .show_reference_line(true);
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::drawing::draw_braille_line_pb;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBuffer, Z_DATA, Z_MARKER};
use crate::spines::Spines;
use crate::statistics::{QQDistribution, qq_points};
use crate::style::MarkerShape;
use crate::theme::Theme;

/// A quantile-quantile (Q-Q) plot widget.
///
/// Compares sample data against a theoretical distribution. Theoretical
/// quantiles are plotted on the x-axis, sample quantiles on the y-axis.
/// A reference line passing through the first and third quartiles of
/// both distributions is optionally drawn.
pub struct QQPlot {
    /// Raw sample data.
    data: Vec<f64>,
    /// Theoretical distribution to compare against.
    distribution: QQDistribution,
    /// X-axis configuration.
    x_axis: Axis,
    /// Y-axis configuration.
    y_axis: Axis,
    /// Chart title.
    title: Option<String>,
    /// Whether to show the quartile-matched reference line (default: true).
    show_reference_line: bool,
    /// Marker shape for QQ points.
    marker: MarkerShape,
    /// Optional color override for the points.
    color: Option<Color>,
    /// Visual theme.
    theme: Theme,
    /// Spine visibility control.
    spines: Spines,
    /// Reference lines drawn across the plot area.
    reference_lines: Vec<ReferenceLine>,
    /// Annotations drawn within the plot area.
    annotations: Vec<Annotation>,
}

impl QQPlot {
    /// Create a new Q-Q plot from sample data.
    ///
    /// Defaults to comparing against a standard normal distribution
    /// with the reference line enabled.
    pub fn new(data: Vec<f64>) -> Self {
        Self {
            data,
            distribution: QQDistribution::default(),
            x_axis: Axis::new().label("Theoretical Quantiles"),
            y_axis: Axis::new().label("Sample Quantiles"),
            title: None,
            show_reference_line: true,
            marker: MarkerShape::Circle,
            color: None,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }

    /// Set the theoretical distribution for comparison.
    pub fn distribution(mut self, dist: QQDistribution) -> Self {
        self.distribution = dist;
        self
    }

    /// Set the X-axis configuration.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y-axis configuration.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Show or hide the quartile-matched reference line (default: true).
    ///
    /// The reference line passes through the first and third quartiles
    /// of both the theoretical and sample distributions.
    pub fn show_reference_line(mut self, show: bool) -> Self {
        self.show_reference_line = show;
        self
    }

    /// Set the marker shape for QQ points.
    pub fn marker(mut self, marker: MarkerShape) -> Self {
        self.marker = marker;
        self
    }

    /// Set the color for points and reference line.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
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

impl Widget for &QQPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.data.is_empty() {
            return;
        }

        // Compute QQ points: Vec<(theoretical, sample)>
        let points = qq_points(&self.data, &self.distribution);
        if points.is_empty() {
            return;
        }

        // Determine data bounds from QQ points
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for &(tx, sy) in &points {
            if tx.is_finite() {
                if tx < x_min {
                    x_min = tx;
                }
                if tx > x_max {
                    x_max = tx;
                }
            }
            if sy.is_finite() {
                if sy < y_min {
                    y_min = sy;
                }
                if sy > y_max {
                    y_max = sy;
                }
            }
        }

        if !x_min.is_finite() || !x_max.is_finite() {
            x_min = -3.0;
            x_max = 3.0;
        }
        if !y_min.is_finite() || !y_max.is_finite() {
            y_min = -3.0;
            y_max = 3.0;
        }

        // Add a small margin so points are not on the edge
        let x_margin = (x_max - x_min).max(1.0) * 0.05;
        let y_margin = (y_max - y_min).max(1.0) * 0.05;
        x_min -= x_margin;
        x_max += x_margin;
        y_min -= y_margin;
        y_max += y_margin;

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let mut pb = PlotBuffer::new(area);

        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let bounds = DataBounds {
            x_lo,
            x_hi,
            y_lo,
            y_hi,
        };

        let Some(pa) = frame.render_to_pb(&mut pb, area, bounds) else {
            return;
        };

        let point_color = self.color.unwrap_or(self.theme.primary);

        // Draw quartile-matched reference line if enabled
        if self.show_reference_line && points.len() >= 4 {
            // Compute Q1 and Q3 from both theoretical and sample quantiles
            let n = points.len();
            let q1_idx = n / 4;
            let q3_idx = (3 * n) / 4;

            // Points are already sorted by theoretical quantile
            let (t_q1, s_q1) = points[q1_idx];
            let (t_q3, s_q3) = points[q3_idx];

            // Compute the line passing through (t_q1, s_q1) and (t_q3, s_q3)
            let dt = t_q3 - t_q1;
            if dt.abs() > f64::EPSILON {
                let slope = (s_q3 - s_q1) / dt;
                let intercept = s_q1 - slope * t_q1;

                // Evaluate the reference line at the plot bounds
                let ref_y_at_xlo = slope * x_lo + intercept;
                let ref_y_at_xhi = slope * x_hi + intercept;

                let ref_color = self
                    .color
                    .map(dim_color)
                    .unwrap_or(self.theme.grid_color);

                // Draw the reference line across the full x range
                let n_segs: usize = 100;
                for i in 0..n_segs {
                    let t0 = i as f64 / n_segs as f64;
                    let t1 = (i + 1) as f64 / n_segs as f64;

                    let rx0 = x_lo + t0 * (x_hi - x_lo);
                    let rx1 = x_lo + t1 * (x_hi - x_lo);
                    let ry0 = ref_y_at_xlo + t0 * (ref_y_at_xhi - ref_y_at_xlo);
                    let ry1 = ref_y_at_xlo + t1 * (ref_y_at_xhi - ref_y_at_xlo);

                    let sx0 = pa.screen_x(rx0);
                    let sy0 = pa.screen_y(ry0);
                    let sx1 = pa.screen_x(rx1);
                    let sy1 = pa.screen_y(ry1);

                    draw_braille_line_pb(&mut pb, sx0, sy0, sx1, sy1, ref_color, &pa, Z_DATA);
                }
            }
        }

        // Draw scatter markers at each QQ point
        for &(tx, sy) in &points {
            if !tx.is_finite() || !sy.is_finite() {
                continue;
            }
            let sx = pa.screen_x(tx);
            let screen_y = pa.screen_y(sy);
            let xi = sx.round() as u16;
            let yi = screen_y.round() as u16;

            if pa.contains(xi, yi) {
                pb.set_char(xi, yi, self.marker.char(), point_color, Z_MARKER);
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite to terminal buffer
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);
    }
}

/// Produce a dimmed version of a color for the reference line.
///
/// Blends the given color toward gray to make it visually subordinate
/// to the data markers.
fn dim_color(c: Color) -> Color {
    match c {
        Color::Rgb(r, g, b) => {
            let r = (r as u16 + 128) / 2;
            let g = (g as u16 + 128) / 2;
            let b = (b as u16 + 128) / 2;
            Color::Rgb(r as u8, g as u8, b as u8)
        }
        _ => Color::Gray,
    }
}
