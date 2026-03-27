//! Empirical Cumulative Distribution Function (ECDF) plot widget.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::ecdf::{EcdfPlot, EcdfDataset};
//!
//! let plot = EcdfPlot::new()
//!     .dataset(EcdfDataset::new("Sample A", vec![1.0, 2.0, 3.0, 4.0, 5.0], Color::Cyan))
//!     .dataset(EcdfDataset::new("Sample B", vec![2.0, 3.0, 4.0, 5.0, 8.0], Color::Yellow))
//!     .title("ECDF Comparison");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::plot_buffer::{PlotBuffer, Z_DATA};
use crate::spines::Spines;
use crate::theme::Theme;

/// A single dataset for the ECDF plot.
#[derive(Clone, Debug)]
pub struct EcdfDataset {
    /// Dataset name (for legend).
    pub name: String,
    /// Raw data values.
    pub data: Vec<f64>,
    /// Line color.
    pub color: Color,
}

impl EcdfDataset {
    /// Create a new ECDF dataset.
    pub fn new(name: impl Into<String>, data: Vec<f64>, color: Color) -> Self {
        Self {
            name: name.into(),
            data,
            color,
        }
    }
}

/// An Empirical Cumulative Distribution Function plot widget.
///
/// Draws one or more ECDFs as step functions. Each dataset is sorted and
/// the ECDF is computed as `F(x) = (number of values <= x) / n`.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::ecdf::{EcdfPlot, EcdfDataset};
/// use ratatui::style::Color;
///
/// let plot = EcdfPlot::new()
///     .dataset(EcdfDataset::new("Normal", vec![1.0, 2.0, 3.0], Color::Cyan))
///     .title("ECDF");
/// ```
pub struct EcdfPlot {
    datasets: Vec<EcdfDataset>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_legend: bool,
    legend_position: LegendPosition,
    complementary: bool,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for EcdfPlot {
    fn default() -> Self {
        Self {
            datasets: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_legend: true,
            legend_position: LegendPosition::TopLeft,
            complementary: false,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl EcdfPlot {
    /// Create an empty ECDF plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a dataset.
    pub fn dataset(mut self, ds: EcdfDataset) -> Self {
        self.datasets.push(ds);
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

    /// Set the plot title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Show or hide the legend.
    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    /// Set the legend position.
    pub fn legend_position(mut self, pos: LegendPosition) -> Self {
        self.legend_position = pos;
        self
    }

    /// If true, plot the complementary ECDF (survival function): 1 - ECDF(x).
    pub fn complementary(mut self, c: bool) -> Self {
        self.complementary = c;
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

    /// Compute the ECDF step points for a dataset.
    ///
    /// Returns a vec of (x, y) pairs representing the step function.
    /// The step is drawn Post-style: value jumps at the data point.
    fn compute_ecdf(data: &[f64], complementary: bool) -> Vec<(f64, f64)> {
        let mut sorted: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = sorted.len();
        if n == 0 {
            return Vec::new();
        }

        let mut points = Vec::with_capacity(n + 1);

        // Initial point at the leftmost value with y = 0 (or 1 for complementary)
        let start_y = if complementary { 1.0 } else { 0.0 };
        points.push((sorted[0], start_y));

        for (i, &x) in sorted.iter().enumerate() {
            let ecdf_val = (i + 1) as f64 / n as f64;
            let y = if complementary {
                1.0 - ecdf_val
            } else {
                ecdf_val
            };
            points.push((x, y));
        }

        points
    }
}

impl Widget for &EcdfPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.datasets.is_empty() {
            return;
        }

        // Compute data bounds across all datasets
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        for ds in &self.datasets {
            for &v in &ds.data {
                if v.is_finite() {
                    x_min = x_min.min(v);
                    x_max = x_max.max(v);
                }
            }
        }
        if x_min.is_infinite() {
            x_min = 0.0;
            x_max = 1.0;
        }

        // Y bounds are always [0, 1] for ECDF
        let data_y_min = 0.0;
        let data_y_max = 1.0;

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(data_y_min, data_y_max);

        let mut pb = PlotBuffer::new(area);

        // Create and render the plot frame
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

        // Draw each dataset as a step function (Post-style)
        for (si, ds) in self.datasets.iter().enumerate() {
            let points = EcdfPlot::compute_ecdf(&ds.data, self.complementary);
            if points.len() < 2 {
                continue;
            }

            // Draw step function: horizontal at current y, then vertical to next y
            for i in 0..points.len() - 1 {
                let (x0, y0) = points[i];
                let (x1, _y1) = points[i + 1];
                let y1 = points[i + 1].1;

                let sx0 = pa.screen_x(x0);
                let sy0 = pa.screen_y(y0);
                let sx1 = pa.screen_x(x1);
                let sy1 = pa.screen_y(y1);

                // Horizontal segment at y0 from x0 to x1
                pb.draw_line(sx0, sy0, sx1, sy0, ds.color, &pa, Z_DATA + si as u8);

                // Vertical segment at x1 from y0 to y1
                pb.draw_line(sx1, sy0, sx1, sy1, ds.color, &pa, Z_DATA + si as u8);
            }

            // Extend the last step to the right edge of the plot
            if let Some(&(last_x, last_y)) = points.last() {
                let sx_last = pa.screen_x(last_x);
                let sx_end = pa.screen_x(x_hi);
                let sy = pa.screen_y(last_y);

                pb.draw_line(sx_last, sy, sx_end, sy, ds.color, &pa, Z_DATA + si as u8);
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite to buffer before legend
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Draw legend (directly to buf, after composite)
        if self.show_legend && !self.datasets.is_empty() {
            let entries: Vec<LegendEntry> = self
                .datasets
                .iter()
                .map(|ds| LegendEntry {
                    name: ds.name.clone(),
                    color: ds.color,
                    marker: Some(self.theme.chars.marker.legend_line),
                })
                .collect();
            let legend = Legend::new(entries)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(pa.x, pa.y, pa.width, pa.height);
            (&legend).render(legend_area, buf);
        }
    }
}
