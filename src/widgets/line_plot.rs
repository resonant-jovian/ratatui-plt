//! Line plot widget with error bars and fill support.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//!
//! let plot = LinePlot::new()
//!     .series(Series::new("sin").data(
//!         (0..100).map(|i| { let x = i as f64 * 0.1; (x, x.sin()) }).collect()
//!     ).color(Color::Cyan))
//!     .title("Trigonometric Functions")
//!     .x_axis(Axis::new().label("x").grid(true))
//!     .y_axis(Axis::new().label("y"));
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

#[cfg(feature = "statistics")]
use std::collections::BTreeMap;

#[cfg(feature = "statistics")]
use ordered_float::OrderedFloat;

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis};
use crate::frame::{DataBounds, PlotArea, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendPosition};
use crate::linked_view::SharedView;
use crate::plot_buffer::{PlotBackend, Z_DATA, Z_FILL, Z_MARKER, create_backend};
use crate::series::{Series, is_valid_point};
use crate::spines::Spines;
use crate::style::DashPattern;
use crate::theme::Theme;

#[cfg(feature = "statistics")]
use crate::statistics::{bootstrap_ci, mean_estimator, median_estimator};

/// Interpolation mode for line rendering.
///
/// Controls how data points are connected when drawing line segments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum InterpolationMode {
    /// Connect points with straight line segments (default).
    #[default]
    Linear,
    /// Use natural cubic spline interpolation to produce a smooth curve.
    /// Evaluates at 4x the data point count for visual smoothness.
    CubicSpline,
}

/// Line plot step mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepMode {
    /// No stepping (linear interpolation between points).
    None,
    /// Step before the point (horizontal then vertical).
    Pre,
    /// Step at midpoint.
    Mid,
    /// Step after the point (vertical then horizontal).
    Post,
}

/// CI lower and upper bound vectors for one series.
#[cfg(feature = "statistics")]
#[allow(dead_code)]
type CiBounds = Option<(Vec<f64>, Vec<f64>)>;

/// Method for aggregating multiple y-values at each x.
#[cfg(feature = "statistics")]
#[derive(Clone, Debug, Default)]
pub enum EstimatorType {
    /// Use the arithmetic mean as the point estimate.
    #[default]
    Mean,
    /// Use the median as the point estimate.
    Median,
}

/// How to display the confidence interval.
#[cfg(feature = "statistics")]
#[derive(Clone, Debug, Default)]
pub enum ErrorStyle {
    /// Render the CI as a shaded band between the lower and upper bounds.
    #[default]
    Band,
    /// Render the CI as vertical error bars at each aggregated point.
    Bars,
}

/// A 2D line plot widget.
#[derive(Clone)]
pub struct LinePlot {
    series: Vec<Series>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    aspect_ratio: AspectRatio,
    show_legend: bool,
    legend_position: LegendPosition,
    step_mode: StepMode,
    interpolation: InterpolationMode,
    annotations: Vec<Annotation>,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    shared_view: Option<SharedView>,
    #[cfg(feature = "statistics")]
    estimator: Option<EstimatorType>,
    #[cfg(feature = "statistics")]
    error_style: ErrorStyle,
    #[cfg(feature = "statistics")]
    ci_level: f64,
    #[cfg(feature = "statistics")]
    n_bootstrap: usize,
}

impl Default for LinePlot {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            aspect_ratio: AspectRatio::Auto,
            show_legend: true,
            legend_position: LegendPosition::TopRight,
            step_mode: StepMode::None,
            interpolation: InterpolationMode::default(),
            annotations: Vec::new(),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            shared_view: None,
            #[cfg(feature = "statistics")]
            estimator: None,
            #[cfg(feature = "statistics")]
            error_style: ErrorStyle::default(),
            #[cfg(feature = "statistics")]
            ci_level: 0.95,
            #[cfg(feature = "statistics")]
            n_bootstrap: 1000,
        }
    }
}

impl LinePlot {
    /// Create an empty line plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a data series.
    pub fn series(mut self, s: Series) -> Self {
        self.series.push(s);
        self
    }

    /// Add multiple series.
    pub fn series_vec(mut self, s: Vec<Series>) -> Self {
        self.series.extend(s);
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

    /// Set the aspect ratio.
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
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

    /// Set the step mode for step-line plots.
    pub fn step_mode(mut self, mode: StepMode) -> Self {
        self.step_mode = mode;
        self
    }

    /// Set the interpolation mode.
    ///
    /// When set to [`InterpolationMode::CubicSpline`], each series is
    /// smoothed with a natural cubic spline before rendering. The original
    /// rendering pipeline is unchanged — it simply receives more points.
    pub fn interpolation(mut self, mode: InterpolationMode) -> Self {
        self.interpolation = mode;
        self
    }

    /// Add an annotation.
    pub fn annotation(mut self, ann: Annotation) -> Self {
        self.annotations.push(ann);
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

    /// Link this plot to a shared view state for synchronized bounds.
    pub fn shared_view(mut self, sv: SharedView) -> Self {
        self.shared_view = Some(sv);
        self
    }

    /// Set the estimator for aggregating repeated y-values per x.
    ///
    /// When set, multiple y-values sharing the same x coordinate are
    /// aggregated using the chosen estimator. A bootstrap confidence
    /// interval is computed and rendered according to [`ErrorStyle`].
    #[cfg(feature = "statistics")]
    pub fn estimator(mut self, e: EstimatorType) -> Self {
        self.estimator = Some(e);
        self
    }

    /// Set the error style for confidence interval display.
    #[cfg(feature = "statistics")]
    pub fn error_style(mut self, s: ErrorStyle) -> Self {
        self.error_style = s;
        self
    }

    /// Set the confidence level for bootstrap CI (default 0.95).
    #[cfg(feature = "statistics")]
    pub fn ci_level(mut self, level: f64) -> Self {
        self.ci_level = level;
        self
    }

    /// Set the number of bootstrap resamples (default 1000).
    #[cfg(feature = "statistics")]
    pub fn n_bootstrap(mut self, n: usize) -> Self {
        self.n_bootstrap = n;
        self
    }
}

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for LinePlot {
    fn render_plotters(
        &self,
        area: Rect,
        buf: &mut Buffer,
        theme: &Theme,
    ) {
        use crate::plotters_render::{bridge, helpers, theme_bridge};
        use crate::series::is_valid_point;

        let (data_x_min, data_x_max) = self.compute_x_bounds();
        let (data_y_min, data_y_max) = self.compute_y_bounds();
        let (x_lo, x_hi) =
            self.x_axis.resolve_bounds(data_x_min, data_x_max);
        let (y_lo, y_hi) =
            self.y_axis.resolve_bounds(data_y_min, data_y_max);

        let series_ref = &self.series;
        let x_axis_ref = &self.x_axis;
        let y_axis_ref = &self.y_axis;
        let title_ref = self.title.as_deref();
        let color_cycle = &theme.color_cycle;

        bridge::render_plotters_to_buf(
            area,
            buf,
            theme_bridge::theme_bg_rgb(theme),
            |root| {
                let Ok(mut chart) = helpers::build_cartesian_2d(
                    root,
                    x_axis_ref,
                    y_axis_ref,
                    title_ref,
                    theme,
                    x_lo..x_hi,
                    y_lo..y_hi,
                ) else {
                    return;
                };

                for (si, series) in series_ref.iter().enumerate()
                {
                    let color = series
                        .color
                        .unwrap_or_else(|| color_cycle.at(si));
                    let pc = theme_bridge::to_plotters_color(color);

                    let points: Vec<(f64, f64)> = series
                        .data
                        .iter()
                        .copied()
                        .filter(|&(x, y)| is_valid_point(x, y))
                        .collect();

                    let _ = chart.draw_series(
                        plotters::series::LineSeries::new(
                            points.iter().copied(),
                            plotters::style::ShapeStyle::from(pc)
                                .stroke_width(4),
                        ),
                    );
                }
            },
        );
    }
}

impl Widget for &LinePlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        #[cfg(feature = "plotters-render")]
        {
            if crate::plotters_render::should_use_plotters() {
                use crate::plotters_render::PlottersRenderable;
                self.render_plotters(area, buf, &self.theme);
                return;
            }
        }
        // Compute data bounds
        let (data_x_min, data_x_max) = self.compute_x_bounds();
        let (data_y_min, data_y_max) = self.compute_y_bounds();

        let (mut x_lo, mut x_hi) = self.x_axis.resolve_bounds(data_x_min, data_x_max);
        let (mut y_lo, mut y_hi) = self.y_axis.resolve_bounds(data_y_min, data_y_max);

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

        let mut pb = create_backend(area);

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

        let clip = ClipRect::from_plot_area(&pa);

        // Resolve series colors: use explicitly set color, or auto-assign from theme color cycle
        let mut color_cycle = self.theme.color_cycle.clone();
        let resolved_colors: Vec<Color> = self
            .series
            .iter()
            .map(|s| {
                if let Some(c) = s.color {
                    c
                } else {
                    color_cycle.next_color()
                }
            })
            .collect();

        // When a statistical estimator is configured, aggregate each
        // series (grouping y-values by x) and collect CI bounds.
        // The aggregated series replace the originals for all
        // downstream rendering.
        #[cfg(feature = "statistics")]
        let (agg_series, ci_bounds): (Vec<Series>, Vec<CiBounds>) = if let Some(ref est) =
            self.estimator
        {
            self.series
                .iter()
                .map(|s| {
                    let (agg, lo, hi) = aggregate_series(s, est, self.ci_level, self.n_bootstrap);
                    (agg, Some((lo, hi)))
                })
                .unzip()
        } else {
            self.series.iter().map(|s| (s.clone(), None)).unzip()
        };

        #[cfg(feature = "statistics")]
        let render_series: &[Series] = &agg_series;
        #[cfg(not(feature = "statistics"))]
        let render_series: &[Series] = &self.series;

        // Apply cubic spline interpolation when requested.
        // This produces denser point arrays for smooth curves while leaving
        // the downstream rendering code unchanged.
        let interpolated: Vec<Vec<(f64, f64)>> =
            if self.interpolation == InterpolationMode::CubicSpline {
                render_series
                    .iter()
                    .map(|s| {
                        let valid: Vec<(f64, f64)> = s
                            .data
                            .iter()
                            .copied()
                            .filter(|&(x, y)| is_valid_point(x, y))
                            .collect();
                        if valid.len() < 3 {
                            valid
                        } else {
                            cubic_spline_interpolate(&valid)
                        }
                    })
                    .collect()
            } else {
                render_series.iter().map(|s| s.data.clone()).collect()
            };

        // Draw fill regions (using interpolated data for smoother fill)
        for (si, s) in render_series.iter().enumerate() {
            if let Some(ref fill_to) = s.fill_to {
                let baseline = match fill_to {
                    crate::series::FillTo::Baseline(y) => *y,
                    _ => y_lo,
                };
                let baseline_screen = pa.screen_y(baseline);

                let data = &interpolated[si];
                // Fill column-by-column between adjacent interpolated points.
                // For each screen column inside the plot area, linearly interpolate
                // the series y-value from the (possibly spline-densified) data and
                // fill between that y and the baseline.
                for col_offset in 0..pa.width {
                    let screen_x = pa.x + col_offset;
                    // Map screen column back to data x
                    let data_x = x_lo
                        + (col_offset as f64 / (pa.width.saturating_sub(1)).max(1) as f64)
                            * (x_hi - x_lo);

                    // Linearly interpolate y at data_x from the interpolated points
                    let data_y = interp_y_at(data, data_x);
                    if !data_y.is_finite() {
                        continue;
                    }

                    let sy = pa.screen_y(data_y);
                    let y_top = sy.round().min(baseline_screen.round()) as u16;
                    let y_bot = sy.round().max(baseline_screen.round()) as u16;

                    for y in y_top..=y_bot {
                        if pa.contains(screen_x, y) {
                            pb.set_bg(screen_x, y, resolved_colors[si], Z_FILL);
                        }
                    }
                }
            }
        }

        // Draw confidence interval regions (statistics feature)
        #[cfg(feature = "statistics")]
        for (si, ci_opt) in ci_bounds.iter().enumerate() {
            if let Some((y_lo_ci, y_hi_ci)) = ci_opt {
                let color = resolved_colors[si];
                let series_data = &render_series[si].data;

                match self.error_style {
                    ErrorStyle::Band => {
                        // Build sorted (x, lo, hi) triples for
                        // interpolation
                        let ci_pts: Vec<(f64, f64, f64)> = series_data
                            .iter()
                            .enumerate()
                            .filter_map(|(i, &(x, _))| {
                                if i < y_lo_ci.len() && i < y_hi_ci.len() && x.is_finite() {
                                    Some((x, y_lo_ci[i], y_hi_ci[i]))
                                } else {
                                    None
                                }
                            })
                            .collect();

                        if ci_pts.is_empty() {
                            continue;
                        }

                        let fill_char = self.theme.chars.fill.light;
                        for col_offset in 0..pa.width {
                            let screen_x = pa.x + col_offset;
                            let data_x = x_lo
                                + (col_offset as f64 / (pa.width.saturating_sub(1)).max(1) as f64)
                                    * (x_hi - x_lo);

                            let lo_y = interp_ci_at(&ci_pts, data_x, true);
                            let hi_y = interp_ci_at(&ci_pts, data_x, false);

                            if !lo_y.is_finite() || !hi_y.is_finite() {
                                continue;
                            }

                            let sy_lo_px = pa.screen_y(lo_y);
                            let sy_hi_px = pa.screen_y(hi_y);

                            // screen_y is inverted (higher data
                            // y = lower screen y)
                            let y_top = sy_hi_px.round().min(sy_lo_px.round()) as u16;
                            let y_bot = sy_hi_px.round().max(sy_lo_px.round()) as u16;

                            for y in y_top..=y_bot {
                                if pa.contains(screen_x, y) {
                                    pb.set_char(screen_x, y, fill_char, color, Z_FILL);
                                }
                            }
                        }
                    }
                    ErrorStyle::Bars => {
                        for (i, &(x, _y)) in series_data.iter().enumerate() {
                            if i >= y_lo_ci.len() || i >= y_hi_ci.len() {
                                continue;
                            }
                            let sx = pa.screen_x(x);
                            let xi = sx.round() as u16;
                            if xi < pa.x || xi >= pa.x + pa.width {
                                continue;
                            }

                            let lo = y_lo_ci[i];
                            let hi = y_hi_ci[i];

                            let sy_lo_px = pa.screen_y(lo);
                            let sy_hi_px = pa.screen_y(hi);

                            let y_top = sy_hi_px.round() as u16;
                            let y_bot = sy_lo_px.round() as u16;

                            for ey in y_top..=y_bot {
                                if pa.contains(xi, ey) {
                                    pb.set_char(
                                        xi,
                                        ey,
                                        self.theme.chars.border.vertical,
                                        color,
                                        Z_DATA,
                                    );
                                }
                            }
                            if pa.contains(xi, y_top) {
                                pb.set_char(
                                    xi,
                                    y_top,
                                    self.theme.chars.tick.cap_top,
                                    color,
                                    Z_DATA,
                                );
                            }
                            if pa.contains(xi, y_bot) {
                                pb.set_char(
                                    xi,
                                    y_bot,
                                    self.theme.chars.tick.cap_bottom,
                                    color,
                                    Z_DATA,
                                );
                            }
                        }
                    }
                }
            }
        }

        // Draw error bars (from series-level y_err)
        for (si, s) in render_series.iter().enumerate() {
            let color = resolved_colors[si];
            if s.y_err_low.is_some() || s.y_err_high.is_some() {
                for (i, &(x, y)) in s.data.iter().enumerate() {
                    let sx = pa.screen_x(x);
                    let xi = sx.round() as u16;
                    if xi < pa.x || xi >= pa.x + pa.width {
                        continue;
                    }

                    let lo = y - s.y_err_low.as_ref().map_or(0.0, |e| e[i]);
                    let hi = y + s.y_err_high.as_ref().map_or(0.0, |e| e[i]);

                    let sy_lo = pa.screen_y(lo);
                    let sy_hi = pa.screen_y(hi);

                    let y_top = sy_hi.round() as u16;
                    let y_bot = sy_lo.round() as u16;

                    for ey in y_top..=y_bot {
                        if pa.contains(xi, ey) {
                            pb.set_char(xi, ey, self.theme.chars.border.vertical, color, Z_DATA);
                        }
                    }
                    // Caps
                    if pa.contains(xi, y_top) {
                        pb.set_char(xi, y_top, self.theme.chars.tick.cap_top, color, Z_DATA);
                    }
                    if pa.contains(xi, y_bot) {
                        pb.set_char(xi, y_bot, self.theme.chars.tick.cap_bottom, color, Z_DATA);
                    }
                }
            }
        }

        // Draw line series (using interpolated data for line segments)
        for (si, s) in render_series.iter().enumerate() {
            let color = resolved_colors[si];
            let line_data = &interpolated[si];
            if line_data.len() < 2 {
                // Just draw markers for single-point series
                for &(x, y) in &s.data {
                    let sx = pa.screen_x(x);
                    let sy = pa.screen_y(y);
                    let xi = sx.round() as u16;
                    let yi = sy.round() as u16;
                    if pa.contains(xi, yi) {
                        let ch = s
                            .marker
                            .map_or(self.theme.chars.marker.default_point, |m| m.char());
                        pb.set_char(xi, yi, ch, color, Z_MARKER);
                    }
                }
                continue;
            }

            // Draw lines between consecutive points, breaking at NaN
            for i in 0..line_data.len() - 1 {
                let (x0, y0) = line_data[i];
                let (x1, y1) = line_data[i + 1];

                // Skip line segments where either endpoint is NaN/infinite
                if !is_valid_point(x0, y0) || !is_valid_point(x1, y1) {
                    continue;
                }

                match self.step_mode {
                    StepMode::None => {
                        // Normal linear interpolation
                        let sx0 = pa.screen_x(x0);
                        let sy0 = pa.screen_y(y0);
                        let sx1 = pa.screen_x(x1);
                        let sy1 = pa.screen_y(y1);
                        draw_line_pb(
                            &mut pb,
                            &LineSegment {
                                x0: sx0,
                                y0: sy0,
                                x1: sx1,
                                y1: sy1,
                            },
                            color,
                            &s.line_style.pattern,
                            &clip,
                            Z_DATA + si as u8,
                        );
                    }
                    StepMode::Pre => {
                        // Horizontal then vertical: (x0,y0) -> (x1,y0) -> (x1,y1)
                        let sx0 = pa.screen_x(x0);
                        let sy0 = pa.screen_y(y0);
                        let sx1 = pa.screen_x(x1);
                        let sy1 = pa.screen_y(y1);
                        // Horizontal segment at y0
                        draw_line_pb(
                            &mut pb,
                            &LineSegment {
                                x0: sx0,
                                y0: sy0,
                                x1: sx1,
                                y1: sy0,
                            },
                            color,
                            &s.line_style.pattern,
                            &clip,
                            Z_DATA + si as u8,
                        );
                        // Vertical segment at x1
                        draw_line_pb(
                            &mut pb,
                            &LineSegment {
                                x0: sx1,
                                y0: sy0,
                                x1: sx1,
                                y1: sy1,
                            },
                            color,
                            &s.line_style.pattern,
                            &clip,
                            Z_DATA + si as u8,
                        );
                    }
                    StepMode::Post => {
                        // Vertical then horizontal: (x0,y0) -> (x0,y1) -> (x1,y1)
                        let sx0 = pa.screen_x(x0);
                        let sy0 = pa.screen_y(y0);
                        let sx1 = pa.screen_x(x1);
                        let sy1 = pa.screen_y(y1);
                        // Vertical segment at x0
                        draw_line_pb(
                            &mut pb,
                            &LineSegment {
                                x0: sx0,
                                y0: sy0,
                                x1: sx0,
                                y1: sy1,
                            },
                            color,
                            &s.line_style.pattern,
                            &clip,
                            Z_DATA + si as u8,
                        );
                        // Horizontal segment at y1
                        draw_line_pb(
                            &mut pb,
                            &LineSegment {
                                x0: sx0,
                                y0: sy1,
                                x1: sx1,
                                y1: sy1,
                            },
                            color,
                            &s.line_style.pattern,
                            &clip,
                            Z_DATA + si as u8,
                        );
                    }
                    StepMode::Mid => {
                        // Step at midpoint: (x0,y0) -> (mid_x,y0) -> (mid_x,y1) -> (x1,y1)
                        let mid_x = (x0 + x1) / 2.0;
                        let sx0 = pa.screen_x(x0);
                        let sy0 = pa.screen_y(y0);
                        let smx = pa.screen_x(mid_x);
                        let sx1 = pa.screen_x(x1);
                        let sy1 = pa.screen_y(y1);
                        // Horizontal at y0 from x0 to mid
                        draw_line_pb(
                            &mut pb,
                            &LineSegment {
                                x0: sx0,
                                y0: sy0,
                                x1: smx,
                                y1: sy0,
                            },
                            color,
                            &s.line_style.pattern,
                            &clip,
                            Z_DATA + si as u8,
                        );
                        // Vertical at mid from y0 to y1
                        draw_line_pb(
                            &mut pb,
                            &LineSegment {
                                x0: smx,
                                y0: sy0,
                                x1: smx,
                                y1: sy1,
                            },
                            color,
                            &s.line_style.pattern,
                            &clip,
                            Z_DATA + si as u8,
                        );
                        // Horizontal at y1 from mid to x1
                        draw_line_pb(
                            &mut pb,
                            &LineSegment {
                                x0: smx,
                                y0: sy1,
                                x1: sx1,
                                y1: sy1,
                            },
                            color,
                            &s.line_style.pattern,
                            &clip,
                            Z_DATA + si as u8,
                        );
                    }
                }
            }

            // Draw markers (skip NaN points)
            if let Some(marker) = s.marker {
                for &(x, y) in &s.data {
                    if !is_valid_point(x, y) {
                        continue;
                    }
                    let sx = pa.screen_x(x);
                    let sy = pa.screen_y(y);
                    let xi = sx.round() as u16;
                    let yi = sy.round() as u16;
                    if pa.contains(xi, yi) {
                        pb.set_char(xi, yi, marker.char(), color, Z_MARKER);
                    }
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite to buffer before legend
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Draw legend (directly to buf, after composite)
        if self.show_legend && !self.series.is_empty() {
            let legend = Legend::from_series(&self.series)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(pa.x, pa.y, pa.width, pa.height);
            (&legend).render(legend_area, buf);
        }
    }
}

impl LinePlot {
    fn compute_x_bounds(&self) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for s in &self.series {
            if let Some((lo, hi)) = s.x_bounds() {
                min = min.min(lo);
                max = max.max(hi);
            }
        }
        if min.is_infinite() {
            (0.0, 1.0)
        } else {
            (min, max)
        }
    }

    fn compute_y_bounds(&self) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for s in &self.series {
            if let Some((lo, hi)) = s.y_bounds() {
                min = min.min(lo);
                max = max.max(hi);
            }
        }
        if min.is_infinite() {
            (0.0, 1.0)
        } else {
            (min, max)
        }
    }
}

/// Aggregate a series by grouping y-values at each unique x,
/// computing a point estimate and bootstrap confidence interval.
///
/// Returns a new `Series` with one point per unique x (sorted),
/// plus vectors of CI lower and upper bounds aligned to those points.
#[cfg(feature = "statistics")]
#[allow(dead_code)]
fn aggregate_series(
    series: &Series,
    estimator: &EstimatorType,
    ci_level: f64,
    n_bootstrap: usize,
) -> (Series, Vec<f64>, Vec<f64>) {
    // Group (x, y) pairs by x using OrderedFloat for BTreeMap key.
    let mut groups: BTreeMap<OrderedFloat<f64>, Vec<f64>> = BTreeMap::new();
    for &(x, y) in &series.data {
        if !is_valid_point(x, y) {
            continue;
        }
        groups.entry(OrderedFloat(x)).or_default().push(y);
    }

    let est_fn = match estimator {
        EstimatorType::Mean => mean_estimator,
        EstimatorType::Median => median_estimator,
    };

    let mut agg_data = Vec::with_capacity(groups.len());
    let mut y_low = Vec::with_capacity(groups.len());
    let mut y_high = Vec::with_capacity(groups.len());

    for (ox, ys) in &groups {
        let x = ox.into_inner();
        let point_est = est_fn(ys);

        if ys.len() < 2 {
            // Not enough data for bootstrap; CI collapses to point.
            agg_data.push((x, point_est));
            y_low.push(point_est);
            y_high.push(point_est);
        } else {
            let ci = bootstrap_ci(ys, est_fn, n_bootstrap, ci_level, 42);
            agg_data.push((x, ci.estimate));
            y_low.push(ci.lower);
            y_high.push(ci.upper);
        }
    }

    let mut agg_series = Series::new(&series.name).data(agg_data);
    if let Some(c) = series.color {
        agg_series = agg_series.color(c);
    }
    if let Some(m) = series.marker {
        agg_series = agg_series.marker(m);
    }

    (agg_series, y_low, y_high)
}

/// Clipping rectangle for line drawing.
struct ClipRect {
    x_min: u16,
    y_min: u16,
    x_max: u16,
    y_max: u16,
}

impl ClipRect {
    /// Create a `ClipRect` from a `PlotArea`.
    fn from_plot_area(pa: &PlotArea) -> Self {
        Self {
            x_min: pa.x,
            y_min: pa.y,
            x_max: pa.x + pa.width,
            y_max: pa.y + pa.height,
        }
    }
}

/// Braille sub-pixel bit layout for each column/row within a cell.
/// Braille characters (U+2800..=U+28FF) encode 8 dots in a 2x4 grid.
const BRAILLE_BITS: [[u8; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40], // column 0: rows 0-3
    [0x08, 0x10, 0x20, 0x80], // column 1: rows 0-3
];

/// Screen-space line segment endpoints.
struct LineSegment {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

/// Draw a line between two screen points using Bresenham's at braille sub-pixel resolution,
/// writing into a [`PlotBuffer`] at the given Z-level.
fn draw_line_pb(
    pb: &mut dyn PlotBackend,
    seg: &LineSegment,
    color: Color,
    pattern: &DashPattern,
    clip: &ClipRect,
    z: u8,
) {
    let LineSegment { x0, y0, x1, y1 } = *seg;
    // Scale to braille sub-pixel coordinates (2x horizontal, 4x vertical)
    let mut ix0 = (x0 * 2.0).round() as i32;
    let mut iy0 = (y0 * 4.0).round() as i32;
    let ix1 = (x1 * 2.0).round() as i32;
    let iy1 = (y1 * 4.0).round() as i32;

    let dx = (ix1 - ix0).abs();
    let dy = -(iy1 - iy0).abs();
    let sx = if ix0 < ix1 { 1 } else { -1 };
    let sy = if iy0 < iy1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut step = 0u32;

    loop {
        let draw = match pattern {
            DashPattern::Solid => true,
            DashPattern::Dashed => (step / 8).is_multiple_of(2),
            DashPattern::Dotted => (step / 3).is_multiple_of(2),
            DashPattern::DashDot => {
                let cycle = step % 14;
                cycle < 8 || (10..12).contains(&cycle)
            }
            DashPattern::Custom(segs) => {
                if segs.is_empty() {
                    true
                } else {
                    let total: u32 = segs.iter().map(|&s| s as u32).sum();
                    if total == 0 {
                        true
                    } else {
                        let pos = step % total;
                        let mut accum = 0u32;
                        let mut on = true;
                        let mut result = true;
                        for &seg in segs {
                            accum += seg as u32;
                            if pos < accum {
                                result = on;
                                break;
                            }
                            on = !on;
                        }
                        result
                    }
                }
            }
        };

        if draw && ix0 >= 0 && iy0 >= 0 {
            let cell_x = (ix0 / 2) as u16;
            let cell_y = (iy0 / 4) as u16;
            if cell_x >= clip.x_min
                && cell_x < clip.x_max
                && cell_y >= clip.y_min
                && cell_y < clip.y_max
            {
                let dot_col = (ix0 % 2) as usize;
                let dot_row = (iy0 % 4) as usize;
                let bit = BRAILLE_BITS[dot_col][dot_row];
                pb.set_braille(cell_x, cell_y, bit, color, z);
            }
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
        step += 1;
    }
}

/// Linearly interpolate a y value from sorted (x, y) data at a given x.
///
/// Returns `f64::NAN` when `data` is empty. Clamps to endpoint values
/// when `x` falls outside the data range.
fn interp_y_at(data: &[(f64, f64)], x: f64) -> f64 {
    if data.is_empty() {
        return f64::NAN;
    }
    if data.len() == 1 {
        return data[0].1;
    }
    if x <= data[0].0 {
        return data[0].1;
    }
    let last = data.len() - 1;
    if x >= data[last].0 {
        return data[last].1;
    }
    for i in 0..last {
        let (x0, y0) = data[i];
        let (x1, y1) = data[i + 1];
        if x >= x0 && x <= x1 {
            let dx = x1 - x0;
            if dx.abs() < 1e-15 {
                return y0;
            }
            let t = (x - x0) / dx;
            return y0 + t * (y1 - y0);
        }
    }
    data[last].1
}

/// Linearly interpolate CI bounds from sorted (x, lo, hi) triples.
///
/// When `lower` is true, interpolates the lower bound; otherwise the
/// upper bound. Returns `f64::NAN` when `data` is empty.
#[cfg(feature = "statistics")]
fn interp_ci_at(data: &[(f64, f64, f64)], x: f64, lower: bool) -> f64 {
    if data.is_empty() {
        return f64::NAN;
    }
    let pick = |t: &(f64, f64, f64)| {
        if lower { t.1 } else { t.2 }
    };
    if data.len() == 1 {
        return pick(&data[0]);
    }
    if x <= data[0].0 {
        return pick(&data[0]);
    }
    let last = data.len() - 1;
    if x >= data[last].0 {
        return pick(&data[last]);
    }
    for i in 0..last {
        let x0 = data[i].0;
        let x1 = data[i + 1].0;
        if x >= x0 && x <= x1 {
            let dx = x1 - x0;
            if dx.abs() < 1e-15 {
                return pick(&data[i]);
            }
            let t = (x - x0) / dx;
            let v0 = pick(&data[i]);
            let v1 = pick(&data[i + 1]);
            return v0 + t * (v1 - v0);
        }
    }
    pick(&data[last])
}

/// Compute a natural cubic spline through `pts` and evaluate at 4x density.
///
/// `pts` must be sorted by x, contain no NaN values, and have at least 3
/// elements. Returns evenly-spaced evaluated points from x_min to x_max.
///
/// Algorithm:
/// 1. Build the tridiagonal system for natural spline second derivatives.
/// 2. Solve via the Thomas algorithm (forward elimination + back substitution).
/// 3. Compute cubic polynomial coefficients per interval.
/// 4. Evaluate at `4 * n` evenly-spaced x values.
fn cubic_spline_interpolate(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = pts.len();
    if n < 3 {
        return pts.to_vec();
    }

    // h[i] = x[i+1] - x[i]
    let h: Vec<f64> = (0..n - 1).map(|i| pts[i + 1].0 - pts[i].0).collect();

    // Check for degenerate intervals — if any h is too small, fall back
    if h.iter().any(|&hi| hi.abs() < 1e-15) {
        return pts.to_vec();
    }

    // Set up tridiagonal system for second derivatives (natural spline: M[0] = M[n-1] = 0)
    // Interior equations: h[i-1]*M[i-1] + 2*(h[i-1]+h[i])*M[i] + h[i]*M[i+1] = 6*d[i]
    // where d[i] = (y[i+1]-y[i])/h[i] - (y[i]-y[i-1])/h[i-1]
    let m = n - 2; // number of interior unknowns
    let mut diag = vec![0.0; m]; // main diagonal
    let mut upper = vec![0.0; m]; // upper diagonal
    let mut rhs = vec![0.0; m]; // right-hand side

    for i in 0..m {
        let idx = i + 1; // maps to global index
        diag[i] = 2.0 * (h[idx - 1] + h[idx]);
        if i + 1 < m {
            upper[i] = h[idx];
        }
        let slope_right = (pts[idx + 1].1 - pts[idx].1) / h[idx];
        let slope_left = (pts[idx].1 - pts[idx - 1].1) / h[idx - 1];
        rhs[i] = 6.0 * (slope_right - slope_left);
    }

    // Thomas algorithm — forward sweep
    // lower[i] = h[i] (the sub-diagonal), but we consume it during elimination
    for i in 1..m {
        let lower_i = h[i]; // h[global_idx - 1] where global_idx = i + 1
        if diag[i - 1].abs() < 1e-30 {
            // Degenerate pivot; fall back to linear
            return pts.to_vec();
        }
        let factor = lower_i / diag[i - 1];
        diag[i] -= factor * upper[i - 1];
        rhs[i] -= factor * rhs[i - 1];
    }

    // Back substitution
    let mut moments = vec![0.0; n]; // M[0] = M[n-1] = 0 (natural spline)
    if diag[m - 1].abs() < 1e-30 {
        return pts.to_vec();
    }
    moments[m] = rhs[m - 1] / diag[m - 1]; // M[n-2]
    for i in (0..m - 1).rev() {
        if diag[i].abs() < 1e-30 {
            return pts.to_vec();
        }
        moments[i + 1] = (rhs[i] - upper[i] * moments[i + 2]) / diag[i];
    }

    // Evaluate spline at 4x density
    let out_count = 4 * n;
    let x_min = pts[0].0;
    let x_max = pts[n - 1].0;
    let x_span = x_max - x_min;
    if x_span.abs() < 1e-15 {
        return pts.to_vec();
    }

    let mut result = Vec::with_capacity(out_count);
    let mut seg = 0usize; // current spline segment index

    for k in 0..out_count {
        let x = x_min + x_span * (k as f64) / (out_count - 1).max(1) as f64;

        // Advance segment index so that pts[seg].0 <= x <= pts[seg+1].0
        while seg + 2 < n && x > pts[seg + 1].0 {
            seg += 1;
        }

        let hi = h[seg];
        let a = (pts[seg + 1].0 - x) / hi;
        let b = (x - pts[seg].0) / hi;

        let y = a * pts[seg].1
            + b * pts[seg + 1].1
            + (a * a * a - a) * (hi * hi / 6.0) * moments[seg]
            + (b * b * b - b) * (hi * hi / 6.0) * moments[seg + 1];

        result.push((x, y));
    }

    result
}
