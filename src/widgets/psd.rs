//! Power Spectral Density (PSD) plot widget.
//!
//! Renders a PSD series as a line plot, typically with a logarithmic frequency axis.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::frame::{DataBounds, PlotArea, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendPosition};
use crate::series::{Series, is_valid_point};
use crate::spines::Spines;
use crate::style::DashPattern;
use crate::theme::Theme;

/// A Power Spectral Density plot widget.
///
/// Renders a PSD series (x = frequency, y = power in dB) as a line plot.
/// By default uses a linear x-axis and a linear y-axis (power is already in dB).
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::fft::psd;
/// use ratatui_plt::widgets::psd::PsdPlot;
///
/// let signal: Vec<f64> = (0..1024).map(|i| (i as f64 * 0.1).sin()).collect();
/// let series = psd(&signal, 44100.0);
/// let plot = PsdPlot::new()
///     .series(series)
///     .title("Power Spectral Density");
/// ```
pub struct PsdPlot {
    series: Vec<Series>,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    show_legend: bool,
    legend_position: LegendPosition,
}

impl Default for PsdPlot {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            show_legend: true,
            legend_position: LegendPosition::TopRight,
        }
    }
}

impl PsdPlot {
    /// Create an empty PSD plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a PSD data series.
    pub fn series(mut self, s: Series) -> Self {
        self.series.push(s);
        self
    }

    /// Add multiple series.
    pub fn series_vec(mut self, s: Vec<Series>) -> Self {
        self.series.extend(s);
        self
    }

    /// Set the plot title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the X axis (frequency axis).
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y axis (power axis, typically in dB).
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

impl Widget for &PsdPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (data_x_min, data_x_max) = self.compute_x_bounds();
        let (data_y_min, data_y_max) = self.compute_y_bounds();

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(data_x_min, data_x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(data_y_min, data_y_max);

        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, DataBounds { x_lo, x_hi, y_lo, y_hi }) else {
            return;
        };

        let clip = ClipRect::from_plot_area(&pa);

        // Draw line series
        for s in &self.series {
            if s.data.len() < 2 {
                for &(x, y) in &s.data {
                    let sx = pa.screen_x(x);
                    let sy = pa.screen_y(y);
                    let xi = sx.round() as u16;
                    let yi = sy.round() as u16;
                    if pa.contains(xi, yi) {
                        buf[(xi, yi)].set_char('\u{25cf}').set_fg(s.color);
                    }
                }
                continue;
            }

            for i in 0..s.data.len() - 1 {
                let (x0, y0) = s.data[i];
                let (x1, y1) = s.data[i + 1];

                if !is_valid_point(x0, y0) || !is_valid_point(x1, y1) {
                    continue;
                }

                let sx0 = pa.screen_x(x0);
                let sy0 = pa.screen_y(y0);
                let sx1 = pa.screen_x(x1);
                let sy1 = pa.screen_y(y1);
                draw_line(buf, &LineSegment { x0: sx0, y0: sy0, x1: sx1, y1: sy1 }, s.color, &s.line_style.pattern, &clip);
            }

            // Draw markers
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
                        buf[(xi, yi)].set_char(marker.char()).set_fg(s.color);
                    }
                }
            }
        }

        // Draw legend
        if self.show_legend && !self.series.is_empty() {
            let legend = Legend::from_series(&self.series)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(pa.x, pa.y, pa.width, pa.height);
            (&legend).render(legend_area, buf);
        }
    }
}

/// Clipping rectangle for line drawing.
struct ClipRect {
    x_min: u16,
    y_min: u16,
    x_max: u16,
    y_max: u16,
}

impl ClipRect {
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
const BRAILLE_BITS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
const BRAILLE_BASE: u32 = 0x2800;

/// OR a braille dot into the buffer cell, preserving existing dots.
fn write_braille(buf: &mut Buffer, x: u16, y: u16, bits: u8, color: Color) {
    let existing = {
        let ch = buf[(x, y)].symbol().chars().next().unwrap_or(' ');
        let code = ch as u32;
        if (BRAILLE_BASE..=0x28FF).contains(&code) {
            (code - BRAILLE_BASE) as u8
        } else {
            0
        }
    };
    let combined = existing | bits;
    if let Some(ch) = char::from_u32(BRAILLE_BASE + combined as u32) {
        buf[(x, y)].set_char(ch).set_fg(color);
    }
}

/// Screen-space line segment endpoints.
struct LineSegment {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

/// Draw a line between two screen points using Bresenham's at braille sub-pixel resolution.
fn draw_line(
    buf: &mut Buffer,
    seg: &LineSegment,
    color: Color,
    pattern: &DashPattern,
    clip: &ClipRect,
) {
    let LineSegment { x0, y0, x1, y1 } = *seg;
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
                write_braille(buf, cell_x, cell_y, bit, color);
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
