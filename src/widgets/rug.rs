//! Rug plot widget for marginal tick marks along an axis edge.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::rug::{RugPlot, RugDataset, RugSide};
//!
//! let plot = RugPlot::new()
//!     .dataset(RugDataset::new("Observations", vec![1.0, 2.5, 3.0, 4.2], Color::Cyan))
//!     .side(RugSide::Bottom)
//!     .title("Rug Plot");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBuffer, Z_DATA};
use crate::spines::Spines;
use crate::theme::Theme;

/// Which edge of the plot area the rug ticks are drawn on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum RugSide {
    /// Bottom edge (ticks extend upward).
    #[default]
    Bottom,
    /// Top edge (ticks extend downward).
    Top,
    /// Left edge (ticks extend rightward).
    Left,
    /// Right edge (ticks extend leftward).
    Right,
}

/// A single rug dataset with observation values.
#[derive(Clone, Debug)]
pub struct RugDataset {
    /// Dataset name.
    pub name: String,
    /// Raw data values (positions along the axis).
    pub data: Vec<f64>,
    /// Tick color.
    pub color: Color,
}

impl RugDataset {
    /// Create a new rug dataset.
    pub fn new(name: impl Into<String>, data: Vec<f64>, color: Color) -> Self {
        Self {
            name: name.into(),
            data,
            color,
        }
    }
}

/// A rug plot widget showing marginal tick marks along an axis edge.
///
/// Draws short tick marks at each data value position, useful for showing
/// the distribution of individual observations.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::rug::{RugPlot, RugDataset, RugSide};
/// use ratatui::style::Color;
///
/// let plot = RugPlot::new()
///     .dataset(RugDataset::new("Data", vec![1.0, 2.0, 3.0], Color::Red))
///     .side(RugSide::Bottom)
///     .height(2);
/// ```
pub struct RugPlot {
    datasets: Vec<RugDataset>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    side: RugSide,
    height: u16,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for RugPlot {
    fn default() -> Self {
        Self {
            datasets: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            side: RugSide::Bottom,
            height: 1,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl RugPlot {
    /// Create an empty rug plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a dataset.
    pub fn dataset(mut self, ds: RugDataset) -> Self {
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

    /// Set which axis edge to draw ticks on.
    pub fn side(mut self, side: RugSide) -> Self {
        self.side = side;
        self
    }

    /// Set the height of rug ticks in characters (default 1).
    pub fn height(mut self, h: u16) -> Self {
        self.height = h.max(1);
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

impl Widget for &RugPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.datasets.is_empty() {
            return;
        }

        // Compute data bounds based on the side direction
        let is_horizontal = matches!(self.side, RugSide::Bottom | RugSide::Top);

        let mut data_min = f64::INFINITY;
        let mut data_max = f64::NEG_INFINITY;
        for ds in &self.datasets {
            for &v in &ds.data {
                if v.is_finite() {
                    data_min = data_min.min(v);
                    data_max = data_max.max(v);
                }
            }
        }
        if data_min.is_infinite() {
            data_min = 0.0;
            data_max = 1.0;
        }

        // For horizontal rug, data maps to x; for vertical rug, data maps to y.
        // The other axis just provides a small region for the ticks.
        let (x_lo, x_hi, y_lo, y_hi) = if is_horizontal {
            let (xl, xh) = self.x_axis.resolve_bounds(data_min, data_max);
            let (yl, yh) = self.y_axis.resolve_bounds(0.0, 1.0);
            (xl, xh, yl, yh)
        } else {
            let (xl, xh) = self.x_axis.resolve_bounds(0.0, 1.0);
            let (yl, yh) = self.y_axis.resolve_bounds(data_min, data_max);
            (xl, xh, yl, yh)
        };

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

        let tick_height = self.height.min(pa.height).min(pa.width);

        // Draw rug ticks for each dataset
        for ds in &self.datasets {
            for &v in &ds.data {
                if !v.is_finite() {
                    continue;
                }

                match self.side {
                    RugSide::Bottom => {
                        let sx = pa.screen_x(v).round() as u16;
                        for dy in 0..tick_height {
                            let y = (pa.y + pa.height).saturating_sub(1 + dy);
                            if pa.contains(sx, y) {
                                pb.set_char(sx, y, '│', ds.color, Z_DATA);
                            }
                        }
                    }
                    RugSide::Top => {
                        let sx = pa.screen_x(v).round() as u16;
                        for dy in 0..tick_height {
                            let y = pa.y + dy;
                            if pa.contains(sx, y) {
                                pb.set_char(sx, y, '│', ds.color, Z_DATA);
                            }
                        }
                    }
                    RugSide::Left => {
                        let sy = pa.screen_y(v).round() as u16;
                        for dx in 0..tick_height {
                            let x = pa.x + dx;
                            if pa.contains(x, sy) {
                                pb.set_char(x, sy, '─', ds.color, Z_DATA);
                            }
                        }
                    }
                    RugSide::Right => {
                        let sy = pa.screen_y(v).round() as u16;
                        for dx in 0..tick_height {
                            let x = (pa.x + pa.width).saturating_sub(1 + dx);
                            if pa.contains(x, sy) {
                                pb.set_char(x, sy, '─', ds.color, Z_DATA);
                            }
                        }
                    }
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite to buffer
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);
    }
}
