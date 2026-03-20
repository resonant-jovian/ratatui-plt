//! Twin axes widget for dual y-axis plots.
//!
//! Allows overlaying two plots with different y-axis scales on the same area.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::series::Series;
use crate::transform::data_to_screen;

/// A dual y-axis plot that overlays two sets of series with independent y scales.
///
/// # Example
///
/// ```
/// use ratatui_sim::prelude::*;
/// use ratatui_sim::widgets::twin_axes::TwinAxes;
///
/// let plot = TwinAxes::new()
///     .primary(Series::new("Temperature").data(vec![(0.0, 20.0), (1.0, 25.0)]).color(Color::Red))
///     .secondary(Series::new("Pressure").data(vec![(0.0, 1013.0), (1.0, 1015.0)]).color(Color::Blue))
///     .x_axis(Axis::new().label("Time"))
///     .primary_y_axis(Axis::new().label("Temp (°C)"))
///     .secondary_y_axis(Axis::new().label("Pressure (hPa)"));
/// ```
pub struct TwinAxes {
    primary_series: Vec<Series>,
    secondary_series: Vec<Series>,
    x_axis: Axis,
    primary_y_axis: Axis,
    secondary_y_axis: Axis,
    title: Option<String>,
}

impl Default for TwinAxes {
    fn default() -> Self {
        Self {
            primary_series: Vec::new(),
            secondary_series: Vec::new(),
            x_axis: Axis::new(),
            primary_y_axis: Axis::new(),
            secondary_y_axis: Axis::new(),
            title: None,
        }
    }
}

impl TwinAxes {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a series to the primary (left) y-axis.
    pub fn primary(mut self, s: Series) -> Self {
        self.primary_series.push(s);
        self
    }

    /// Add a series to the secondary (right) y-axis.
    pub fn secondary(mut self, s: Series) -> Self {
        self.secondary_series.push(s);
        self
    }

    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    pub fn primary_y_axis(mut self, axis: Axis) -> Self {
        self.primary_y_axis = axis;
        self
    }

    pub fn secondary_y_axis(mut self, axis: Axis) -> Self {
        self.secondary_y_axis = axis;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    fn compute_bounds(series: &[Series]) -> (f64, f64, f64, f64) {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for s in series {
            if let Some((lo, hi)) = s.x_bounds() {
                x_min = x_min.min(lo);
                x_max = x_max.max(hi);
            }
            if let Some((lo, hi)) = s.y_bounds() {
                y_min = y_min.min(lo);
                y_max = y_max.max(hi);
            }
        }
        if x_min.is_infinite() { x_min = 0.0; x_max = 1.0; }
        if y_min.is_infinite() { y_min = 0.0; y_max = 1.0; }
        (x_min, x_max, y_min, y_max)
    }
}

impl Widget for &TwinAxes {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 4 {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let left_label_width: u16 = 8;
        let right_label_width: u16 = 8;
        let tick_height: u16 = 1;

        let px = area.x + left_label_width;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(left_label_width + right_label_width + 1);
        let ph = area.height.saturating_sub(title_height + tick_height);

        if pw < 2 || ph < 2 {
            return;
        }

        // Title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_style(Style::default().fg(Color::White));
                }
            }
        }

        // Compute bounds for both series sets
        let (px_min, px_max, py_min, py_max) = TwinAxes::compute_bounds(&self.primary_series);
        let (sx_min, sx_max, sy_min, sy_max) = TwinAxes::compute_bounds(&self.secondary_series);

        // Shared x-axis bounds
        let x_min = px_min.min(sx_min);
        let x_max = px_max.max(sx_max);
        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (py_lo, py_hi) = self.primary_y_axis.resolve_bounds(py_min, py_max);
        let (sy_lo, sy_hi) = self.secondary_y_axis.resolve_bounds(sy_min, sy_max);

        // Draw axes
        for x in px..px + pw {
            if x < area.x + area.width {
                buf[(x, py + ph)].set_char('─').set_fg(Color::DarkGray);
            }
        }
        // Left y-axis
        for y in py..py + ph {
            buf[(px.saturating_sub(1), y)].set_char('│').set_fg(Color::DarkGray);
        }
        // Right y-axis
        let right_x = px + pw;
        if right_x < area.x + area.width {
            for y in py..py + ph {
                buf[(right_x, y)].set_char('│').set_fg(Color::DarkGray);
            }
        }

        // X tick labels
        let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let label_start = xi.saturating_sub(label.len() as u16 / 2);
            let y = py + ph;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        }

        // Left (primary) y tick labels
        let py_ticks = self.primary_y_axis.tick_positions(py_lo, py_hi);
        for &tv in &py_ticks {
            let sy = data_to_screen(tv, py_lo, py_hi, (py + ph - 1) as f64, py as f64);
            let label = self.primary_y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ph {
                let label_start = px.saturating_sub(label.len() as u16 + 1);
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16 + 1;
                    if lx < px && lx >= area.x {
                        let color = self.primary_series.first().map_or(Color::DarkGray, |s| s.color);
                        buf[(lx, yi)].set_char(ch).set_fg(color);
                    }
                }
            }
        }

        // Right (secondary) y tick labels
        let sy_ticks = self.secondary_y_axis.tick_positions(sy_lo, sy_hi);
        for &tv in &sy_ticks {
            let sy = data_to_screen(tv, sy_lo, sy_hi, (py + ph - 1) as f64, py as f64);
            let label = self.secondary_y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ph {
                let start_x = right_x + 1;
                for (j, ch) in label.chars().enumerate() {
                    let lx = start_x + j as u16;
                    if lx < area.x + area.width {
                        let color = self.secondary_series.first().map_or(Color::DarkGray, |s| s.color);
                        buf[(lx, yi)].set_char(ch).set_fg(color);
                    }
                }
            }
        }

        // Draw primary series lines
        for s in &self.primary_series {
            for i in 0..s.data.len().saturating_sub(1) {
                let (x0, y0) = s.data[i];
                let (x1, y1) = s.data[i + 1];
                if !x0.is_finite() || !y0.is_finite() || !x1.is_finite() || !y1.is_finite() {
                    continue;
                }
                let sx0 = data_to_screen(x0, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
                let sy0 = data_to_screen(y0, py_lo, py_hi, (py + ph - 1) as f64, py as f64);
                let sx1 = data_to_screen(x1, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
                let sy1 = data_to_screen(y1, py_lo, py_hi, (py + ph - 1) as f64, py as f64);
                draw_simple_line(buf, sx0, sy0, sx1, sy1, s.color, px, py, pw, ph);
            }
        }

        // Draw secondary series lines
        for s in &self.secondary_series {
            for i in 0..s.data.len().saturating_sub(1) {
                let (x0, y0) = s.data[i];
                let (x1, y1) = s.data[i + 1];
                if !x0.is_finite() || !y0.is_finite() || !x1.is_finite() || !y1.is_finite() {
                    continue;
                }
                let sx0 = data_to_screen(x0, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
                let sy0 = data_to_screen(y0, sy_lo, sy_hi, (py + ph - 1) as f64, py as f64);
                let sx1 = data_to_screen(x1, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
                let sy1 = data_to_screen(y1, sy_lo, sy_hi, (py + ph - 1) as f64, py as f64);
                draw_simple_line(buf, sx0, sy0, sx1, sy1, s.color, px, py, pw, ph);
            }
        }
    }
}

/// Simple Bresenham line drawing within a clipping rectangle.
#[allow(clippy::too_many_arguments)]
fn draw_simple_line(
    buf: &mut Buffer,
    x0: f64, y0: f64, x1: f64, y1: f64,
    color: Color,
    clip_x: u16, clip_y: u16, clip_w: u16, clip_h: u16,
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

    loop {
        let px = ix0 as u16;
        let py = iy0 as u16;
        if px >= clip_x && px < clip_x + clip_w && py >= clip_y && py < clip_y + clip_h {
            let ch = if dx > dy.unsigned_abs() as i32 * 2 {
                '─'
            } else if dy.unsigned_abs() as i32 > dx * 2 {
                '│'
            } else if (sx > 0 && sy > 0) || (sx < 0 && sy < 0) {
                '╲'
            } else {
                '╱'
            };
            buf[(px, py)].set_char(ch).set_fg(color);
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
    }
}
