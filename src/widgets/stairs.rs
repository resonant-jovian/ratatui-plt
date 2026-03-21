//! Stairs (step function) plot widget from pre-computed values and edges.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::stairs::{StairsPlot, StairsDataset};
//!
//! let plot = StairsPlot::new()
//!     .dataset(StairsDataset::new(
//!         "Histogram",
//!         vec![0.0, 1.0, 2.0, 3.0, 4.0],
//!         vec![5.0, 12.0, 8.0, 3.0],
//!         Color::Cyan,
//!     ))
//!     .title("Step Function");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::drawing::draw_braille_line;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::spines::Spines;
use crate::theme::Theme;

/// A single stairs dataset with pre-computed edges and values.
#[derive(Clone, Debug)]
pub struct StairsDataset {
    /// Dataset name (for legend).
    pub name: String,
    /// n+1 edge values defining the bin boundaries.
    pub edges: Vec<f64>,
    /// n step values (one per bin).
    pub values: Vec<f64>,
    /// Line color.
    pub color: Color,
}

impl StairsDataset {
    /// Create a new stairs dataset.
    ///
    /// `edges` must have exactly `values.len() + 1` elements.
    pub fn new(name: impl Into<String>, edges: Vec<f64>, values: Vec<f64>, color: Color) -> Self {
        Self {
            name: name.into(),
            edges,
            values,
            color,
        }
    }
}

/// A stairs (step function) plot widget.
///
/// Draws horizontal lines at each value between consecutive edges, connected
/// by vertical lines at edges. Optionally fills down to a baseline.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::stairs::{StairsPlot, StairsDataset};
/// use ratatui::style::Color;
///
/// let plot = StairsPlot::new()
///     .dataset(StairsDataset::new(
///         "Steps",
///         vec![0.0, 1.0, 2.0, 3.0],
///         vec![2.0, 5.0, 1.0],
///         Color::Green,
///     ))
///     .title("Stairs Plot");
/// ```
pub struct StairsPlot {
    datasets: Vec<StairsDataset>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_legend: bool,
    legend_position: LegendPosition,
    baseline: Option<f64>,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for StairsPlot {
    fn default() -> Self {
        Self {
            datasets: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_legend: true,
            legend_position: LegendPosition::TopRight,
            baseline: None,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl StairsPlot {
    /// Create an empty stairs plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a dataset.
    pub fn dataset(mut self, ds: StairsDataset) -> Self {
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

    /// Set a baseline value to fill to. When set, the area between
    /// the step function and the baseline is filled.
    pub fn baseline(mut self, y: f64) -> Self {
        self.baseline = Some(y);
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

impl Widget for &StairsPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.datasets.is_empty() {
            return;
        }

        // Compute data bounds across all datasets
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for ds in &self.datasets {
            for &e in &ds.edges {
                if e.is_finite() {
                    x_min = x_min.min(e);
                    x_max = x_max.max(e);
                }
            }
            for &v in &ds.values {
                if v.is_finite() {
                    y_min = y_min.min(v);
                    y_max = y_max.max(v);
                }
            }
        }

        // Include baseline in y bounds if set
        if let Some(bl) = self.baseline {
            y_min = y_min.min(bl);
            y_max = y_max.max(bl);
        }

        if x_min.is_infinite() {
            x_min = 0.0;
            x_max = 1.0;
        }
        if y_min.is_infinite() {
            y_min = 0.0;
            y_max = 1.0;
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Create and render the plot frame
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, DataBounds { x_lo, x_hi, y_lo, y_hi }) else {
            return;
        };

        // Draw each dataset
        for ds in &self.datasets {
            let n = ds.values.len();
            if n == 0 || ds.edges.len() != n + 1 {
                continue;
            }

            // Draw fill to baseline if set
            if let Some(bl) = self.baseline {
                let sy_base = pa.screen_y(bl);

                for i in 0..n {
                    let sx_left = pa.screen_x(ds.edges[i]);
                    let sx_right = pa.screen_x(ds.edges[i + 1]);
                    let sy_val = pa.screen_y(ds.values[i]);

                    let x_start = sx_left.round() as u16;
                    let x_end = sx_right.round() as u16;
                    let y_top = sy_val.round().min(sy_base.round()) as u16;
                    let y_bot = sy_val.round().max(sy_base.round()) as u16;

                    for x in x_start..x_end {
                        for y in y_top..=y_bot {
                            if pa.contains(x, y) {
                                buf[(x, y)].set_char('░').set_fg(ds.color);
                            }
                        }
                    }
                }
            }

            // Draw the step outline
            for i in 0..n {
                let sx_left = pa.screen_x(ds.edges[i]);
                let sx_right = pa.screen_x(ds.edges[i + 1]);
                let sy = pa.screen_y(ds.values[i]);

                // Horizontal segment at current value
                draw_braille_line(buf, sx_left, sy, sx_right, sy, ds.color, &pa);

                // Vertical segment at the right edge connecting to next value
                if i + 1 < n {
                    let sy_next = pa.screen_y(ds.values[i + 1]);
                    draw_braille_line(buf, sx_right, sy, sx_right, sy_next, ds.color, &pa);
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations(&pa, &self.annotations, buf);

        // Draw legend
        if self.show_legend && !self.datasets.is_empty() {
            let entries: Vec<LegendEntry> = self
                .datasets
                .iter()
                .map(|ds| LegendEntry {
                    name: ds.name.clone(),
                    color: ds.color,
                    marker: Some('━'),
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
