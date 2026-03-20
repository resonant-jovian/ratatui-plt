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
use crate::frame::{PlotFrame, ReferenceLine};
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

        let Some(pa) = frame.render(area, buf, x_lo, x_hi, y_lo, y_hi) else {
            return;
        };

        // Draw each band
        for band in &self.bands {
            let n = band.x.len().min(band.y_lower.len()).min(band.y_upper.len());

            // Fill the region between y_lower and y_upper for each x
            for i in 0..n {
                let x = band.x[i];
                let yl = band.y_lower[i];
                let yu = band.y_upper[i];

                if !x.is_finite() || !yl.is_finite() || !yu.is_finite() {
                    continue;
                }

                let sx = pa.screen_x(x).round() as u16;
                let sy_lower = pa.screen_y(yl).round() as u16;
                let sy_upper = pa.screen_y(yu).round() as u16;

                // screen_y inverts: lower data y -> higher screen y
                let y_top = sy_upper.min(sy_lower);
                let y_bot = sy_upper.max(sy_lower);

                if sx >= pa.x && sx < pa.x + pa.width {
                    for y in y_top..=y_bot {
                        if pa.contains(sx, y) {
                            buf[(sx, y)].set_char(band.alpha_char).set_fg(band.color);
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

/// Draw a boundary line segment between two data points using Bresenham's algorithm.
fn draw_boundary_line(
    buf: &mut Buffer,
    pa: &crate::frame::PlotArea,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
) {
    let sx0 = pa.screen_x(x0).round() as i32;
    let sy0 = pa.screen_y(y0).round() as i32;
    let sx1 = pa.screen_x(x1).round() as i32;
    let sy1 = pa.screen_y(y1).round() as i32;

    let dx = (sx1 - sx0).abs();
    let dy = -(sy1 - sy0).abs();
    let step_x = if sx0 < sx1 { 1 } else { -1 };
    let step_y = if sy0 < sy1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut cx = sx0;
    let mut cy = sy0;

    loop {
        let ux = cx as u16;
        let uy = cy as u16;
        if pa.contains(ux, uy) {
            // Choose line character based on direction
            let ch = if dx > dy.abs() { '─' } else { '│' };
            buf[(ux, uy)].set_char(ch).set_fg(color);
        }

        if cx == sx1 && cy == sy1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            cx += step_x;
        }
        if e2 <= dx {
            err += dx;
            cy += step_y;
        }
    }
}
