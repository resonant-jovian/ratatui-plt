//! Standalone error bar plot widget.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::theme::Theme;
use crate::transform::data_to_screen;

/// Error bar direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorDirection {
    Vertical,
    Horizontal,
    Both,
}

/// A standalone error bar plot.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::error_bar::ErrorBarPlot;
///
/// let plot = ErrorBarPlot::new()
///     .data(vec![(1.0, 2.0)], vec![0.3], vec![0.5])
///     .title("Measurement Errors");
/// ```
pub struct ErrorBarPlot {
    points: Vec<(f64, f64)>,
    y_err_low: Vec<f64>,
    y_err_high: Vec<f64>,
    x_err_low: Vec<f64>,
    x_err_high: Vec<f64>,
    direction: ErrorDirection,
    color: Color,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    theme: Theme,
}

impl Default for ErrorBarPlot {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            y_err_low: Vec::new(),
            y_err_high: Vec::new(),
            x_err_low: Vec::new(),
            x_err_high: Vec::new(),
            direction: ErrorDirection::Vertical,
            color: Color::White,
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            theme: Theme::get_default(),
        }
    }
}

impl ErrorBarPlot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set data points with symmetric/asymmetric y-error bars.
    pub fn data(mut self, points: Vec<(f64, f64)>, err_low: Vec<f64>, err_high: Vec<f64>) -> Self {
        self.points = points;
        self.y_err_low = err_low;
        self.y_err_high = err_high;
        self
    }

    /// Set horizontal error bars.
    pub fn x_errors(mut self, err_low: Vec<f64>, err_high: Vec<f64>) -> Self {
        self.x_err_low = err_low;
        self.x_err_high = err_high;
        if self.direction == ErrorDirection::Vertical {
            self.direction = ErrorDirection::Both;
        }
        self
    }

    pub fn direction(mut self, d: ErrorDirection) -> Self {
        self.direction = d;
        self
    }

    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    pub fn theme(mut self, t: Theme) -> Self {
        self.theme = t;
        self
    }
}

impl Widget for &ErrorBarPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.points.is_empty() {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let y_label_width: u16 = 8;
        let tick_height: u16 = 1;

        let px = area.x + y_label_width;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(y_label_width + 1);
        let ph = area.height.saturating_sub(title_height + tick_height);

        if pw < 2 || ph < 2 {
            return;
        }

        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        // Compute bounds
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for (i, &(x, y)) in self.points.iter().enumerate() {
            let xlo = x - self.x_err_low.get(i).copied().unwrap_or(0.0);
            let xhi = x + self.x_err_high.get(i).copied().unwrap_or(0.0);
            let ylo = y - self.y_err_low.get(i).copied().unwrap_or(0.0);
            let yhi = y + self.y_err_high.get(i).copied().unwrap_or(0.0);
            x_min = x_min.min(xlo);
            x_max = x_max.max(xhi);
            y_min = y_min.min(ylo);
            y_max = y_max.max(yhi);
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Draw axes
        for x in px..px + pw {
            buf[(x, py + ph)]
                .set_char('─')
                .set_fg(self.theme.axis_color);
        }
        for y in py..py + ph {
            buf[(px.saturating_sub(1), y)]
                .set_char('│')
                .set_fg(self.theme.axis_color);
        }

        // Draw grid
        let x_grid = self.x_axis.grid || self.theme.grid_visible;
        let y_grid = self.y_axis.grid || self.theme.grid_visible;
        if x_grid {
            let gx_ticks = self.x_axis.tick_positions(x_lo, x_hi);
            for &tv in &gx_ticks {
                let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
                let xi = sx.round() as u16;
                if xi >= px && xi < px + pw {
                    for y in py..py + ph {
                        buf[(xi, y)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }
        if y_grid {
            let gy_ticks = self.y_axis.tick_positions(y_lo, y_hi);
            for &tv in &gy_ticks {
                let sy = data_to_screen(tv, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ph {
                    for x in px..px + pw {
                        buf[(x, yi)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }

        // Draw error bars and points
        for (i, &(x, y)) in self.points.iter().enumerate() {
            let sx = data_to_screen(x, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
            let sy = data_to_screen(y, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
            let xi = sx.round() as u16;
            let yi = sy.round() as u16;

            // Vertical error bars
            if matches!(
                self.direction,
                ErrorDirection::Vertical | ErrorDirection::Both
            ) {
                let elo = self.y_err_low.get(i).copied().unwrap_or(0.0);
                let ehi = self.y_err_high.get(i).copied().unwrap_or(0.0);
                let sy_lo = data_to_screen(y - elo, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
                let sy_hi = data_to_screen(y + ehi, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
                let y_top = sy_hi.round() as u16;
                let y_bot = sy_lo.round() as u16;

                if xi >= px && xi < px + pw {
                    for ey in y_top..=y_bot {
                        if ey >= py && ey < py + ph {
                            buf[(xi, ey)].set_char('│').set_fg(self.color);
                        }
                    }
                    if y_top >= py && y_top < py + ph {
                        buf[(xi, y_top)].set_char('┬').set_fg(self.color);
                    }
                    if y_bot >= py && y_bot < py + ph {
                        buf[(xi, y_bot)].set_char('┴').set_fg(self.color);
                    }
                }
            }

            // Horizontal error bars
            if matches!(
                self.direction,
                ErrorDirection::Horizontal | ErrorDirection::Both
            ) {
                let elo = self.x_err_low.get(i).copied().unwrap_or(0.0);
                let ehi = self.x_err_high.get(i).copied().unwrap_or(0.0);
                let sx_lo = data_to_screen(x - elo, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
                let sx_hi = data_to_screen(x + ehi, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
                let x_left = sx_lo.round() as u16;
                let x_right = sx_hi.round() as u16;

                if yi >= py && yi < py + ph {
                    for ex in x_left..=x_right {
                        if ex >= px && ex < px + pw {
                            buf[(ex, yi)].set_char('─').set_fg(self.color);
                        }
                    }
                    if x_left >= px && x_left < px + pw {
                        buf[(x_left, yi)].set_char('├').set_fg(self.color);
                    }
                    if x_right >= px && x_right < px + pw {
                        buf[(x_right, yi)].set_char('┤').set_fg(self.color);
                    }
                }
            }

            // Draw center point
            if xi >= px && xi < px + pw && yi >= py && yi < py + ph {
                buf[(xi, yi)].set_char('●').set_fg(self.color);
            }
        }
    }
}
