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

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis};
use crate::frame::{PlotArea, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendPosition};
use crate::series::{Series, is_valid_point};
use crate::spines::Spines;
use crate::style::DashPattern;
use crate::theme::Theme;

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
    annotations: Vec<Annotation>,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
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
            annotations: Vec::new(),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
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
}

impl Widget for &LinePlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Compute data bounds
        let (data_x_min, data_x_max) = self.compute_x_bounds();
        let (data_y_min, data_y_max) = self.compute_y_bounds();

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(data_x_min, data_x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(data_y_min, data_y_max);

        // Create and render the plot frame (title, axes, grid, ticks, labels, spines, ref lines)
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .aspect_ratio(self.aspect_ratio.clone())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, x_lo, x_hi, y_lo, y_hi) else {
            return;
        };

        let clip = ClipRect::from_plot_area(&pa);

        // Draw fill regions
        for s in &self.series {
            if let Some(ref fill_to) = s.fill_to {
                let baseline = match fill_to {
                    crate::series::FillTo::Baseline(y) => *y,
                    _ => y_lo,
                };
                let baseline_screen = pa.screen_y(baseline);

                for point in &s.data {
                    let sx = pa.screen_x(point.0);
                    let sy = pa.screen_y(point.1);
                    let xi = sx.round() as u16;
                    let y_top = sy.round().min(baseline_screen.round()) as u16;
                    let y_bot = sy.round().max(baseline_screen.round()) as u16;

                    if xi >= pa.x && xi < pa.x + pa.width {
                        for y in y_top..=y_bot {
                            if pa.contains(xi, y) {
                                buf[(xi, y)].set_char('░').set_fg(s.color);
                            }
                        }
                    }
                }
            }
        }

        // Draw error bars
        for s in &self.series {
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
                            buf[(xi, ey)].set_char('│').set_fg(s.color);
                        }
                    }
                    // Caps
                    if pa.contains(xi, y_top) {
                        buf[(xi, y_top)].set_char('┬').set_fg(s.color);
                    }
                    if pa.contains(xi, y_bot) {
                        buf[(xi, y_bot)].set_char('┴').set_fg(s.color);
                    }
                }
            }
        }

        // Draw line series
        for s in &self.series {
            if s.data.len() < 2 {
                // Just draw markers for single-point series
                for &(x, y) in &s.data {
                    let sx = pa.screen_x(x);
                    let sy = pa.screen_y(y);
                    let xi = sx.round() as u16;
                    let yi = sy.round() as u16;
                    if pa.contains(xi, yi) {
                        let ch = s.marker.map_or('●', |m| m.char());
                        buf[(xi, yi)].set_char(ch).set_fg(s.color);
                    }
                }
                continue;
            }

            // Draw lines between consecutive points, breaking at NaN
            for i in 0..s.data.len() - 1 {
                let (x0, y0) = s.data[i];
                let (x1, y1) = s.data[i + 1];

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
                        draw_line(
                            buf,
                            sx0,
                            sy0,
                            sx1,
                            sy1,
                            s.color,
                            &s.line_style.pattern,
                            &clip,
                        );
                    }
                    StepMode::Pre => {
                        // Horizontal then vertical: (x0,y0) -> (x1,y0) -> (x1,y1)
                        let sx0 = pa.screen_x(x0);
                        let sy0 = pa.screen_y(y0);
                        let sx1 = pa.screen_x(x1);
                        let sy1 = pa.screen_y(y1);
                        // Horizontal segment at y0
                        draw_line(
                            buf,
                            sx0,
                            sy0,
                            sx1,
                            sy0,
                            s.color,
                            &s.line_style.pattern,
                            &clip,
                        );
                        // Vertical segment at x1
                        draw_line(
                            buf,
                            sx1,
                            sy0,
                            sx1,
                            sy1,
                            s.color,
                            &s.line_style.pattern,
                            &clip,
                        );
                    }
                    StepMode::Post => {
                        // Vertical then horizontal: (x0,y0) -> (x0,y1) -> (x1,y1)
                        let sx0 = pa.screen_x(x0);
                        let sy0 = pa.screen_y(y0);
                        let sx1 = pa.screen_x(x1);
                        let sy1 = pa.screen_y(y1);
                        // Vertical segment at x0
                        draw_line(
                            buf,
                            sx0,
                            sy0,
                            sx0,
                            sy1,
                            s.color,
                            &s.line_style.pattern,
                            &clip,
                        );
                        // Horizontal segment at y1
                        draw_line(
                            buf,
                            sx0,
                            sy1,
                            sx1,
                            sy1,
                            s.color,
                            &s.line_style.pattern,
                            &clip,
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
                        draw_line(
                            buf,
                            sx0,
                            sy0,
                            smx,
                            sy0,
                            s.color,
                            &s.line_style.pattern,
                            &clip,
                        );
                        // Vertical at mid from y0 to y1
                        draw_line(
                            buf,
                            smx,
                            sy0,
                            smx,
                            sy1,
                            s.color,
                            &s.line_style.pattern,
                            &clip,
                        );
                        // Horizontal at y1 from mid to x1
                        draw_line(
                            buf,
                            smx,
                            sy1,
                            sx1,
                            sy1,
                            s.color,
                            &s.line_style.pattern,
                            &clip,
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
                        buf[(xi, yi)].set_char(marker.char()).set_fg(s.color);
                    }
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations(&pa, &self.annotations, buf);

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

#[allow(clippy::too_many_arguments)]
/// Draw a line between two screen points using Bresenham's at braille sub-pixel resolution (2x4 per cell).
fn draw_line(
    buf: &mut Buffer,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
    pattern: &DashPattern,
    clip: &ClipRect,
) {
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
