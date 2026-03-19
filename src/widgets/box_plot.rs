//! Box plot widget for distribution comparison.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::transform::data_to_screen;

/// A single box-and-whisker dataset.
#[derive(Clone, Debug)]
pub struct BoxData {
    /// Category label.
    pub label: String,
    /// Raw data values (will be sorted to compute quartiles).
    pub values: Vec<f64>,
    /// Box color.
    pub color: Color,
}

impl BoxData {
    pub fn new(label: impl Into<String>, values: Vec<f64>, color: Color) -> Self {
        Self {
            label: label.into(),
            values,
            color,
        }
    }

    /// Compute quartiles (Q1, median, Q3).
    pub fn quartiles(&self) -> (f64, f64, f64) {
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();
        if n == 0 {
            return (0.0, 0.0, 0.0);
        }
        let median = percentile(&sorted, 50.0);
        let q1 = percentile(&sorted, 25.0);
        let q3 = percentile(&sorted, 75.0);
        (q1, median, q3)
    }

    /// Compute whisker endpoints (1.5 × IQR rule).
    pub fn whiskers(&self) -> (f64, f64) {
        let (q1, _, q3) = self.quartiles();
        let iqr = q3 - q1;
        let lower_fence = q1 - 1.5 * iqr;
        let upper_fence = q3 + 1.5 * iqr;

        let lower = self
            .values
            .iter()
            .filter(|&&v| v >= lower_fence)
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let upper = self
            .values
            .iter()
            .filter(|&&v| v <= upper_fence)
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        (lower, upper)
    }

    /// Get outlier values.
    pub fn outliers(&self) -> Vec<f64> {
        let (q1, _, q3) = self.quartiles();
        let iqr = q3 - q1;
        let lower = q1 - 1.5 * iqr;
        let upper = q3 + 1.5 * iqr;
        self.values
            .iter()
            .filter(|&&v| v < lower || v > upper)
            .cloned()
            .collect()
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let k = (p / 100.0) * (sorted.len() - 1) as f64;
    let f = k.floor() as usize;
    let c = f.min(sorted.len() - 1);
    let d = k - f as f64;
    if c + 1 < sorted.len() {
        sorted[c] + d * (sorted[c + 1] - sorted[c])
    } else {
        sorted[c]
    }
}

/// A box plot widget.
///
/// # Example
///
/// ```
/// use ratatui_sim::widgets::box_plot::{BoxPlot, BoxData};
/// use ratatui::style::Color;
///
/// let plot = BoxPlot::new()
///     .box_data(BoxData::new("Group A", vec![1.0, 2.0, 3.0, 4.0, 5.0], Color::Cyan))
///     .box_data(BoxData::new("Group B", vec![2.0, 3.0, 4.0, 5.0, 8.0], Color::Yellow))
///     .title("Distribution Comparison");
/// ```
pub struct BoxPlot {
    data: Vec<BoxData>,
    title: Option<String>,
    y_axis: Axis,
    show_outliers: bool,
}

impl Default for BoxPlot {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            title: None,
            y_axis: Axis::new(),
            show_outliers: true,
        }
    }
}

impl BoxPlot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn box_data(mut self, d: BoxData) -> Self {
        self.data.push(d);
        self
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    pub fn show_outliers(mut self, show: bool) -> Self {
        self.show_outliers = show;
        self
    }
}

impl Widget for &BoxPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.data.is_empty() {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let y_label_width: u16 = 8;
        let cat_height: u16 = 1;

        let px = area.x + y_label_width;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(y_label_width + 1);
        let ph = area.height.saturating_sub(title_height + cat_height);

        if pw < 2 || ph < 2 {
            return;
        }

        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(Color::White);
                }
            }
        }

        // Compute global y range
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for d in &self.data {
            for &v in &d.values {
                y_min = y_min.min(v);
                y_max = y_max.max(v);
            }
        }
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Draw axes
        for y in py..py + ph {
            buf[(px.saturating_sub(1), y)].set_char('│').set_fg(Color::DarkGray);
        }

        let n = self.data.len();
        let box_width = (pw / n as u16).saturating_sub(2).max(3);

        for (i, d) in self.data.iter().enumerate() {
            let center_x = px + (i as u16 * pw / n as u16) + pw / n as u16 / 2;
            let box_left = center_x.saturating_sub(box_width / 2);
            let box_right = box_left + box_width;

            let (q1, median, q3) = d.quartiles();
            let (whisker_lo, whisker_hi) = d.whiskers();

            let sy_q1 = data_to_screen(q1, y_lo, y_hi, (py + ph - 1) as f64, py as f64).round() as u16;
            let sy_median = data_to_screen(median, y_lo, y_hi, (py + ph - 1) as f64, py as f64).round() as u16;
            let sy_q3 = data_to_screen(q3, y_lo, y_hi, (py + ph - 1) as f64, py as f64).round() as u16;
            let sy_wlo = data_to_screen(whisker_lo, y_lo, y_hi, (py + ph - 1) as f64, py as f64).round() as u16;
            let sy_whi = data_to_screen(whisker_hi, y_lo, y_hi, (py + ph - 1) as f64, py as f64).round() as u16;

            // Draw box (Q1 to Q3)
            for x in box_left..box_right {
                if x >= px && x < px + pw {
                    if sy_q3 >= py && sy_q3 < py + ph {
                        buf[(x, sy_q3)].set_char('─').set_fg(d.color);
                    }
                    if sy_q1 >= py && sy_q1 < py + ph {
                        buf[(x, sy_q1)].set_char('─').set_fg(d.color);
                    }
                }
            }
            // Box sides
            for y in sy_q3..=sy_q1 {
                if y >= py && y < py + ph {
                    if box_left >= px && box_left < px + pw {
                        buf[(box_left, y)].set_char('│').set_fg(d.color);
                    }
                    if box_right >= px && box_right < px + pw {
                        buf[(box_right, y)].set_char('│').set_fg(d.color);
                    }
                }
            }

            // Median line
            for x in box_left..=box_right {
                if x >= px && x < px + pw && sy_median >= py && sy_median < py + ph {
                    buf[(x, sy_median)].set_char('━').set_fg(d.color);
                }
            }

            // Whiskers
            for y in sy_whi..sy_q3 {
                if y >= py && y < py + ph {
                    buf[(center_x, y)].set_char('╎').set_fg(d.color);
                }
            }
            for y in sy_q1..=sy_wlo {
                if y >= py && y < py + ph {
                    buf[(center_x, y)].set_char('╎').set_fg(d.color);
                }
            }

            // Whisker caps
            let cap_left = center_x.saturating_sub(box_width / 4);
            let cap_right = center_x + box_width / 4;
            for x in cap_left..=cap_right {
                if x >= px && x < px + pw {
                    if sy_whi >= py && sy_whi < py + ph {
                        buf[(x, sy_whi)].set_char('─').set_fg(d.color);
                    }
                    if sy_wlo >= py && sy_wlo < py + ph {
                        buf[(x, sy_wlo)].set_char('─').set_fg(d.color);
                    }
                }
            }

            // Outliers
            if self.show_outliers {
                for &v in &d.outliers() {
                    let sy = data_to_screen(v, y_lo, y_hi, (py + ph - 1) as f64, py as f64).round() as u16;
                    if center_x >= px && center_x < px + pw && sy >= py && sy < py + ph {
                        buf[(center_x, sy)].set_char('○').set_fg(d.color);
                    }
                }
            }

            // Category label
            let label = &d.label;
            let label_start = center_x.saturating_sub(label.len() as u16 / 2);
            let label_y = py + ph;
            if label_y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, label_y)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        }

        // Y axis ticks
        let y_ticks = self.y_axis.tick_positions(y_lo, y_hi);
        for &tv in &y_ticks {
            let sy = data_to_screen(tv, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
            let label = self.y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ph {
                let start = px.saturating_sub(label.len() as u16 + 1);
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < px {
                        buf[(lx, yi)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        }
    }
}
