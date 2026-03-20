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
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis};
use crate::legend::{Legend, LegendPosition};
use crate::series::{Series, is_valid_point};
use crate::style::DashPattern;
use crate::theme::Theme;
use crate::transform::{apply_aspect_ratio, data_to_screen};

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
}

impl Widget for &LinePlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        // Reserve space for title, axis labels, and tick labels
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let x_label_height: u16 = if self.x_axis.label.is_some() { 1 } else { 0 };
        let y_label_width: u16 = 8; // Space for y-axis tick labels
        let tick_height: u16 = 1;

        let plot_x = area.x + y_label_width;
        let plot_y = area.y + title_height;
        let plot_width = area.width.saturating_sub(y_label_width + 1);
        let plot_height = area
            .height
            .saturating_sub(title_height + tick_height + x_label_height);

        if plot_width < 2 || plot_height < 2 {
            return;
        }

        // Compute data bounds
        let (data_x_min, data_x_max) = self.compute_x_bounds();
        let (data_y_min, data_y_max) = self.compute_y_bounds();

        let (x_min, x_max) = self.x_axis.resolve_bounds(data_x_min, data_x_max);
        let (y_min, y_max) = self.y_axis.resolve_bounds(data_y_min, data_y_max);

        // Apply aspect ratio
        let (ax_off, ay_off, aw, ah) = apply_aspect_ratio(
            &self.aspect_ratio,
            x_max - x_min,
            y_max - y_min,
            plot_width,
            plot_height,
        );
        let px = plot_x + ax_off;
        let py = plot_y + ay_off;

        // Draw title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)]
                        .set_char(ch)
                        .set_style(Style::default().fg(self.theme.foreground));
                }
            }
        }

        // Draw axes border
        for x in px..px + aw {
            if x < area.x + area.width {
                buf[(x, py + ah)]
                    .set_char('─')
                    .set_fg(self.theme.axis_color);
            }
        }
        for y in py..py + ah {
            buf[(px.saturating_sub(1), y)]
                .set_char('│')
                .set_fg(self.theme.axis_color);
        }

        // Draw grid lines
        let x_grid = self.x_axis.grid || self.theme.grid_visible;
        let y_grid = self.y_axis.grid || self.theme.grid_visible;
        if x_grid {
            let x_ticks = self.x_axis.tick_positions(x_min, x_max);
            for &tv in &x_ticks {
                let sx = data_to_screen(tv, x_min, x_max, px as f64, (px + aw - 1) as f64);
                let xi = sx.round() as u16;
                if xi >= px && xi < px + aw {
                    for y in py..py + ah {
                        buf[(xi, y)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }
        if y_grid {
            let y_ticks = self.y_axis.tick_positions(y_min, y_max);
            for &tv in &y_ticks {
                let sy = data_to_screen(tv, y_min, y_max, (py + ah - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ah {
                    for x in px..px + aw {
                        buf[(x, yi)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }

        // Draw tick labels
        let x_ticks = self.x_axis.tick_positions(x_min, x_max);
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_min, x_max, px as f64, (px + aw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let label_start = xi.saturating_sub(label.len() as u16 / 2);
            let y = py + ah;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }

        let y_ticks = self.y_axis.tick_positions(y_min, y_max);
        for &tv in &y_ticks {
            let sy = data_to_screen(tv, y_min, y_max, (py + ah - 1) as f64, py as f64);
            let label = self.y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            let label_end = px.saturating_sub(2);
            if yi >= py && yi < py + ah {
                let label_start = label_end.saturating_sub(label.len() as u16);
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16 + 1;
                    if lx < px && lx >= area.x {
                        buf[(lx, yi)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }

        // Draw axis labels
        if let Some(ref label) = self.x_axis.label {
            let y = area.y + area.height - 1;
            let start = px + (aw.saturating_sub(label.len() as u16)) / 2;
            for (i, ch) in label.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width && y < area.y + area.height {
                    buf[(x, y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        if let Some(ref label) = self.y_axis.label {
            // Draw vertically on the left
            let x = area.x;
            let start_y = py + (ah.saturating_sub(label.len() as u16)) / 2;
            for (i, ch) in label.chars().enumerate() {
                let y = start_y + i as u16;
                if y < py + ah {
                    buf[(x, y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        // Draw fill regions
        for s in &self.series {
            if let Some(ref fill_to) = s.fill_to {
                let baseline = match fill_to {
                    crate::series::FillTo::Baseline(y) => *y,
                    _ => y_min,
                };
                let baseline_screen =
                    data_to_screen(baseline, y_min, y_max, (py + ah - 1) as f64, py as f64);

                for point in &s.data {
                    let sx = data_to_screen(point.0, x_min, x_max, px as f64, (px + aw - 1) as f64);
                    let sy = data_to_screen(point.1, y_min, y_max, (py + ah - 1) as f64, py as f64);
                    let xi = sx.round() as u16;
                    let y_top = sy.round().min(baseline_screen.round()) as u16;
                    let y_bot = sy.round().max(baseline_screen.round()) as u16;

                    if xi >= px && xi < px + aw {
                        for y in y_top..=y_bot {
                            if y >= py && y < py + ah {
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
                    let sx = data_to_screen(x, x_min, x_max, px as f64, (px + aw - 1) as f64);
                    let xi = sx.round() as u16;
                    if xi < px || xi >= px + aw {
                        continue;
                    }

                    let lo = y - s.y_err_low.as_ref().map_or(0.0, |e| e[i]);
                    let hi = y + s.y_err_high.as_ref().map_or(0.0, |e| e[i]);

                    let sy_lo = data_to_screen(lo, y_min, y_max, (py + ah - 1) as f64, py as f64);
                    let sy_hi = data_to_screen(hi, y_min, y_max, (py + ah - 1) as f64, py as f64);

                    let y_top = sy_hi.round() as u16;
                    let y_bot = sy_lo.round() as u16;

                    for ey in y_top..=y_bot {
                        if ey >= py && ey < py + ah {
                            buf[(xi, ey)].set_char('│').set_fg(s.color);
                        }
                    }
                    // Caps
                    if y_top >= py && y_top < py + ah {
                        buf[(xi, y_top)].set_char('┬').set_fg(s.color);
                    }
                    if y_bot >= py && y_bot < py + ah {
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
                    let sx = data_to_screen(x, x_min, x_max, px as f64, (px + aw - 1) as f64);
                    let sy = data_to_screen(y, y_min, y_max, (py + ah - 1) as f64, py as f64);
                    let xi = sx.round() as u16;
                    let yi = sy.round() as u16;
                    if xi >= px && xi < px + aw && yi >= py && yi < py + ah {
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

                let sx0 = data_to_screen(x0, x_min, x_max, px as f64, (px + aw - 1) as f64);
                let sy0 = data_to_screen(y0, y_min, y_max, (py + ah - 1) as f64, py as f64);
                let sx1 = data_to_screen(x1, x_min, x_max, px as f64, (px + aw - 1) as f64);
                let sy1 = data_to_screen(y1, y_min, y_max, (py + ah - 1) as f64, py as f64);

                draw_line(
                    buf,
                    sx0,
                    sy0,
                    sx1,
                    sy1,
                    s.color,
                    &s.line_style.pattern,
                    &ClipRect {
                        x_min: px,
                        y_min: py,
                        x_max: px + aw,
                        y_max: py + ah,
                    },
                );
            }

            // Draw markers (skip NaN points)
            if let Some(marker) = s.marker {
                for &(x, y) in &s.data {
                    if !is_valid_point(x, y) {
                        continue;
                    }
                    let sx = data_to_screen(x, x_min, x_max, px as f64, (px + aw - 1) as f64);
                    let sy = data_to_screen(y, y_min, y_max, (py + ah - 1) as f64, py as f64);
                    let xi = sx.round() as u16;
                    let yi = sy.round() as u16;
                    if xi >= px && xi < px + aw && yi >= py && yi < py + ah {
                        buf[(xi, yi)].set_char(marker.char()).set_fg(s.color);
                    }
                }
            }
        }

        // Draw annotations
        for ann in &self.annotations {
            let sx = data_to_screen(ann.text_x, x_min, x_max, px as f64, (px + aw - 1) as f64);
            let sy = data_to_screen(ann.text_y, y_min, y_max, (py + ah - 1) as f64, py as f64);
            let xi = sx.round() as u16;
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ah {
                for (j, ch) in ann.text.chars().enumerate() {
                    let x = xi + j as u16;
                    if x >= px && x < px + aw {
                        buf[(x, yi)].set_char(ch).set_fg(ann.color);
                    }
                }
            }
        }

        // Draw legend
        if self.show_legend && !self.series.is_empty() {
            let legend = Legend::from_series(&self.series)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(px, py, aw, ah);
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

#[allow(clippy::too_many_arguments)]
/// Draw a line between two screen points using Bresenham's algorithm.
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
    let mut ix0 = x0.round() as i32;
    let mut iy0 = y0.round() as i32;
    let ix1 = x1.round() as i32;
    let iy1 = y1.round() as i32;

    let dx = (ix1 - ix0).abs();
    let dy = -(iy1 - iy0).abs();
    let sx = if ix0 < ix1 { 1 } else { -1 };
    let sy = if iy0 < iy1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut step = 0u32;

    loop {
        // Compute step direction for local character selection
        let e2 = 2 * err;
        let stepped_x = e2 >= dy;
        let stepped_y = e2 <= dx;

        let draw = match pattern {
            DashPattern::Solid => true,
            DashPattern::Dashed => (step / 3).is_multiple_of(2),
            DashPattern::Dotted => step.is_multiple_of(2),
            DashPattern::DashDot => {
                let cycle = step % 5;
                cycle < 3 || cycle == 4
            }
        };

        if draw {
            let px = ix0 as u16;
            let py = iy0 as u16;
            if px >= clip.x_min && px < clip.x_max && py >= clip.y_min && py < clip.y_max {
                let ch = match (stepped_x, stepped_y) {
                    (true, false) => '─',
                    (false, true) => '│',
                    (true, true) if (sx > 0) == (sy > 0) => '╲',
                    (true, true) => '╱',
                    _ => '·',
                };
                buf[(px, py)].set_char(ch).set_fg(color);
            }
        }

        if ix0 == ix1 && iy0 == iy1 {
            break;
        }
        if stepped_x {
            err += dy;
            ix0 += sx;
        }
        if stepped_y {
            err += dx;
            iy0 += sy;
        }
        step += 1;
    }
}
