//! Scatter plot widget with color and size mapping.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis};
use crate::colormap::{Colormap, Viridis};
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendPosition};
use crate::norm::{LinearNorm, Normalize};
use crate::series::Series;
use crate::spines::Spines;
use crate::style::MarkerShape;
use crate::theme::Theme;

/// A 2D scatter plot widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let plot = ScatterPlot::new()
///     .series(Series::new("data").data(vec![(1.0, 2.0), (3.0, 4.0)]).marker(MarkerShape::Circle));
/// ```
pub struct ScatterPlot {
    series: Vec<Series>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    aspect_ratio: AspectRatio,
    show_legend: bool,
    legend_position: LegendPosition,
    /// Optional per-point color values (for single-series color mapping).
    color_values: Option<Vec<f64>>,
    /// Colormap for color-mapped points.
    colormap: Box<dyn Colormap>,
    /// Normalizer for color values.
    color_norm: Box<dyn Normalize>,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for ScatterPlot {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            aspect_ratio: AspectRatio::Auto,
            show_legend: true,
            legend_position: LegendPosition::TopRight,
            color_values: None,
            colormap: Box::new(Viridis),
            color_norm: Box::new(LinearNorm::new(0.0, 1.0)),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl ScatterPlot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn series(mut self, s: Series) -> Self {
        self.series.push(s);
        self
    }

    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
        self
    }

    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    pub fn legend_position(mut self, pos: LegendPosition) -> Self {
        self.legend_position = pos;
        self
    }

    /// Set per-point color values for color mapping.
    pub fn color_values(mut self, values: Vec<f64>) -> Self {
        let vmin = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let vmax = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        self.color_norm = Box::new(LinearNorm::new(vmin, vmax));
        self.color_values = Some(values);
        self
    }

    /// Set the colormap for color-mapped points.
    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    /// Set the normalization for color values.
    pub fn color_norm(mut self, norm: impl Normalize + 'static) -> Self {
        self.color_norm = Box::new(norm);
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

impl Widget for &ScatterPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Compute data bounds
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for s in &self.series {
            if let Some((lo, hi)) = s.x_bounds() {
                x_min = x_min.min(lo);
                x_max = x_max.max(hi);
            }
            if let Some((lo, hi)) = s.y_bounds() {
                y_min = y_min.min(lo);
                y_max = y_max.max(hi);
            }
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

        // Create and render the plot frame (title, axes, grid, ticks, labels, spines, ref lines)
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .aspect_ratio(self.aspect_ratio.clone())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(
            area,
            buf,
            DataBounds {
                x_lo,
                x_hi,
                y_lo,
                y_hi,
            },
        ) else {
            return;
        };

        // Draw scatter points (skip NaN)
        let mut global_point_idx = 0usize;
        for s in &self.series {
            let marker = s.marker.unwrap_or(MarkerShape::Dot);
            for &(x, y) in &s.data {
                if !x.is_finite() || !y.is_finite() {
                    global_point_idx += 1;
                    continue;
                }
                let sx = pa.screen_x(x);
                let sy = pa.screen_y(y);
                let xi = sx.round() as u16;
                let yi = sy.round() as u16;

                if pa.contains(xi, yi) {
                    let color = if let Some(ref cv) = self.color_values {
                        if global_point_idx < cv.len() {
                            let t = self.color_norm.normalize(cv[global_point_idx]);
                            self.colormap.color_at(t)
                        } else {
                            s.color
                        }
                    } else {
                        s.color
                    };
                    buf[(xi, yi)].set_char(marker.char()).set_fg(color);
                }
                global_point_idx += 1;
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations(&pa, &self.annotations, buf);

        // Draw legend
        if self.show_legend && !self.series.is_empty() && self.color_values.is_none() {
            let legend = Legend::from_series(&self.series)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(pa.x, pa.y, pa.width, pa.height);
            (&legend).render(legend_area, buf);
        }
    }
}
