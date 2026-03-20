//! Stem plot widget for discrete event visualization.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::style::MarkerShape;
use crate::theme::Theme;
use crate::transform::data_to_screen;

/// A stem plot widget — vertical lines from a baseline to data points.
///
/// Ideal for discrete events (supernovae, collision events, impulse responses).
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::stem_plot::StemPlot;
/// use ratatui::style::Color;
///
/// let plot = StemPlot::new(vec![(1.0, 3.0), (2.0, 5.0), (3.0, 2.0)])
///     .color(Color::Green)
///     .baseline(0.0)
///     .title("Events");
/// ```
pub struct StemPlot {
    data: Vec<(f64, f64)>,
    baseline: f64,
    color: Color,
    marker: MarkerShape,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    theme: Theme,
}

impl StemPlot {
    pub fn new(data: Vec<(f64, f64)>) -> Self {
        Self {
            data,
            baseline: 0.0,
            color: Color::Cyan,
            marker: MarkerShape::FilledCircle,
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            theme: Theme::get_default(),
        }
    }

    pub fn baseline(mut self, b: f64) -> Self {
        self.baseline = b;
        self
    }

    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }

    pub fn marker(mut self, m: MarkerShape) -> Self {
        self.marker = m;
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

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for &StemPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.data.is_empty() {
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

        // Title
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
        let x_min = self.data.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x_max = self
            .data
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let y_min = self
            .data
            .iter()
            .map(|p| p.1)
            .chain(std::iter::once(self.baseline))
            .fold(f64::INFINITY, f64::min);
        let y_max = self
            .data
            .iter()
            .map(|p| p.1)
            .chain(std::iter::once(self.baseline))
            .fold(f64::NEG_INFINITY, f64::max);

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

        // Draw baseline
        let base_sy = data_to_screen(self.baseline, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
        let base_yi = base_sy.round() as u16;
        if base_yi >= py && base_yi < py + ph {
            for x in px..px + pw {
                buf[(x, base_yi)]
                    .set_char('─')
                    .set_fg(self.theme.axis_color);
            }
        }

        // Draw stems and markers
        for &(x, y) in &self.data {
            let sx = data_to_screen(x, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
            let sy = data_to_screen(y, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
            let xi = sx.round() as u16;
            let yi = sy.round() as u16;

            if xi < px || xi >= px + pw {
                continue;
            }

            // Draw stem line
            let (y_top, y_bot) = if yi < base_yi {
                (yi, base_yi)
            } else {
                (base_yi, yi)
            };
            for sy in y_top..=y_bot {
                if sy >= py && sy < py + ph {
                    buf[(xi, sy)].set_char('│').set_fg(self.color);
                }
            }

            // Draw marker at data point
            if yi >= py && yi < py + ph {
                buf[(xi, yi)]
                    .set_char(self.marker.char())
                    .set_fg(self.color);
            }
        }

        // Tick labels
        let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let start = xi.saturating_sub(label.len() as u16 / 2);
            let y = py + ph;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }
    }
}
