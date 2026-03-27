//! Scatter plot widget with color and size mapping.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
#[cfg(feature = "statistics")]
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis};
use crate::colormap::{Colormap, Viridis};
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendPosition};
use crate::linked_view::SharedView;
use crate::norm::{LinearNorm, Normalize};
#[cfg(feature = "statistics")]
use crate::plot_buffer::Z_DATA;
use crate::plot_buffer::{PlotBackend, Z_MARKER, create_backend};
use crate::series::Series;
use crate::spines::Spines;
use crate::style::MarkerShape;
use crate::theme::Theme;

use super::joint_plot::{MarginalConfig, MarginalType, render_marginal_right, render_marginal_top};

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
#[allow(dead_code)]
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
    /// Optional per-point size values for bubble mode.
    size_values: Option<Vec<f64>>,
    /// Min and max marker size in characters for bubble mode.
    size_range: (f64, f64),
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
    shared_view: Option<SharedView>,
    /// Top marginal distribution type (x-axis).
    marginal_x: MarginalType,
    /// Right marginal distribution type (y-axis).
    marginal_y: MarginalType,
    /// Fraction of area used for marginal panels (default 0.2).
    marginal_ratio: f64,
    /// Number of histogram bins for marginal panels (default 20).
    marginal_bins: usize,
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
            size_values: None,
            size_range: (1.0, 5.0),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
            shared_view: None,
            marginal_x: MarginalType::None,
            marginal_y: MarginalType::None,
            marginal_ratio: 0.2,
            marginal_bins: 20,
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

    /// Set per-point size values for bubble mode.
    ///
    /// Each value maps to a marker radius between `size_range.0` and `size_range.1`.
    /// In the terminal, larger sizes fill adjacent cells around the marker position.
    pub fn size_values(mut self, values: Vec<f64>) -> Self {
        self.size_values = Some(values);
        self
    }

    /// Set the min/max marker size in characters for bubble mode (default: 1.0–5.0).
    pub fn size_range(mut self, min: f64, max: f64) -> Self {
        self.size_range = (min, max);
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

    /// Set the top marginal distribution type (x-axis).
    pub fn marginal_x(mut self, mt: MarginalType) -> Self {
        self.marginal_x = mt;
        self
    }

    /// Set the right marginal distribution type (y-axis).
    pub fn marginal_y(mut self, mt: MarginalType) -> Self {
        self.marginal_y = mt;
        self
    }

    /// Set the fraction of area used for marginals (default 0.2).
    pub fn marginal_ratio(mut self, ratio: f64) -> Self {
        self.marginal_ratio = ratio;
        self
    }

    /// Set the number of histogram bins for marginals
    /// (default 20).
    pub fn marginal_bins(mut self, bins: usize) -> Self {
        self.marginal_bins = bins;
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
        if area.width < 6 || area.height < 6 {
            return;
        }

        let has_top = self.marginal_x != MarginalType::None;
        let has_right = self.marginal_y != MarginalType::None;

        // Compute marginal sizes
        let top_height = if has_top {
            (area.height as f64 * self.marginal_ratio).round() as u16
        } else {
            0
        };
        let right_width = if has_right {
            (area.width as f64 * self.marginal_ratio).round() as u16
        } else {
            0
        };

        // Central scatter area (reduced when marginals
        // are present)
        let central_width = area.width.saturating_sub(right_width);
        let central_height = area.height.saturating_sub(top_height);
        if central_width < 6 || central_height < 6 {
            return;
        }

        let central_area = Rect::new(area.x, area.y + top_height, central_width, central_height);

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

        let mut pb = create_backend(central_area);

        // Create and render the plot frame
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

        let Some(pa) = frame.render_to_pb(&mut pb, central_area, bounds) else {
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
                    let fallback = s.color.unwrap_or(self.theme.primary);
                    let color = if let Some(ref cv) = self.color_values {
                        if global_point_idx < cv.len() {
                            let t = self.color_norm.normalize(cv[global_point_idx]);
                            self.colormap.color_at(t)
                        } else {
                            fallback
                        }
                    } else {
                        fallback
                    };

                    // Bubble mode: variable radius
                    if let Some(ref sv) = self.size_values {
                        if global_point_idx < sv.len() {
                            let sv_min = sv
                                .iter()
                                .cloned()
                                .filter(|v| v.is_finite())
                                .fold(f64::INFINITY, f64::min);
                            let sv_max = sv
                                .iter()
                                .cloned()
                                .filter(|v| v.is_finite())
                                .fold(f64::NEG_INFINITY, f64::max);
                            let sv_range = sv_max - sv_min;
                            let t = if sv_range > 0.0 {
                                (sv[global_point_idx] - sv_min) / sv_range
                            } else {
                                0.5
                            };
                            let radius = (self.size_range.0
                                + t * (self.size_range.1 - self.size_range.0))
                                .max(0.5);
                            let r_int = radius.round() as i16;
                            for dy in -r_int..=r_int {
                                for dx in -r_int..=r_int {
                                    if dx * dx + dy * dy <= r_int * r_int {
                                        let bx = xi as i16 + dx;
                                        let by = yi as i16 + dy;
                                        if bx >= 0 && by >= 0 {
                                            let bxu = bx as u16;
                                            let byu = by as u16;
                                            if pa.contains(bxu, byu) {
                                                pb.set_char(
                                                    bxu,
                                                    byu,
                                                    self.theme.chars.fill.solid,
                                                    color,
                                                    Z_MARKER,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            pb.set_char(xi, yi, marker.char(), color, Z_MARKER);
                        }
                    } else {
                        pb.set_char(xi, yi, marker.char(), color, Z_MARKER);
                    }
                }
                global_point_idx += 1;
            }
        }

        // Draw trendline (statistics feature)
        #[cfg(feature = "statistics")]
        if let Some(ref ttype) = self.trendline {
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

            let trend_color = self.trendline_color.unwrap_or_else(|| {
                self.series
                    .first()
                    .and_then(|s| s.color)
                    .unwrap_or(self.theme.primary)
            });

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
                TrendlineType::Lowess(frac) => crate::statistics::lowess(&all_x, &all_y, *frac)
                    .map(|result| {
                        eval_xs
                            .iter()
                            .map(|&ex| interpolate_lowess(&result.x, &result.y, ex))
                            .collect()
                    }),
            };

            if let Some(ys) = eval_ys {
                for i in 0..eval_xs.len() - 1 {
                    let sx0 = pa.screen_x(eval_xs[i]);
                    let sy0 = pa.screen_y(ys[i]);
                    let sx1 = pa.screen_x(eval_xs[i + 1]);
                    let sy1 = pa.screen_y(ys[i + 1]);
                    pb.draw_line(sx0, sy0, sx1, sy1, trend_color, &pa, Z_DATA);
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite to buffer before legend
        pb.composite(buf);

        frame.draw_end_labels(buf, central_area, &pa);

        // Draw legend (directly to buf, after composite)
        if self.show_legend && !self.series.is_empty() && self.color_values.is_none() {
            let legend = Legend::from_series(&self.series)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(pa.x, pa.y, pa.width, pa.height);
            (&legend).render(legend_area, buf);
        }

        // Render marginal distributions
        if has_top || has_right {
            let marginal_color = self
                .series
                .first()
                .and_then(|s| s.color)
                .unwrap_or(self.theme.primary);

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

            // Top marginal (x-axis distribution)
            if has_top && top_height >= 2 {
                let top_area = Rect::new(
                    pa.x,
                    area.y,
                    pa.width,
                    top_height.min(area.y + top_height - area.y),
                );
                let cfg = MarginalConfig {
                    data: &all_x,
                    marginal_type: &self.marginal_x,
                    bins: self.marginal_bins,
                    data_lo: pa.x_lo,
                    data_hi: pa.x_hi,
                    plot_origin: pa.x,
                    plot_extent: pa.width,
                    color: marginal_color,
                    chars: &self.theme.chars,
                };
                render_marginal_top(buf, top_area, &cfg);
            }

            // Right marginal (y-axis distribution)
            if has_right && right_width >= 2 {
                let right_area = Rect::new(area.x + central_width, pa.y, right_width, pa.height);
                let cfg = MarginalConfig {
                    data: &all_y,
                    marginal_type: &self.marginal_y,
                    bins: self.marginal_bins,
                    data_lo: pa.y_lo,
                    data_hi: pa.y_hi,
                    plot_origin: pa.y,
                    plot_extent: pa.height,
                    color: marginal_color,
                    chars: &self.theme.chars,
                };
                render_marginal_right(buf, right_area, &cfg);
            }
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
