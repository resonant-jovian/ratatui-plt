//! Boxen plot (letter-value plot) widget for detailed distribution visualization.
//!
//! A letter-value plot shows nested boxes for increasingly extreme quantiles,
//! revealing more structure than a standard box plot. Each successive level
//! represents a deeper quantile split (fourths, eighths, sixteenths, etc.).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{PlotFrame, ReferenceLine};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::ticker::NullLocator;

/// A single group (category) of data in a boxen plot.
#[derive(Clone, Debug)]
pub struct BoxenGroup {
    /// Category label.
    pub name: String,
    /// Raw data values.
    pub data: Vec<f64>,
    /// Box color.
    pub color: Color,
}

impl BoxenGroup {
    /// Create a new boxen group.
    pub fn new(name: impl Into<String>, data: Vec<f64>, color: Color) -> Self {
        Self {
            name: name.into(),
            data,
            color,
        }
    }

    /// Return sorted finite values.
    fn sorted_finite(&self) -> Vec<f64> {
        let mut v: Vec<f64> = self
            .data
            .iter()
            .copied()
            .filter(|x| x.is_finite())
            .collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v
    }
}

/// Compute a percentile from sorted data using linear interpolation.
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

/// A boxen (letter-value) plot widget.
///
/// Shows nested boxes for increasingly extreme quantiles, revealing more
/// detail about the distribution tails than a standard box plot. Each level
/// splits the data further: level 1 = fourths (25th/75th), level 2 = eighths
/// (12.5th/87.5th), etc.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::widgets::boxen::{BoxenPlot, BoxenGroup};
///
/// let plot = BoxenPlot::new()
///     .group(BoxenGroup::new("A", vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], Color::Cyan))
///     .group(BoxenGroup::new("B", vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], Color::Yellow))
///     .title("Letter-Value Plot");
/// ```
pub struct BoxenPlot {
    groups: Vec<BoxenGroup>,
    title: Option<String>,
    y_axis: Axis,
    k_depth: Option<usize>,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for BoxenPlot {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            title: None,
            y_axis: Axis::new(),
            k_depth: None,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl BoxenPlot {
    /// Create a new empty boxen plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a group (category) of data.
    pub fn group(mut self, g: BoxenGroup) -> Self {
        self.groups.push(g);
        self
    }

    /// Set the plot title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Set the y-axis configuration.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the quantile depth. `None` = auto (ceil(log2(n)) - 1).
    pub fn k_depth(mut self, k: Option<usize>) -> Self {
        self.k_depth = k;
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

/// Compute the auto k_depth for letter-value plots: ceil(log2(n)) - 1.
fn auto_k_depth(n: usize) -> usize {
    if n < 4 {
        return 1;
    }
    let k = (n as f64).log2().ceil() as usize;
    k.saturating_sub(1).max(1)
}

/// Compute the letter-value quantile levels.
///
/// Returns a list of (lower_percentile, upper_percentile) pairs.
/// Level 0 = fourths (25/75), level 1 = eighths (12.5/87.5), etc.
fn letter_value_levels(k: usize) -> Vec<(f64, f64)> {
    let mut levels = Vec::new();
    for i in 0..k {
        let divisor = 2.0_f64.powi(i as i32 + 2);
        let lower = 100.0 / divisor;
        let upper = 100.0 - lower;
        levels.push((lower, upper));
    }
    levels
}

/// Shading characters for successive box levels (innermost to outermost).
const LEVEL_CHARS: [char; 4] = ['█', '▓', '▒', '░'];

/// Get the fill character for a given level index.
fn level_char(level: usize) -> char {
    LEVEL_CHARS[level.min(LEVEL_CHARS.len() - 1)]
}

/// Compute a depth-based color gradient for letter-value levels.
///
/// Level 0 (innermost / IQR) gets the brightest/most saturated color.
/// Deeper levels get progressively darker, creating a visual gradient
/// that emphasizes the center of the distribution.
fn depth_gradient_color(base: Color, level_idx: usize, total_levels: usize) -> Color {
    match base {
        Color::Rgb(r, g, b) => {
            // Scale brightness: level 0 = full brightness, deepest = ~30%
            let t = if total_levels > 1 {
                level_idx as f64 / (total_levels - 1) as f64
            } else {
                0.0
            };
            let scale = 1.0 - 0.7 * t;
            Color::Rgb(
                (r as f64 * scale).round() as u8,
                (g as f64 * scale).round() as u8,
                (b as f64 * scale).round() as u8,
            )
        }
        Color::Indexed(idx) => {
            // For indexed colors, map to an RGB approximation and darken
            let (r, g, b) = indexed_to_rgb(idx);
            let t = if total_levels > 1 {
                level_idx as f64 / (total_levels - 1) as f64
            } else {
                0.0
            };
            let scale = 1.0 - 0.7 * t;
            Color::Rgb(
                (r as f64 * scale).round() as u8,
                (g as f64 * scale).round() as u8,
                (b as f64 * scale).round() as u8,
            )
        }
        // Named ANSI colors: convert to RGB, darken
        named => {
            let (r, g, b) = named_color_to_rgb(named);
            let t = if total_levels > 1 {
                level_idx as f64 / (total_levels - 1) as f64
            } else {
                0.0
            };
            let scale = 1.0 - 0.7 * t;
            Color::Rgb(
                (r as f64 * scale).round() as u8,
                (g as f64 * scale).round() as u8,
                (b as f64 * scale).round() as u8,
            )
        }
    }
}

/// Convert a named ratatui Color to approximate RGB values.
fn named_color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::Gray => (170, 170, 170),
        Color::DarkGray => (118, 118, 118),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        Color::White => (255, 255, 255),
        _ => (200, 200, 200),
    }
}

/// Convert a 256-color indexed value to approximate RGB.
fn indexed_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0 => (0, 0, 0),
        1 => (205, 49, 49),
        2 => (13, 188, 121),
        3 => (229, 229, 16),
        4 => (36, 114, 200),
        5 => (188, 63, 188),
        6 => (17, 168, 205),
        7 => (170, 170, 170),
        8 => (118, 118, 118),
        9 => (241, 76, 76),
        10 => (35, 209, 139),
        11 => (245, 245, 67),
        12 => (59, 142, 234),
        13 => (214, 112, 214),
        14 => (41, 184, 219),
        15 => (255, 255, 255),
        // 6x6x6 color cube (16-231)
        16..=231 => {
            let c = idx - 16;
            let r = c / 36;
            let g = (c % 36) / 6;
            let b = c % 6;
            let to_val = |v: u8| if v == 0 { 0u8 } else { 55 + 40 * v };
            (to_val(r), to_val(g), to_val(b))
        }
        // Grayscale ramp (232-255)
        232..=255 => {
            let v = 8 + 10 * (idx - 232);
            (v, v, v)
        }
    }
}

impl Widget for &BoxenPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.groups.is_empty() {
            return;
        }

        // Compute global y range
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for g in &self.groups {
            for &v in &g.data {
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

        // Use NullLocator for x-axis (categories drawn manually)
        let x_axis = Axis::new().locator(NullLocator);
        let n = self.groups.len();
        let x_lo = 0.0;
        let x_hi = n as f64;

        // Create and render the plot frame
        let frame = PlotFrame::new(&x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, x_lo, x_hi, y_lo, y_hi) else {
            return;
        };

        let col_width = pa.width / n.max(1) as u16;
        // Maximum box width (widest level)
        let max_box_width = col_width.saturating_sub(2).max(3);

        for (i, group) in self.groups.iter().enumerate() {
            let sorted = group.sorted_finite();
            if sorted.is_empty() {
                continue;
            }

            let center_x = pa.x + (i as u16 * pa.width / n as u16) + pa.width / n as u16 / 2;

            // Determine k_depth
            let k = self.k_depth.unwrap_or_else(|| auto_k_depth(sorted.len()));
            let levels = letter_value_levels(k);

            // Compute median
            let median = percentile(&sorted, 50.0);

            // Draw levels from outermost (widest quantile range, narrowest box)
            // to innermost (IQR, widest box) so inner boxes paint over outer.
            for (level_idx, &(lower_pct, upper_pct)) in levels.iter().enumerate().rev() {
                let lower_val = percentile(&sorted, lower_pct);
                let upper_val = percentile(&sorted, upper_pct);

                let sy_lower = pa.screen_y(lower_val).round() as u16;
                let sy_upper = pa.screen_y(upper_val).round() as u16;

                // Box width: level 0 (fourths) is widest, each subsequent level narrower.
                // Width decreases proportionally: level 0 = max, level k-1 = ~30% of max.
                let width_fraction = if k > 1 {
                    1.0 - 0.7 * (level_idx as f64 / (k as f64 - 1.0).max(1.0))
                } else {
                    1.0
                };
                let box_width = ((max_box_width as f64 * width_fraction).round() as u16).max(1);
                let box_left = center_x.saturating_sub(box_width / 2);
                let box_right = box_left + box_width;

                let fill_ch = level_char(level_idx);

                // The top of the box is sy_upper (smaller y = higher on screen)
                // The bottom is sy_lower (larger y = lower on screen)
                let top = sy_upper.min(sy_lower);
                let bottom = sy_upper.max(sy_lower);

                // Compute depth-based color for fill: deeper levels use more saturated/darker colors
                let fill_color = depth_gradient_color(group.color, level_idx, k);

                // Fill the box
                for y in top..=bottom {
                    for x in box_left..box_right {
                        if pa.contains(x, y) {
                            buf[(x, y)].set_char(fill_ch).set_fg(fill_color);
                        }
                    }
                }

                // Compute depth-based color: deeper levels get more saturated/darker
                let depth_color = depth_gradient_color(group.color, level_idx, k);

                // Draw box outline: top edge with corners
                if pa.contains(box_left, top) {
                    buf[(box_left, top)].set_char('┌').set_fg(depth_color);
                }
                for x in (box_left + 1)..box_right.saturating_sub(1) {
                    if pa.contains(x, top) {
                        buf[(x, top)].set_char('─').set_fg(depth_color);
                    }
                }
                if box_right > box_left + 1 && pa.contains(box_right - 1, top) {
                    buf[(box_right - 1, top)].set_char('┐').set_fg(depth_color);
                }
                // Bottom edge with corners
                if pa.contains(box_left, bottom) {
                    buf[(box_left, bottom)].set_char('└').set_fg(depth_color);
                }
                for x in (box_left + 1)..box_right.saturating_sub(1) {
                    if pa.contains(x, bottom) {
                        buf[(x, bottom)].set_char('─').set_fg(depth_color);
                    }
                }
                if box_right > box_left + 1 && pa.contains(box_right - 1, bottom) {
                    buf[(box_right - 1, bottom)].set_char('┘').set_fg(depth_color);
                }
                // Side edges — skip corner rows
                for y in (top + 1)..bottom {
                    if pa.contains(box_left, y) {
                        buf[(box_left, y)].set_char('│').set_fg(depth_color);
                    }
                    if box_right > 0 && pa.contains(box_right - 1, y) {
                        buf[(box_right - 1, y)].set_char('│').set_fg(depth_color);
                    }
                }
            }

            // Draw median line on top
            let sy_median = pa.screen_y(median).round() as u16;
            let median_left = center_x.saturating_sub(max_box_width / 2);
            let median_right = median_left + max_box_width;
            for x in median_left..median_right {
                if pa.contains(x, sy_median) {
                    buf[(x, sy_median)].set_char('━').set_fg(group.color);
                }
            }

            // Category label
            let label = &group.name;
            let label_start = center_x.saturating_sub(label.len() as u16 / 2);
            let label_y = pa.y + pa.height;
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

        // Draw annotations
        PlotFrame::draw_annotations(&pa, &self.annotations, buf);
    }
}
