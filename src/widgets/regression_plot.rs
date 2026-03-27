//! Scatter plot with fitted regression line and confidence band.
//!
//! Renders scatter points from a [`Series`], computes a regression fit (linear
//! or polynomial), and draws the fitted curve with an optional confidence band
//! showing the prediction uncertainty.
//!
//! Requires the `statistics` feature flag.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//!
//! let series = Series::new("data")
//!     .data(vec![(1.0, 2.1), (2.0, 3.9), (3.0, 6.2), (4.0, 7.8)]);
//!
//! let plot = RegressionPlot::new(series)
//!     .order(1)
//!     .ci(0.95)
//!     .title("Linear Regression");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBackend, Z_DATA, Z_FILL, Z_MARKER, create_backend};
use crate::series::Series;
use crate::spines::Spines;
use crate::style::MarkerShape;
use crate::theme::Theme;

/// Number of evaluation points for the regression curve and CI band.
const N_EVAL: usize = 100;

/// A scatter plot with fitted regression line and confidence band.
///
/// Computes ordinary least squares (linear or polynomial) regression on the
/// provided data and overlays the fitted curve. When `show_ci` is enabled a
/// confidence band is drawn using the classic OLS standard error formula.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let s = Series::new("measurements")
///     .data(vec![(1.0, 2.1), (2.0, 4.0), (3.0, 5.9), (4.0, 8.1)]);
///
/// let plot = RegressionPlot::new(s)
///     .order(1)
///     .ci(0.95)
///     .title("OLS Fit");
/// ```
pub struct RegressionPlot {
    series: Series,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    /// Polynomial degree (1 = linear, default).
    order: usize,
    /// Confidence interval level (default: 0.95).
    ci: f64,
    /// Show scatter points (default: true).
    show_scatter: bool,
    /// Show confidence band (default: true).
    show_ci: bool,
    /// Override color for the regression line.
    line_color: Option<Color>,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl RegressionPlot {
    /// Create a new regression plot from a series of (x, y) data.
    pub fn new(series: Series) -> Self {
        Self {
            series,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            order: 1,
            ci: 0.95,
            show_scatter: true,
            show_ci: true,
            line_color: None,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }

    /// Set the polynomial degree for the fit (1 = linear, default).
    pub fn order(mut self, order: usize) -> Self {
        self.order = order.max(1);
        self
    }

    /// Set the confidence interval level (default: 0.95).
    ///
    /// Value is clamped to (0, 1).
    pub fn ci(mut self, ci: f64) -> Self {
        self.ci = ci.clamp(0.01, 0.99);
        self
    }

    /// Show or hide scatter points (default: true).
    pub fn show_scatter(mut self, show: bool) -> Self {
        self.show_scatter = show;
        self
    }

    /// Show or hide the confidence band (default: true).
    pub fn show_ci(mut self, show: bool) -> Self {
        self.show_ci = show;
        self
    }

    /// Set the regression line color (defaults to theme primary).
    pub fn line_color(mut self, color: Color) -> Self {
        self.line_color = Some(color);
        self
    }

    /// Set the plot title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the X axis configuration.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y axis configuration.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
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

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

impl Widget for &RegressionPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // ── 1. Extract finite (x, y) pairs ──────────────────────────────
        let (xs, ys): (Vec<f64>, Vec<f64>) = self
            .series
            .data
            .iter()
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .map(|&(x, y)| (x, y))
            .unzip();

        if xs.len() < 2 {
            return;
        }

        // ── 2. Compute axis bounds ──────────────────────────────────────
        let data_x_min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let data_x_max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let data_y_min = ys.iter().cloned().fold(f64::INFINITY, f64::min);
        let data_y_max = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(data_x_min, data_x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(data_y_min, data_y_max);

        // ── 3. Set up PlotFrame ─────────────────────────────────────────
        let mut pb = create_backend(area);

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

        // ── 4. Draw scatter points ──────────────────────────────────────
        if self.show_scatter {
            let marker = self.series.marker.unwrap_or(MarkerShape::Dot);
            let point_color = self.series.color.unwrap_or(self.theme.primary);

            for &(x, y) in &self.series.data {
                if !x.is_finite() || !y.is_finite() {
                    continue;
                }
                let sx = pa.screen_x(x);
                let sy = pa.screen_y(y);
                let xi = sx.round() as u16;
                let yi = sy.round() as u16;
                if pa.contains(xi, yi) {
                    pb.set_char(xi, yi, marker.char(), point_color, Z_MARKER);
                }
            }
        }

        // ── 5. Compute regression ───────────────────────────────────────
        let line_color = self
            .line_color
            .or(self.series.color)
            .unwrap_or(self.theme.secondary);

        // Generate evaluation x values spanning the plot range.
        let eval_xs: Vec<f64> = (0..N_EVAL)
            .map(|i| x_lo + (x_hi - x_lo) * i as f64 / (N_EVAL - 1).max(1) as f64)
            .collect();

        // Fit and evaluate: dispatch on order.
        let eval_ys: Option<Vec<f64>> = if self.order == 1 {
            crate::statistics::linear_regression(&xs, &ys)
                .map(|fit| eval_xs.iter().map(|&x| fit.eval(x)).collect())
        } else {
            crate::statistics::poly_fit(&xs, &ys, self.order)
                .map(|fit| eval_xs.iter().map(|&x| fit.eval(x)).collect())
        };

        let Some(predicted) = eval_ys else {
            // Fit failed — still draw scatter and annotations, then return.
            PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);
            pb.composite(buf);
            frame.draw_end_labels(buf, area, &pa);
            return;
        };

        // ── 6. Draw the regression line ─────────────────────────────────
        for i in 0..eval_xs.len() - 1 {
            let sx0 = pa.screen_x(eval_xs[i]);
            let sy0 = pa.screen_y(predicted[i]);
            let sx1 = pa.screen_x(eval_xs[i + 1]);
            let sy1 = pa.screen_y(predicted[i + 1]);
            pb.draw_line(sx0, sy0, sx1, sy1, line_color, &pa, Z_DATA);
        }

        // ── 7. Draw confidence band ─────────────────────────────────────
        if self.show_ci {
            // Compute residuals against a per-data-point predicted value.
            let data_predicted: Vec<f64> = if self.order == 1 {
                crate::statistics::linear_regression(&xs, &ys)
                    .map(|fit| xs.iter().map(|&x| fit.eval(x)).collect())
                    .unwrap_or_default()
            } else {
                crate::statistics::poly_fit(&xs, &ys, self.order)
                    .map(|fit| xs.iter().map(|&x| fit.eval(x)).collect())
                    .unwrap_or_default()
            };

            if data_predicted.len() == ys.len() {
                let n = xs.len() as f64;
                let dof = n - (self.order as f64 + 1.0);

                if dof > 0.0 {
                    // Residual sum of squares.
                    let ss_res: f64 = ys
                        .iter()
                        .zip(data_predicted.iter())
                        .map(|(&y, &yp)| (y - yp) * (y - yp))
                        .sum();

                    let mse = ss_res / dof;
                    let se = mse.sqrt(); // residual standard error

                    // For the CI band we use the approximate z-multiplier.
                    // Proper t-distribution quantiles require a table; for
                    // large n the normal approximation is sufficient.
                    let z = ci_z_multiplier(self.ci);

                    // Mean of x values (used in the leverage formula).
                    let x_bar = crate::statistics::mean(&xs);
                    let ss_xx: f64 = xs.iter().map(|&x| (x - x_bar) * (x - x_bar)).sum();

                    // Avoid division by zero when all x values are identical.
                    let ss_xx_safe = if ss_xx.abs() < 1e-15 { 1.0 } else { ss_xx };

                    // Compute upper and lower CI bounds at each eval point.
                    // For polynomial order > 1 the exact leverage formula differs,
                    // but the linear-regression style band is a reasonable visual
                    // approximation and avoids needing matrix inversion.
                    let ci_half: Vec<f64> = eval_xs
                        .iter()
                        .map(|&x| {
                            let leverage = 1.0 / n + (x - x_bar) * (x - x_bar) / ss_xx_safe;
                            z * se * leverage.sqrt()
                        })
                        .collect();

                    // Use a dimmed version of the line color for the CI band.
                    let band_color = dim_color(line_color);

                    // Draw the band column by column using half-block characters.
                    for col_offset in 0..pa.width {
                        let screen_x = pa.x + col_offset;
                        let t = col_offset as f64 / (pa.width - 1).max(1) as f64;
                        // Map screen column to eval index.
                        let fi = t * (N_EVAL - 1) as f64;
                        let idx = (fi as usize).min(N_EVAL - 2);
                        let frac = fi - idx as f64;

                        // Linearly interpolate predicted and ci_half at this column.
                        let yp = predicted[idx] + frac * (predicted[idx + 1] - predicted[idx]);
                        let ch = ci_half[idx] + frac * (ci_half[idx + 1] - ci_half[idx]);

                        let y_upper = yp + ch;
                        let y_lower = yp - ch;

                        let sy_top = pa.screen_y(y_upper.max(y_lower));
                        let sy_bot = pa.screen_y(y_upper.min(y_lower));

                        // Half-pixel resolution: each cell has top (even) and bottom (odd).
                        let hp_top = (sy_top * 2.0).round() as i32;
                        let hp_bot = (sy_bot * 2.0).round() as i32;

                        if hp_top >= hp_bot {
                            // Zero-height band at this column — skip.
                            continue;
                        }

                        let cell_top = (hp_top.max(0) / 2) as u16;
                        let cell_bot = ((hp_bot - 1).max(0) / 2) as u16;

                        for cell_y in cell_top..=cell_bot {
                            if !pa.contains(screen_x, cell_y) {
                                continue;
                            }
                            let hp_cell_top = cell_y as i32 * 2;
                            let hp_cell_bot = hp_cell_top + 1;

                            let top_in = hp_top <= hp_cell_top && hp_bot > hp_cell_top;
                            let bot_in = hp_top <= hp_cell_bot && hp_bot > hp_cell_bot;

                            match (top_in, bot_in) {
                                (true, true) => {
                                    pb.set_bg(screen_x, cell_y, band_color, Z_FILL);
                                }
                                (true, false) => {
                                    pb.set_char(
                                        screen_x,
                                        cell_y,
                                        self.theme.chars.fill.half_upper,
                                        band_color,
                                        Z_FILL,
                                    );
                                }
                                (false, true) => {
                                    pb.set_char(
                                        screen_x,
                                        cell_y,
                                        self.theme.chars.fill.half_lower,
                                        band_color,
                                        Z_FILL,
                                    );
                                }
                                (false, false) => {}
                            }
                        }
                    }
                }
            }
        }

        // ── 8. Annotations, composite, labels ───────────────────────────
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);
        pb.composite(buf);
        frame.draw_end_labels(buf, area, &pa);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Approximate z-multiplier for a two-tailed confidence interval.
///
/// Uses a small lookup table for common CI levels, falling back to a rough
/// inverse-normal approximation for other values. This avoids pulling in a
/// full statistical distribution crate.
fn ci_z_multiplier(ci: f64) -> f64 {
    // Common exact-ish values.
    if (ci - 0.90).abs() < 1e-6 {
        return 1.645;
    }
    if (ci - 0.95).abs() < 1e-6 {
        return 1.96;
    }
    if (ci - 0.99).abs() < 1e-6 {
        return 2.576;
    }

    // Rational approximation (Abramowitz & Stegun 26.2.23) for the upper
    // tail of the standard normal.
    let alpha = 1.0 - ci;
    let p = 1.0 - alpha / 2.0;
    approx_normal_ppf(p)
}

/// Approximate inverse standard normal CDF for p in (0.5, 1).
fn approx_normal_ppf(p: f64) -> f64 {
    let p = p.clamp(0.5 + 1e-10, 1.0 - 1e-10);
    let t = (-2.0 * (1.0 - p).ln()).sqrt();
    const C0: f64 = 2.515_517;
    const C1: f64 = 0.802_853;
    const C2: f64 = 0.010_328;
    const D1: f64 = 1.432_788;
    const D2: f64 = 0.189_269;
    const D3: f64 = 0.001_308;
    t - (C0 + C1 * t + C2 * t * t) / (1.0 + D1 * t + D2 * t * t + D3 * t * t * t)
}

/// Produce a dimmed version of a color for the confidence band fill.
///
/// For RGB colors, blends toward the background (divides by ~2). For named
/// colors, returns a muted gray.
fn dim_color(c: Color) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb(r / 2, g / 2, b / 2),
        _ => Color::Rgb(80, 80, 100),
    }
}
