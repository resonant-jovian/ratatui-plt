//! Confidence ellipse widget for 2D scatter data.
//!
//! Computes and draws a confidence ellipse based on the 2D covariance matrix
//! of input data. Uses the analytical eigendecomposition of the 2x2 covariance
//! matrix to determine ellipse orientation and semi-axes.
//!
//! # Example
//!
//! ```ignore
//! use ratatui_plt::widgets::confidence_ellipse::ConfidenceEllipse;
//! use ratatui_plt::prelude::*;
//!
//! let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
//! let y = vec![2.0, 3.5, 3.0, 5.0, 4.5];
//!
//! let plot = ConfidenceEllipse::new(x, y)
//!     .level(0.95)
//!     .title("95% Confidence Ellipse")
//!     .show_points(true);
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBackend, Z_DATA, Z_MARKER, create_backend};
use crate::spines::Spines;
use crate::style::MarkerShape;
use crate::theme::Theme;

/// A 2D confidence ellipse widget overlaying scatter data.
///
/// Computes the covariance ellipse at a given confidence level and draws
/// both the scatter points and the ellipse boundary.
pub struct ConfidenceEllipse {
    x: Vec<f64>,
    y: Vec<f64>,
    /// Confidence level in (0, 1). Default: 0.95.
    level: f64,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    /// Whether to show individual scatter points. Default: true.
    show_points: bool,
    /// Color override for the ellipse boundary.
    ellipse_color: Option<Color>,
    /// Color override for scatter points.
    point_color: Option<Color>,
    /// Marker shape for scatter points.
    marker: MarkerShape,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
}

impl ConfidenceEllipse {
    /// Create a new confidence ellipse from x and y data vectors.
    pub fn new(x: Vec<f64>, y: Vec<f64>) -> Self {
        Self {
            x,
            y,
            level: 0.95,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_points: true,
            ellipse_color: None,
            point_color: None,
            marker: MarkerShape::Dot,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
        }
    }

    /// Set the confidence level (0..1). Default: 0.95.
    pub fn level(mut self, level: f64) -> Self {
        self.level = level.clamp(0.01, 0.999);
        self
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

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Whether to show individual scatter points. Default: true.
    pub fn show_points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    /// Set the ellipse boundary color.
    pub fn ellipse_color(mut self, color: Color) -> Self {
        self.ellipse_color = Some(color);
        self
    }

    /// Set the scatter point color.
    pub fn point_color(mut self, color: Color) -> Self {
        self.point_color = Some(color);
        self
    }

    /// Set the scatter point marker shape.
    pub fn marker(mut self, marker: MarkerShape) -> Self {
        self.marker = marker;
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
}

/// Compute mean of a slice of finite values.
fn mean(data: &[f64]) -> f64 {
    let (sum, count) = data
        .iter()
        .filter(|v| v.is_finite())
        .fold((0.0, 0usize), |(s, c), &v| (s + v, c + 1));
    if count == 0 { 0.0 } else { sum / count as f64 }
}

/// Compute 2x2 covariance matrix elements from x and y data.
/// Returns (var_x, cov_xy, var_y).
fn covariance_2d(x: &[f64], y: &[f64]) -> (f64, f64, f64) {
    let n = x.len().min(y.len());
    if n < 2 {
        return (1.0, 0.0, 1.0);
    }

    let mx = mean(x);
    let my = mean(y);

    let mut var_x = 0.0;
    let mut var_y = 0.0;
    let mut cov_xy = 0.0;
    let mut count = 0usize;

    for i in 0..n {
        if x[i].is_finite() && y[i].is_finite() {
            let dx = x[i] - mx;
            let dy = y[i] - my;
            var_x += dx * dx;
            var_y += dy * dy;
            cov_xy += dx * dy;
            count += 1;
        }
    }

    if count < 2 {
        return (1.0, 0.0, 1.0);
    }

    let denom = (count - 1) as f64;
    (var_x / denom, cov_xy / denom, var_y / denom)
}

/// Compute eigenvalues and rotation angle of a 2x2 symmetric matrix [[a, b], [b, c]].
/// Returns (eigenvalue1, eigenvalue2, rotation_angle_radians).
fn eigen_2x2(a: f64, b: f64, c: f64) -> (f64, f64, f64) {
    let trace = a + c;
    let det = a * c - b * b;

    // Eigenvalues of a 2x2 symmetric matrix: (trace +/- sqrt(trace^2 - 4*det)) / 2
    let discriminant = (trace * trace - 4.0 * det).max(0.0);
    let sqrt_disc = discriminant.sqrt();

    let lambda1 = (trace + sqrt_disc) / 2.0;
    let lambda2 = (trace - sqrt_disc) / 2.0;

    // Rotation angle from eigenvector of the larger eigenvalue
    let theta = if b.abs() < 1e-15 {
        if a >= c {
            0.0
        } else {
            std::f64::consts::FRAC_PI_2
        }
    } else {
        (lambda1 - a).atan2(b)
    };

    (lambda1.max(0.0), lambda2.max(0.0), theta)
}

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for ConfidenceEllipse {
    fn render_plotters(
        &self,
        area: ratatui::layout::Rect,
        buf: &mut ratatui::buffer::Buffer,
        theme: &crate::theme::Theme,
    ) {
        use crate::plotters_render::{bridge, theme_bridge};

        bridge::render_plotters_to_buf(
            area,
            buf,
            theme_bridge::theme_bg_rgb(theme),
            |_root| {
                // Minimal implementation - full plotters rendering TBD.
            },
        );
    }
}

impl Widget for &ConfidenceEllipse {
    fn render(self, area: Rect, buf: &mut Buffer) {
        #[cfg(feature = "plotters-render")]
        {
            if crate::plotters_render::should_use_plotters() {
                use crate::plotters_render::PlottersRenderable;
                self.render_plotters(area, buf, &self.theme);
                return;
            }
        }
        if self.x.is_empty() || self.y.is_empty() {
            return;
        }

        let n = self.x.len().min(self.y.len());

        // Compute data bounds
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for i in 0..n {
            if self.x[i].is_finite() && self.y[i].is_finite() {
                x_min = x_min.min(self.x[i]);
                x_max = x_max.max(self.x[i]);
                y_min = y_min.min(self.y[i]);
                y_max = y_max.max(self.y[i]);
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

        // Compute ellipse parameters to extend bounds if needed
        let cx = mean(&self.x[..n]);
        let cy = mean(&self.y[..n]);
        let (var_x, cov_xy, var_y) = covariance_2d(&self.x[..n], &self.y[..n]);
        let (lambda1, lambda2, theta) = eigen_2x2(var_x, cov_xy, var_y);

        // Chi-squared threshold for the given confidence level (2 DOF):
        // chi2 = -2 * ln(1 - level)
        let chi2_thresh = -2.0 * (1.0 - self.level).max(1e-15).ln();

        let semi_a = (lambda1 * chi2_thresh).sqrt();
        let semi_b = (lambda2 * chi2_thresh).sqrt();

        // Extend data bounds to contain the ellipse
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        // Bounding box of the rotated ellipse
        let ell_half_w = ((semi_a * cos_t).powi(2) + (semi_b * sin_t).powi(2)).sqrt();
        let ell_half_h = ((semi_a * sin_t).powi(2) + (semi_b * cos_t).powi(2)).sqrt();

        x_min = x_min.min(cx - ell_half_w);
        x_max = x_max.max(cx + ell_half_w);
        y_min = y_min.min(cy - ell_half_h);
        y_max = y_max.max(cy + ell_half_h);

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let mut pb = create_backend(area);

        // Render plot frame
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

        // Draw scatter points if enabled
        if self.show_points {
            let pt_color = self.point_color.unwrap_or(self.theme.primary);
            for i in 0..n {
                if !self.x[i].is_finite() || !self.y[i].is_finite() {
                    continue;
                }
                let sx = pa.screen_x(self.x[i]).round() as u16;
                let sy = pa.screen_y(self.y[i]).round() as u16;
                if pa.contains(sx, sy) {
                    pb.set_char(sx, sy, self.marker.char(), pt_color, Z_MARKER);
                }
            }
        }

        // Draw ellipse boundary using Braille lines
        let ellipse_color = self.ellipse_color.unwrap_or(self.theme.accent);
        let n_points = 100;

        // Generate ellipse boundary points
        let mut ellipse_pts: Vec<(f64, f64)> = Vec::with_capacity(n_points + 1);
        for k in 0..=n_points {
            let t = 2.0 * std::f64::consts::PI * k as f64 / n_points as f64;
            let ex = cx + semi_a * t.cos() * cos_t - semi_b * t.sin() * sin_t;
            let ey = cy + semi_a * t.cos() * sin_t + semi_b * t.sin() * cos_t;
            ellipse_pts.push((ex, ey));
        }

        // Draw line segments between consecutive ellipse points
        for i in 0..ellipse_pts.len() - 1 {
            let (x0, y0) = ellipse_pts[i];
            let (x1, y1) = ellipse_pts[i + 1];

            let sx0 = pa.screen_x(x0);
            let sy0 = pa.screen_y(y0);
            let sx1 = pa.screen_x(x1);
            let sy1 = pa.screen_y(y1);

            pb.draw_line(sx0, sy0, sx1, sy1, ellipse_color, &pa, Z_DATA);
        }

        pb.composite(buf);
    }
}
