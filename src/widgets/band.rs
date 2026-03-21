//! Band (fill-between) plot widget.
//!
//! Renders filled regions between two y-value arrays, useful for showing
//! confidence intervals, error bands, or shaded areas between curves.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::band::Band;
//!
//! let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
//! let y_lower: Vec<f64> = x.iter().map(|&v| v.sin() - 0.3).collect();
//! let y_upper: Vec<f64> = x.iter().map(|&v| v.sin() + 0.3).collect();
//!
//! let plot = BandPlot::new()
//!     .band(Band::new("confidence", x, y_lower, y_upper).color(Color::Cyan))
//!     .title("Confidence Band")
//!     .x_axis(Axis::new().label("x"))
//!     .y_axis(Axis::new().label("y"));
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::spines::Spines;
use crate::theme::Theme;

/// A single filled band between two y-value arrays.
#[derive(Clone, Debug)]
pub struct Band {
    /// Display name for the legend.
    pub name: String,
    /// X coordinates.
    pub x: Vec<f64>,
    /// Lower y boundary values (one per x).
    pub y_lower: Vec<f64>,
    /// Upper y boundary values (one per x).
    pub y_upper: Vec<f64>,
    /// Band color.
    pub color: Color,
    /// Fill character: '\u{2591}' (light), '\u{2592}' (medium), '\u{2593}' (dense).
    pub alpha_char: char,
}

impl Band {
    /// Create a new band with the given name, x coordinates, and y boundaries.
    pub fn new(name: impl Into<String>, x: Vec<f64>, y_lower: Vec<f64>, y_upper: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            x,
            y_lower,
            y_upper,
            color: Color::Cyan,
            alpha_char: '\u{2591}', // ░
        }
    }

    /// Set the band color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the fill character for the band interior.
    ///
    /// Common choices: '\u{2591}' (░ light), '\u{2592}' (▒ medium), '\u{2593}' (▓ dense).
    pub fn alpha_char(mut self, ch: char) -> Self {
        self.alpha_char = ch;
        self
    }
}

/// A band (fill-between) plot widget.
///
/// Renders one or more filled regions between pairs of y-value arrays,
/// with boundary lines drawn along the edges.
#[derive(Clone)]
pub struct BandPlot {
    bands: Vec<Band>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_legend: bool,
    legend_position: LegendPosition,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for BandPlot {
    fn default() -> Self {
        Self {
            bands: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_legend: true,
            legend_position: LegendPosition::TopRight,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl BandPlot {
    /// Create an empty band plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a band.
    pub fn band(mut self, b: Band) -> Self {
        self.bands.push(b);
        self
    }

    /// Add multiple bands.
    pub fn bands(mut self, bands: Vec<Band>) -> Self {
        self.bands.extend(bands);
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

impl Widget for &BandPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Compute data bounds across all bands
        let (data_x_min, data_x_max, data_y_min, data_y_max) = self.compute_bounds();

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(data_x_min, data_x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(data_y_min, data_y_max);

        // Create and render the plot frame
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, DataBounds { x_lo, x_hi, y_lo, y_hi }) else {
            return;
        };

        // Draw each band
        for band in &self.bands {
            let n = band.x.len().min(band.y_lower.len()).min(band.y_upper.len());

            // Fill the region between y_lower and y_upper using column interpolation.
            // Instead of filling only at data point x-coordinates, iterate over every
            // screen column and interpolate the bounds for gap-free rendering.
            if n >= 2 {
                for col_offset in 0..pa.width {
                    let screen_x = pa.x + col_offset;
                    // Map screen column to data x coordinate
                    let data_x = pa.x_lo
                        + (col_offset as f64 / (pa.width - 1).max(1) as f64)
                            * (pa.x_hi - pa.x_lo);

                    // Find the data segment containing this x and interpolate
                    let yl_interp = interpolate_at(&band.x[..n], &band.y_lower[..n], data_x);
                    let yu_interp = interpolate_at(&band.x[..n], &band.y_upper[..n], data_x);

                    if let (Some(yl), Some(yu)) = (yl_interp, yu_interp) {
                        let sy_lower = pa.screen_y(yl).round() as u16;
                        let sy_upper = pa.screen_y(yu).round() as u16;

                        let y_top = sy_upper.min(sy_lower);
                        let y_bot = sy_upper.max(sy_lower);

                        for y in y_top..=y_bot {
                            if pa.contains(screen_x, y) {
                                buf[(screen_x, y)]
                                    .set_char(band.alpha_char)
                                    .set_fg(band.color);
                            }
                        }
                    }
                }
            }

            // Draw boundary lines (upper and lower edges)
            for i in 0..n.saturating_sub(1) {
                let x0 = band.x[i];
                let x1 = band.x[i + 1];

                // Upper boundary
                let yu0 = band.y_upper[i];
                let yu1 = band.y_upper[i + 1];
                if x0.is_finite() && x1.is_finite() && yu0.is_finite() && yu1.is_finite() {
                    draw_boundary_line(buf, &pa, x0, yu0, x1, yu1, band.color);
                }

                // Lower boundary
                let yl0 = band.y_lower[i];
                let yl1 = band.y_lower[i + 1];
                if x0.is_finite() && x1.is_finite() && yl0.is_finite() && yl1.is_finite() {
                    draw_boundary_line(buf, &pa, x0, yl0, x1, yl1, band.color);
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations(&pa, &self.annotations, buf);

        // Draw legend
        if self.show_legend && !self.bands.is_empty() {
            let entries: Vec<LegendEntry> = self
                .bands
                .iter()
                .map(|b| LegendEntry {
                    name: b.name.clone(),
                    color: b.color,
                    marker: Some(b.alpha_char),
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

impl BandPlot {
    fn compute_bounds(&self) -> (f64, f64, f64, f64) {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for band in &self.bands {
            for &x in &band.x {
                if x.is_finite() {
                    x_min = x_min.min(x);
                    x_max = x_max.max(x);
                }
            }
            for &y in &band.y_lower {
                if y.is_finite() {
                    y_min = y_min.min(y);
                    y_max = y_max.max(y);
                }
            }
            for &y in &band.y_upper {
                if y.is_finite() {
                    y_min = y_min.min(y);
                    y_max = y_max.max(y);
                }
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

        (x_min, x_max, y_min, y_max)
    }
}

/// Linearly interpolate a y value at a given x from sorted (xs, ys) arrays.
/// Returns `None` if x is outside the data range.
fn interpolate_at(xs: &[f64], ys: &[f64], x: f64) -> Option<f64> {
    if xs.len() < 2 || x < xs[0] || x > xs[xs.len() - 1] {
        return None;
    }
    // Binary search for the segment containing x
    let idx = match xs.binary_search_by(|v| v.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal))
    {
        Ok(i) => return Some(ys[i]),
        Err(i) => i,
    };
    if idx == 0 || idx >= xs.len() {
        return None;
    }
    let x0 = xs[idx - 1];
    let x1 = xs[idx];
    let y0 = ys[idx - 1];
    let y1 = ys[idx];
    if (x1 - x0).abs() < f64::EPSILON {
        return Some(y0);
    }
    let t = (x - x0) / (x1 - x0);
    Some(y0 + t * (y1 - y0))
}

/// Draw a boundary line segment between two data points using Braille sub-pixel rendering.
fn draw_boundary_line(
    buf: &mut Buffer,
    pa: &crate::frame::PlotArea,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
) {
    let sx0 = pa.screen_x(x0);
    let sy0 = pa.screen_y(y0);
    let sx1 = pa.screen_x(x1);
    let sy1 = pa.screen_y(y1);
    crate::drawing::draw_braille_line(buf, sx0, sy0, sx1, sy1, color, pa);
}
