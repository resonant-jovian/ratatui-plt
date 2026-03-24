//! Scatter plot widget with color and size mapping.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
#[cfg(feature = "statistics")]
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis};
use crate::colormap::{Colormap, Viridis};
#[cfg(feature = "statistics")]
use crate::drawing::draw_braille_line_pb;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendPosition};
use crate::linked_view::SharedView;
use crate::norm::{LinearNorm, Normalize};
#[cfg(feature = "statistics")]
use crate::plot_buffer::Z_DATA;
use crate::plot_buffer::{PlotBuffer, Z_MARKER};
use crate::series::Series;
use crate::spines::Spines;
use crate::style::MarkerShape;
use crate::theme::Theme;

/// Type of trendline to overlay on a scatter plot.
///
/// Requires the `statistics` feature.
#[cfg(feature = "statistics")]
#[derive(Clone, Debug)]
pub enum TrendlineType {
    /// Ordinary least squares linear regression.
    Linear,
    /// Polynomial regression of the given degree.
    Polynomial(usize),
    /// LOWESS (locally weighted scatterplot smoothing) with given fraction.
    Lowess(f64),
}

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
    shared_view: Option<SharedView>,
    /// Trendline type to overlay.
    #[cfg(feature = "statistics")]
    trendline: Option<TrendlineType>,
    /// Trendline color override.
    #[cfg(feature = "statistics")]
    trendline_color: Option<Color>,
    /// Number of evaluation points for the trendline curve.
    #[cfg(feature = "statistics")]
    trendline_n_points: usize,
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
            shared_view: None,
            #[cfg(feature = "statistics")]
            trendline: None,
            #[cfg(feature = "statistics")]
            trendline_color: None,
            #[cfg(feature = "statistics")]
            trendline_n_points: 100,
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

    /// Link this plot to a shared view state for synchronized bounds.
    pub fn shared_view(mut self, sv: SharedView) -> Self {
        self.shared_view = Some(sv);
        self
    }

    /// Set the trendline type to overlay on the scatter plot.
    ///
    /// Requires the `statistics` feature.
    #[cfg(feature = "statistics")]
    pub fn trendline(mut self, tt: TrendlineType) -> Self {
        self.trendline = Some(tt);
        self
    }

    /// Set the trendline color (defaults to the first series color).
    ///
    /// Requires the `statistics` feature.
    #[cfg(feature = "statistics")]
    pub fn trendline_color(mut self, color: Color) -> Self {
        self.trendline_color = Some(color);
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

        let (mut x_lo, mut x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (mut y_lo, mut y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Apply shared view overrides if linked
        if let Some(ref sv) = self.shared_view {
            let state = sv.borrow();
            if let Some((lo, hi)) = state.x_bounds {
                x_lo = lo;
                x_hi = hi;
            }
            if let Some((lo, hi)) = state.y_bounds {
                y_lo = lo;
                y_hi = hi;
            }
        }

        let mut pb = PlotBuffer::new(area);

        // Create and render the plot frame (title, axes, grid, ticks, labels, spines, ref lines)
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .aspect_ratio(self.aspect_ratio.clone())
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
                    pb.set_char(xi, yi, marker.char(), color, Z_MARKER);
                }
                global_point_idx += 1;
            }
        }

        // Draw trendline (statistics feature)
        #[cfg(feature = "statistics")]
        if let Some(ref ttype) = self.trendline {
            // Collect all (x, y) from all series
            let all_x: Vec<f64> = self
                .series
                .iter()
                .flat_map(|s| s.data.iter().map(|&(x, _)| x))
                .filter(|v| v.is_finite())
                .collect();
            let all_y: Vec<f64> = self
                .series
                .iter()
                .flat_map(|s| s.data.iter().map(|&(_, y)| y))
                .filter(|v| v.is_finite())
                .collect();

            let trend_color = self
                .trendline_color
                .unwrap_or_else(|| self.series.first().map_or(Color::White, |s| s.color));

            // Generate evaluation x values across the plot range
            let n_eval = self.trendline_n_points;
            let eval_xs: Vec<f64> = (0..n_eval)
                .map(|i| x_lo + (x_hi - x_lo) * i as f64 / (n_eval - 1).max(1) as f64)
                .collect();

            let eval_ys: Option<Vec<f64>> = match ttype {
                TrendlineType::Linear => crate::statistics::linear_regression(&all_x, &all_y)
                    .map(|fit| eval_xs.iter().map(|&x| fit.eval(x)).collect()),
                TrendlineType::Polynomial(degree) => {
                    crate::statistics::poly_fit(&all_x, &all_y, *degree)
                        .map(|fit| eval_xs.iter().map(|&x| fit.eval(x)).collect())
                }
                TrendlineType::Lowess(frac) => {
                    crate::statistics::lowess(&all_x, &all_y, *frac).map(|result| {
                        // Interpolate LOWESS result onto eval_xs
                        eval_xs
                            .iter()
                            .map(|&ex| interpolate_lowess(&result.x, &result.y, ex))
                            .collect()
                    })
                }
            };

            if let Some(ys) = eval_ys {
                // Draw as connected braille line segments
                for i in 0..eval_xs.len() - 1 {
                    let sx0 = pa.screen_x(eval_xs[i]);
                    let sy0 = pa.screen_y(ys[i]);
                    let sx1 = pa.screen_x(eval_xs[i + 1]);
                    let sy1 = pa.screen_y(ys[i + 1]);
                    draw_braille_line_pb(&mut pb, sx0, sy0, sx1, sy1, trend_color, &pa, Z_DATA);
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite to buffer before legend
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Draw legend (directly to buf, after composite)
        if self.show_legend && !self.series.is_empty() && self.color_values.is_none() {
            let legend = Legend::from_series(&self.series)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(pa.x, pa.y, pa.width, pa.height);
            (&legend).render(legend_area, buf);
        }
    }
}

/// Linear interpolation of a LOWESS result at a given x value.
#[cfg(feature = "statistics")]
fn interpolate_lowess(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    if xs.len() == 1 {
        return ys[0];
    }
    // Clamp to range
    if x <= xs[0] {
        return ys[0];
    }
    let last = xs.len() - 1;
    if x >= xs[last] {
        return ys[last];
    }
    // Binary search for bracket
    let mut lo = 0;
    let mut hi = last;
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if xs[mid] <= x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let dx = xs[hi] - xs[lo];
    if dx.abs() < f64::EPSILON {
        return ys[lo];
    }
    let t = (x - xs[lo]) / dx;
    ys[lo] * (1.0 - t) + ys[hi] * t
}
