//! Smith chart widget for RF impedance visualization.
//!
//! Renders a Smith chart — the standard visualization for complex impedance
//! data in RF/microwave engineering. The chart maps the complex impedance
//! plane onto a unit circle in the reflection coefficient (Gamma) plane,
//! with overlaid constant-resistance circles and constant-reactance arcs.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::smith_chart::{SmithChart, SmithChartPoint};
//!
//! let chart = SmithChart::new()
//!     .point(SmithChartPoint::new(1.0, 0.5))
//!     .point(SmithChartPoint::new(0.5, -1.0).label("Load"))
//!     .show_trace(true)
//!     .title("S11");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::frame::PlotArea;
use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA, Z_GRID, Z_MARKER};
use crate::style::MarkerShape;
use crate::theme::Theme;

/// A single data point on the Smith chart.
#[derive(Clone, Debug)]
pub struct SmithChartPoint {
    /// Normalized resistance (real part of impedance, Z/Z0).
    pub r: f64,
    /// Normalized reactance (imaginary part of impedance, Z/Z0).
    pub x: f64,
    /// Optional label for this point.
    pub label: Option<String>,
    /// Optional color override.
    pub color: Option<Color>,
}

impl SmithChartPoint {
    /// Create a new point with given normalized resistance and reactance.
    pub fn new(r: f64, x: f64) -> Self {
        Self {
            r,
            x,
            label: None,
            color: None,
        }
    }

    /// Set a label for this point.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set a color override for this point.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Convert impedance to reflection coefficient (Gamma).
    ///
    /// Gamma = (Z - 1) / (Z + 1) where Z = r + jx (normalized to Z0 = 1).
    fn to_gamma(&self) -> (f64, f64) {
        let denom = (self.r + 1.0) * (self.r + 1.0) + self.x * self.x;
        if denom < 1e-15 {
            return (0.0, 0.0);
        }
        let gamma_real = (self.r * self.r + self.x * self.x - 1.0) / denom;
        let gamma_imag = 2.0 * self.x / denom;
        (gamma_real, gamma_imag)
    }
}

/// A Smith chart widget for impedance visualization.
///
/// Renders a unit circle with constant-resistance circles and
/// constant-reactance arcs, plus impedance data points and optional
/// connecting trace.
pub struct SmithChart {
    points: Vec<SmithChartPoint>,
    title: Option<String>,
    /// Whether to show connecting lines between points.
    show_trace: bool,
    marker: MarkerShape,
    /// Number of constant-resistance circles to draw.
    n_r_circles: usize,
    /// Number of constant-reactance arcs to draw.
    n_x_arcs: usize,
    theme: Theme,
}

impl Default for SmithChart {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            title: None,
            show_trace: false,
            marker: MarkerShape::FilledCircle,
            n_r_circles: 6,
            n_x_arcs: 6,
            theme: Theme::get_default(),
        }
    }
}

impl SmithChart {
    /// Create a new empty Smith chart.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a data point.
    pub fn point(mut self, point: SmithChartPoint) -> Self {
        self.points.push(point);
        self
    }

    /// Add multiple data points.
    pub fn points(mut self, pts: Vec<SmithChartPoint>) -> Self {
        self.points.extend(pts);
        self
    }

    /// Set whether to show trace lines connecting points.
    pub fn show_trace(mut self, show: bool) -> Self {
        self.show_trace = show;
        self
    }

    /// Set the marker shape for data points.
    pub fn marker(mut self, marker: MarkerShape) -> Self {
        self.marker = marker;
        self
    }

    /// Set the number of constant-resistance circles.
    pub fn n_r_circles(mut self, n: usize) -> Self {
        self.n_r_circles = n;
        self
    }

    /// Set the number of constant-reactance arcs.
    pub fn n_x_arcs(mut self, n: usize) -> Self {
        self.n_x_arcs = n;
        self
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

/// Map Gamma coordinates to screen coordinates.
///
/// Gamma_real in [-1, 1] maps to screen x, Gamma_imag in [-1, 1] maps to
/// screen y. The chart is drawn within a square region centered in the
/// available area, accounting for the ~2:1 terminal cell aspect ratio.
struct ChartGeometry {
    /// Center of the chart in screen coordinates.
    cx: f64,
    cy: f64,
    /// Radius in screen columns.
    radius_x: f64,
    /// Radius in screen rows.
    radius_y: f64,
    /// PlotArea for braille line clipping.
    pa: PlotArea,
}

impl ChartGeometry {
    fn gamma_to_screen(&self, gr: f64, gi: f64) -> (f64, f64) {
        let sx = self.cx + gr * self.radius_x;
        let sy = self.cy - gi * self.radius_y;
        (sx, sy)
    }
}

impl Widget for &SmithChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 12 || area.height < 8 {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let margin: u16 = 1;

        let py = area.y + title_height + margin;
        let ph = area.height.saturating_sub(title_height + margin * 2);
        let px = area.x + margin;
        let pw = area.width.saturating_sub(margin * 2);

        if ph < 4 || pw < 8 {
            return;
        }

        let mut pb = PlotBuffer::new(area);

        // Draw title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    pb.set_char(x, area.y, ch, self.theme.foreground, Z_CHROME);
                }
            }
        }

        // Compute chart geometry: fit a circle inside the available area,
        // accounting for the terminal cell aspect ratio (~2:1 height:width).
        let cell_aspect = 0.5; // height/width ratio of a terminal cell
        let avail_w = pw as f64;
        let avail_h = ph as f64;

        // The circle must fit in both dimensions. In screen columns, the
        // circle diameter is 2*radius_x. In screen rows, the circle
        // diameter is 2*radius_y. We need radius_x * cell_aspect = radius_y
        // (since each row is twice as tall as a column is wide).
        let radius_x = (avail_w / 2.0).min(avail_h / (2.0 * cell_aspect));
        let radius_y = radius_x * cell_aspect;

        let cx = px as f64 + avail_w / 2.0;
        let cy = py as f64 + avail_h / 2.0;

        let pa = PlotArea {
            x: px,
            y: py,
            width: pw,
            height: ph,
            x_lo: -1.0,
            x_hi: 1.0,
            y_lo: -1.0,
            y_hi: 1.0,
            area,
        };

        let geom = ChartGeometry {
            cx,
            cy,
            radius_x,
            radius_y,
            pa,
        };

        // Draw the unit circle boundary
        draw_circle_braille(&mut pb, &geom, 0.0, 0.0, 1.0, self.theme.axis_color, Z_CHROME);

        // Draw horizontal center line (real axis)
        {
            let (sx0, sy0) = geom.gamma_to_screen(-1.0, 0.0);
            let (sx1, sy1) = geom.gamma_to_screen(1.0, 0.0);
            pb.draw_line(sx0, sy0, sx1, sy1, self.theme.grid_color, &geom.pa, Z_GRID);
        }

        // Draw constant-resistance circles
        // Circle center in Gamma plane: (r/(r+1), 0), radius: 1/(r+1)
        let r_values: Vec<f64> = standard_r_values(self.n_r_circles);
        for &r in &r_values {
            let center_gr = r / (r + 1.0);
            let radius = 1.0 / (r + 1.0);
            draw_circle_braille(
                &mut pb,
                &geom,
                center_gr,
                0.0,
                radius,
                self.theme.grid_color,
                Z_GRID,
            );
        }

        // Draw constant-reactance arcs
        // Arc center in Gamma plane: (1, 1/x), radius: 1/|x|, clipped to unit circle
        let x_values: Vec<f64> = standard_x_values(self.n_x_arcs);
        for &xv in &x_values {
            if xv.abs() < 1e-12 {
                continue;
            }
            let center_gi = 1.0 / xv;
            let arc_radius = 1.0 / xv.abs();
            draw_arc_clipped(
                &mut pb,
                &geom,
                1.0,
                center_gi,
                arc_radius,
                self.theme.grid_color,
                Z_GRID,
            );
        }

        // Draw data trace
        let default_color = self.theme.primary;
        if self.show_trace && self.points.len() >= 2 {
            for i in 0..self.points.len() - 1 {
                let (gr0, gi0) = self.points[i].to_gamma();
                let (gr1, gi1) = self.points[i + 1].to_gamma();
                let (sx0, sy0) = geom.gamma_to_screen(gr0, gi0);
                let (sx1, sy1) = geom.gamma_to_screen(gr1, gi1);
                let color = self.points[i].color.unwrap_or(default_color);
                pb.draw_line(sx0, sy0, sx1, sy1, color, &geom.pa, Z_DATA);
            }
        }

        // Draw data points
        for pt in &self.points {
            let (gr, gi) = pt.to_gamma();
            // Only draw if inside the unit circle (valid impedance)
            if gr * gr + gi * gi > 1.05 {
                continue;
            }
            let (sx, sy) = geom.gamma_to_screen(gr, gi);
            let xi = sx.round() as u16;
            let yi = sy.round() as u16;
            let color = pt.color.unwrap_or(default_color);
            if geom.pa.contains(xi, yi) {
                pb.set_char(xi, yi, self.marker.char(), color, Z_MARKER);
            }

            // Draw label if present
            if let Some(ref label) = pt.label {
                let lx = xi.saturating_add(1);
                for (j, ch) in label.chars().enumerate() {
                    let x = lx + j as u16;
                    if x < area.x + area.width && yi < area.y + area.height {
                        pb.set_char(x, yi, ch, self.theme.foreground, Z_CHROME);
                    }
                }
            }
        }

        pb.composite(buf);
    }
}

/// Generate standard resistance values for constant-R circles.
fn standard_r_values(n: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    let standard = [0.0, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0];
    if n >= standard.len() {
        return standard.to_vec();
    }
    // Pick n evenly-spaced values from the standard set
    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        let idx = i * (standard.len() - 1) / (n - 1).max(1);
        result.push(standard[idx.min(standard.len() - 1)]);
    }
    result
}

/// Generate standard reactance values for constant-X arcs (positive and negative).
fn standard_x_values(n: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    let positive = [0.2, 0.5, 1.0, 2.0, 5.0, 10.0];
    let count = n.min(positive.len());
    let mut result = Vec::with_capacity(count * 2);
    for i in 0..count {
        let idx = i * (positive.len() - 1) / (count - 1).max(1);
        let v = positive[idx.min(positive.len() - 1)];
        result.push(v);
        result.push(-v);
    }
    result
}

/// Draw a circle in the Gamma plane using Braille line segments.
fn draw_circle_braille(
    pb: &mut PlotBuffer,
    geom: &ChartGeometry,
    center_gr: f64,
    center_gi: f64,
    radius: f64,
    color: Color,
    z: u8,
) {
    let n_segments = 64;
    let mut prev: Option<(f64, f64)> = None;
    for i in 0..=n_segments {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / n_segments as f64;
        let gr = center_gr + radius * theta.cos();
        let gi = center_gi + radius * theta.sin();

        // Clip to unit circle
        if gr * gr + gi * gi > 1.01 {
            prev = None;
            continue;
        }

        let (sx, sy) = geom.gamma_to_screen(gr, gi);
        if let Some((px, py)) = prev {
            pb.draw_line(px, py, sx, sy, color, &geom.pa, z);
        }
        prev = Some((sx, sy));
    }
}

/// Draw an arc in the Gamma plane, clipped to the unit circle.
fn draw_arc_clipped(
    pb: &mut PlotBuffer,
    geom: &ChartGeometry,
    center_gr: f64,
    center_gi: f64,
    radius: f64,
    color: Color,
    z: u8,
) {
    let n_segments = 64;
    let mut prev: Option<(f64, f64)> = None;
    for i in 0..=n_segments {
        let theta = 2.0 * std::f64::consts::PI * i as f64 / n_segments as f64;
        let gr = center_gr + radius * theta.cos();
        let gi = center_gi + radius * theta.sin();

        // Clip to unit circle
        if gr * gr + gi * gi > 1.01 {
            prev = None;
            continue;
        }

        let (sx, sy) = geom.gamma_to_screen(gr, gi);
        if let Some((px, py)) = prev {
            pb.draw_line(px, py, sx, sy, color, &geom.pa, z);
        }
        prev = Some((sx, sy));
    }
}
