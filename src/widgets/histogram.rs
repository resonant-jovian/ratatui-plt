//! Histogram widget for distribution visualization.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::theme::Theme;
use crate::transform::data_to_screen;

/// Histogram normalization mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistNorm {
    /// Raw counts.
    Count,
    /// Probability density (area sums to 1).
    Density,
    /// Probability (bar heights sum to 1).
    Probability,
}

/// A histogram widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::histogram::Histogram;
///
/// let hist = Histogram::new(vec![1.0, 1.5, 2.0, 2.5, 3.0, 3.5])
///     .bins(10)
///     .color(ratatui::style::Color::Cyan)
///     .title("Distribution");
/// ```
pub struct Histogram {
    data: Vec<f64>,
    bins: usize,
    range: Option<(f64, f64)>,
    norm_mode: HistNorm,
    color: Color,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    cumulative: bool,
    theme: Theme,
}

impl Histogram {
    /// Create a histogram from raw data.
    pub fn new(data: Vec<f64>) -> Self {
        Self {
            data,
            bins: 20,
            range: None,
            norm_mode: HistNorm::Count,
            color: Color::Cyan,
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            cumulative: false,
            theme: Theme::get_default(),
        }
    }

    /// Set the number of bins.
    pub fn bins(mut self, n: usize) -> Self {
        self.bins = n.max(1);
        self
    }

    /// Set the data range.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.range = Some((min, max));
        self
    }

    /// Set the normalization mode.
    pub fn norm_mode(mut self, mode: HistNorm) -> Self {
        self.norm_mode = mode;
        self
    }

    /// Set the bar color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
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

    /// Enable cumulative histogram.
    pub fn cumulative(mut self, c: bool) -> Self {
        self.cumulative = c;
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Compute bin edges and heights. NaN values are filtered out.
    fn compute_bins(&self) -> (Vec<f64>, Vec<f64>) {
        let (lo, hi) = self.range.unwrap_or_else(|| {
            let min = self
                .data
                .iter()
                .filter(|v| v.is_finite())
                .cloned()
                .fold(f64::INFINITY, f64::min);
            let max = self
                .data
                .iter()
                .filter(|v| v.is_finite())
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            if min == max {
                (min - 1.0, max + 1.0)
            } else {
                (min, max)
            }
        });

        let bin_width = (hi - lo) / self.bins as f64;
        let edges: Vec<f64> = (0..=self.bins).map(|i| lo + i as f64 * bin_width).collect();
        let mut counts = vec![0.0f64; self.bins];

        for &v in &self.data {
            if !v.is_finite() {
                continue;
            }
            if v >= lo && v <= hi {
                let idx = ((v - lo) / bin_width).floor() as usize;
                let idx = idx.min(self.bins - 1);
                counts[idx] += 1.0;
            }
        }

        if self.cumulative {
            for i in 1..counts.len() {
                counts[i] += counts[i - 1];
            }
        }

        let heights = match self.norm_mode {
            HistNorm::Count => counts,
            HistNorm::Density => {
                let total = self.data.len() as f64;
                counts.iter().map(|&c| c / (total * bin_width)).collect()
            }
            HistNorm::Probability => {
                let total = self.data.len() as f64;
                counts.iter().map(|&c| c / total).collect()
            }
        };

        (edges, heights)
    }
}

impl Widget for &Histogram {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
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

        // Draw title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        let (edges, heights) = self.compute_bins();
        if edges.len() < 2 || heights.is_empty() {
            return;
        }

        let x_lo = edges[0];
        let x_hi = *edges.last().unwrap();
        let y_max = heights.iter().cloned().fold(0.0f64, f64::max);
        let y_hi = if y_max == 0.0 { 1.0 } else { y_max * 1.1 };

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
            let gy_ticks = self.y_axis.tick_positions(0.0, y_hi);
            for &tv in &gy_ticks {
                let sy = data_to_screen(tv, 0.0, y_hi, (py + ph - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ph {
                    for x in px..px + pw {
                        buf[(x, yi)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }

        // Draw bars
        for i in 0..heights.len() {
            let bar_left = data_to_screen(edges[i], x_lo, x_hi, px as f64, (px + pw - 1) as f64);
            let bar_right =
                data_to_screen(edges[i + 1], x_lo, x_hi, px as f64, (px + pw - 1) as f64);
            let bar_top = data_to_screen(heights[i], 0.0, y_hi, (py + ph - 1) as f64, py as f64);

            let x_start = bar_left.round() as u16;
            let x_end = bar_right.round() as u16;
            let y_top = bar_top.round() as u16;

            for x in x_start..x_end {
                if x >= px && x < px + pw {
                    for y in y_top..py + ph {
                        if y >= py && y < py + ph {
                            buf[(x, y)].set_char('█').set_fg(self.color);
                        }
                    }
                }
            }

            // Bar outline
            if heights[i] > 0.0 {
                if y_top >= py && y_top < py + ph {
                    for x in x_start..x_end {
                        if x >= px && x < px + pw {
                            buf[(x, y_top)].set_char('─').set_fg(self.theme.axis_color);
                        }
                    }
                }
                if x_start >= px && x_start < px + pw {
                    for y in y_top..py + ph {
                        if y >= py && y < py + ph {
                            buf[(x_start, y)].set_char('│').set_fg(self.theme.axis_color);
                        }
                    }
                }
            }
        }

        // Draw tick labels
        let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
        let tick_y = py + ph;
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let start = xi.saturating_sub(label.len() as u16 / 2);
            if tick_y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, tick_y)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }

        let y_ticks = self.y_axis.tick_positions(0.0, y_hi);
        for &tv in &y_ticks {
            let sy = data_to_screen(tv, 0.0, y_hi, (py + ph - 1) as f64, py as f64);
            let label = self.y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ph {
                let start = px.saturating_sub(label.len() as u16 + 1);
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < px {
                        buf[(lx, yi)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }
    }
}
