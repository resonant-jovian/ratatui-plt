//! Violin plot widget for distribution comparison using kernel density estimation.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::theme::Theme;
use crate::transform::data_to_screen;

/// A single violin dataset.
#[derive(Clone, Debug)]
pub struct ViolinData {
    /// Category label.
    pub label: String,
    /// Raw data values.
    pub values: Vec<f64>,
    /// Violin color.
    pub color: Color,
}

impl ViolinData {
    pub fn new(label: impl Into<String>, values: Vec<f64>, color: Color) -> Self {
        Self {
            label: label.into(),
            values,
            color,
        }
    }

    /// Return a copy of values with NaN/Inf filtered out and sorted.
    fn finite_sorted(&self) -> Vec<f64> {
        let mut v: Vec<f64> = self
            .values
            .iter()
            .copied()
            .filter(|x| x.is_finite())
            .collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v
    }

    /// Compute quartiles (Q1, median, Q3) from sorted finite values.
    fn quartiles(sorted: &[f64]) -> (f64, f64, f64) {
        if sorted.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        (
            percentile(sorted, 25.0),
            percentile(sorted, 50.0),
            percentile(sorted, 75.0),
        )
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

/// Gaussian kernel density estimation.
///
/// For each evaluation point `y`, computes:
///   KDE(y) = (1 / (n * h)) * sum_i  exp(-0.5 * ((y - x_i) / h)^2) / sqrt(2 * pi)
///
/// Uses Silverman's rule of thumb for bandwidth: h = 0.9 * min(std, IQR/1.34) * n^(-1/5).
fn gaussian_kde(sorted_values: &[f64], eval_points: &[f64]) -> Vec<f64> {
    let n = sorted_values.len();
    if n == 0 {
        return vec![0.0; eval_points.len()];
    }
    let nf = n as f64;

    // Compute mean
    let mean = sorted_values.iter().sum::<f64>() / nf;

    // Compute standard deviation
    let variance = sorted_values
        .iter()
        .map(|&x| (x - mean) * (x - mean))
        .sum::<f64>()
        / nf;
    let std_dev = variance.sqrt();

    // Compute IQR
    let q1 = percentile(sorted_values, 25.0);
    let q3 = percentile(sorted_values, 75.0);
    let iqr = q3 - q1;

    // Silverman's rule of thumb
    let spread = if iqr > 0.0 {
        std_dev.min(iqr / 1.34)
    } else if std_dev > 0.0 {
        std_dev
    } else {
        1.0
    };
    let h = 0.9 * spread * nf.powf(-0.2);
    let h = if h <= 0.0 { 1.0 } else { h };

    let inv_h = 1.0 / h;
    let norm_factor = 1.0 / (nf * h * (2.0 * std::f64::consts::PI).sqrt());

    eval_points
        .iter()
        .map(|&y| {
            let sum: f64 = sorted_values
                .iter()
                .map(|&xi| {
                    let z = (y - xi) * inv_h;
                    (-0.5 * z * z).exp()
                })
                .sum();
            sum * norm_factor
        })
        .collect()
}

/// A violin plot widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::violin_plot::{ViolinPlot, ViolinData};
/// use ratatui::style::Color;
///
/// let plot = ViolinPlot::new()
///     .dataset(ViolinData::new("Group A", vec![1.0, 2.0, 3.0, 4.0, 5.0], Color::Cyan))
///     .dataset(ViolinData::new("Group B", vec![2.0, 3.0, 4.0, 5.0, 8.0], Color::Yellow))
///     .title("Violin Comparison")
///     .show_box(true);
/// ```
pub struct ViolinPlot {
    datasets: Vec<ViolinData>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_box: bool,
    /// Visual theme.
    theme: Theme,
}

impl Default for ViolinPlot {
    fn default() -> Self {
        Self {
            datasets: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_box: true,
            theme: Theme::get_default(),
        }
    }
}

impl ViolinPlot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dataset(mut self, d: ViolinData) -> Self {
        self.datasets.push(d);
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

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn show_box(mut self, show: bool) -> Self {
        self.show_box = show;
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for &ViolinPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.datasets.is_empty() {
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

        // Compute global y range from all datasets (finite values only)
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for d in &self.datasets {
            for &v in &d.values {
                if v.is_finite() {
                    y_min = y_min.min(v);
                    y_max = y_max.max(v);
                }
            }
        }
        if !y_min.is_finite() || !y_max.is_finite() {
            return;
        }
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Draw y-axis line
        for y in py..py + ph {
            buf[(px.saturating_sub(1), y)]
                .set_char('│')
                .set_fg(self.theme.axis_color);
        }

        // Draw grid (y-axis only; x-axis is categorical)
        let y_grid = self.y_axis.grid || self.theme.grid_visible;
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

        let n = self.datasets.len();
        let slot_width = pw / n as u16;

        // Number of KDE evaluation points (one per screen row, doubled for half-block resolution)
        let n_eval = (ph as usize * 2).max(10);

        for (i, d) in self.datasets.iter().enumerate() {
            let sorted = d.finite_sorted();
            if sorted.is_empty() {
                continue;
            }

            let center_x = px + (i as u16 * slot_width) + slot_width / 2;

            // Build evaluation grid spanning the y range
            let eval_points: Vec<f64> = (0..n_eval)
                .map(|j| y_lo + (y_hi - y_lo) * j as f64 / (n_eval - 1).max(1) as f64)
                .collect();

            let kde_values = gaussian_kde(&sorted, &eval_points);

            // Find max KDE value for scaling the width
            let kde_max = kde_values.iter().cloned().fold(0.0f64, f64::max);
            if kde_max <= 0.0 {
                continue;
            }

            // Max half-width in characters for one side of the violin
            let max_half_width = (slot_width / 2).saturating_sub(1).max(1) as f64;

            // Render the violin shape using half-block characters.
            // We process pairs of eval_points to produce half-block rows.
            // Each screen row covers two eval indices (top half, bottom half).
            // We iterate screen rows from top (high y) to bottom (low y).
            for row in 0..ph {
                let screen_y = py + row;
                if screen_y >= area.y + area.height {
                    break;
                }

                // Map screen row to eval index. Top of screen = high y, bottom = low y.
                // top half-pixel
                let top_eval_idx_f = (1.0 - (row as f64 * 2.0) / (n_eval as f64 - 1.0).max(1.0))
                    * (n_eval - 1) as f64;
                let top_idx = (top_eval_idx_f.round() as usize).min(n_eval - 1);
                // bottom half-pixel
                let bot_eval_idx_f = (1.0
                    - (row as f64 * 2.0 + 1.0) / (n_eval as f64 - 1.0).max(1.0))
                    * (n_eval - 1) as f64;
                let bot_idx = (bot_eval_idx_f.round() as usize).min(n_eval - 1);

                let top_width = (kde_values[top_idx] / kde_max * max_half_width).round() as u16;
                let bot_width = (kde_values[bot_idx] / kde_max * max_half_width).round() as u16;

                // Draw mirrored filled violin using half-block chars
                // For each column offset from center, determine if top/bottom halves are filled
                let max_w = top_width.max(bot_width);
                for dx in 0..=max_w {
                    let top_filled = dx <= top_width && top_width > 0;
                    let bot_filled = dx <= bot_width && bot_width > 0;

                    let ch = match (top_filled, bot_filled) {
                        (true, true) => '█',
                        (true, false) => '▀',
                        (false, true) => '▄',
                        (false, false) => continue,
                    };

                    // Draw on both sides (mirrored)
                    let positions = if dx == 0 {
                        vec![center_x]
                    } else {
                        let mut p = Vec::new();
                        if center_x + dx < px + pw {
                            p.push(center_x + dx);
                        }
                        if center_x >= dx + px {
                            p.push(center_x - dx);
                        }
                        p
                    };

                    for &sx in &positions {
                        if sx >= px && sx < px + pw {
                            let style = match (top_filled, bot_filled) {
                                (true, true) => Style::default().fg(d.color),
                                (true, false) => Style::default().fg(d.color),
                                (false, true) => Style::default().fg(d.color),
                                _ => Style::default(),
                            };
                            buf[(sx, screen_y)].set_char(ch).set_style(style);
                        }
                    }
                }
            }

            // Optional inner box plot overlay
            if self.show_box {
                let (q1, median, q3) = ViolinData::quartiles(&sorted);

                let sy_q1 =
                    data_to_screen(q1, y_lo, y_hi, (py + ph - 1) as f64, py as f64).round() as u16;
                let sy_median = data_to_screen(median, y_lo, y_hi, (py + ph - 1) as f64, py as f64)
                    .round() as u16;
                let sy_q3 =
                    data_to_screen(q3, y_lo, y_hi, (py + ph - 1) as f64, py as f64).round() as u16;

                // Draw thin box from Q1 to Q3 (single character wide at center)
                let box_half = 1u16;
                for y in sy_q3..=sy_q1 {
                    if y >= py && y < py + ph {
                        for dx in 0..=box_half {
                            let positions: Vec<u16> = if dx == 0 {
                                vec![center_x]
                            } else {
                                let mut p = Vec::new();
                                if center_x + dx < px + pw {
                                    p.push(center_x + dx);
                                }
                                if center_x >= dx + px {
                                    p.push(center_x - dx);
                                }
                                p
                            };
                            for &sx in &positions {
                                if sx >= px && sx < px + pw {
                                    buf[(sx, y)].set_char('│').set_fg(self.theme.foreground);
                                }
                            }
                        }
                    }
                }

                // Median line
                for dx in 0..=box_half {
                    if sy_median >= py && sy_median < py + ph {
                        let positions: Vec<u16> = if dx == 0 {
                            vec![center_x]
                        } else {
                            let mut p = Vec::new();
                            if center_x + dx < px + pw {
                                p.push(center_x + dx);
                            }
                            if center_x >= dx + px {
                                p.push(center_x - dx);
                            }
                            p
                        };
                        for &sx in &positions {
                            if sx >= px && sx < px + pw {
                                buf[(sx, sy_median)]
                                    .set_char('━')
                                    .set_fg(self.theme.foreground);
                            }
                        }
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
                        buf[(lx, label_y)]
                            .set_char(ch)
                            .set_fg(self.theme.axis_color);
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
                        buf[(lx, yi)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }
    }
}
