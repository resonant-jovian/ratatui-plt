//! Pie/donut chart widget inspired by matplotlib's `pie`.
//!
//! Renders slices of a circle, optionally with a donut hole, labels, and
//! percentage annotations. Supports exploded slices for emphasis.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::pie_chart::{PieChart, PieSlice};
//! use ratatui::style::Color;
//!
//! let chart = PieChart::new()
//!     .slice(PieSlice::new("Rust", 45.0).color(Color::Cyan))
//!     .slice(PieSlice::new("Python", 30.0).color(Color::Yellow))
//!     .slice(PieSlice::new("C++", 25.0).color(Color::Red))
//!     .title("Language Usage")
//!     .show_percentages(true)
//!     .donut_ratio(0.4);
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA};
use crate::theme::Theme;

/// A single slice of the pie chart.
#[derive(Clone, Debug)]
pub struct PieSlice {
    /// Label for the slice.
    pub label: String,
    /// Numeric value (proportion is computed from sum of all slices).
    pub value: f64,
    /// Fill color for the slice.
    pub color: Option<Color>,
    /// Explode offset as a fraction of the radius (0.0 = no offset).
    pub explode: f64,
}

impl PieSlice {
    /// Create a new slice with the given label and value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
            explode: 0.0,
        }
    }

    /// Set the slice color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the explode offset (fraction of radius, e.g. 0.1).
    pub fn explode(mut self, offset: f64) -> Self {
        self.explode = offset;
        self
    }
}

/// A pie/donut chart widget.
///
/// Renders data as angular slices of a circle. When `donut_ratio` is set,
/// the centre is hollowed out to form a donut chart.
pub struct PieChart {
    /// Slices of the pie.
    slices: Vec<PieSlice>,
    /// Chart title.
    title: Option<String>,
    /// Inner-to-outer radius ratio for donut mode (None = filled pie).
    donut_ratio: Option<f64>,
    /// Whether to draw labels outside the pie.
    show_labels: bool,
    /// Whether to show percentage text on each slice.
    show_percentages: bool,
    /// Visual theme.
    theme: Theme,
}

impl Default for PieChart {
    fn default() -> Self {
        Self {
            slices: Vec::new(),
            title: None,
            donut_ratio: None,
            show_labels: true,
            show_percentages: false,
            theme: Theme::get_default(),
        }
    }
}

impl PieChart {
    /// Create a new empty pie chart.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a slice to the pie chart.
    pub fn slice(mut self, s: PieSlice) -> Self {
        self.slices.push(s);
        self
    }

    /// Add multiple slices at once.
    pub fn slices(mut self, slices: Vec<PieSlice>) -> Self {
        self.slices.extend(slices);
        self
    }

    /// Set the chart title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Enable donut mode with the given inner-to-outer radius ratio (0.0..1.0).
    pub fn donut_ratio(mut self, ratio: f64) -> Self {
        self.donut_ratio = Some(ratio.clamp(0.0, 0.99));
        self
    }

    /// Whether to show labels positioned outside the pie.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Whether to show percentage text on or near each slice.
    pub fn show_percentages(mut self, show: bool) -> Self {
        self.show_percentages = show;
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for &PieChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 8 || area.height < 6 || self.slices.is_empty() {
            return;
        }

        let total: f64 = self.slices.iter().map(|s| s.value.max(0.0)).sum();
        if total <= 0.0 {
            return;
        }

        let mut pb = PlotBuffer::new(area);

        // Reserve space for title
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let py = area.y + title_height;
        let ph = area.height.saturating_sub(title_height);

        // Reserve margin for labels on left/right
        let label_margin: u16 = if self.show_labels { 10 } else { 0 };
        let pw = area.width;

        if pw < label_margin * 2 + 4 || ph < 4 {
            return;
        }

        // Draw title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    pb.set_char(x, area.y, ch, self.theme.foreground, Z_CHROME);
                }
            }
        }

        // Centre of the pie in screen coordinates
        let cx = area.x as f64 + pw as f64 / 2.0;
        let cy = py as f64 + ph as f64 / 2.0;

        // Radius (account for terminal cells being ~2x tall as wide)
        let r_screen_x = ((pw as f64 / 2.0) - label_margin as f64 - 1.0).max(2.0);
        let r_screen_y = if self.show_labels || self.show_percentages {
            ((ph as f64 / 2.3) - 1.0).max(2.0)
        } else {
            ((ph as f64 / 2.0) - 1.0).max(2.0)
        };

        let inner_ratio = self.donut_ratio.unwrap_or(0.0);

        // Precompute slice angles (cumulative)
        let mut angles: Vec<(f64, f64)> = Vec::with_capacity(self.slices.len());
        let mut angle_start = -std::f64::consts::FRAC_PI_2; // Start from top (12 o'clock)
        for slice in &self.slices {
            let fraction = slice.value.max(0.0) / total;
            let sweep = fraction * 2.0 * std::f64::consts::PI;
            angles.push((angle_start, angle_start + sweep));
            angle_start += sweep;
        }

        // Pre-resolve slice colors
        let slice_colors: Vec<Color> = self
            .slices
            .iter()
            .enumerate()
            .map(|(i, s)| s.color.unwrap_or_else(|| self.theme.color_cycle.at(i)))
            .collect();

        // For each cell in the drawing area, determine which slice it belongs to
        for screen_y in py..py + ph {
            for screen_x in area.x..area.x + pw {
                // Normalised position relative to centre, accounting for cell aspect ratio
                let nx = (screen_x as f64 - cx) / r_screen_x;
                let ny = (screen_y as f64 - cy) / r_screen_y;
                let r = (nx * nx + ny * ny).sqrt();

                // Check if within pie annulus
                if r > 1.0 || r < inner_ratio {
                    continue;
                }

                // Find which slice this angle belongs to
                for (si, (slice, &(a_start, a_end))) in self.slices.iter().zip(angles.iter()).enumerate() {
                    // Handle exploded slices by shifting the centre
                    let (ecx, ecy) = if slice.explode > 0.0 {
                        let mid_angle = (a_start + a_end) / 2.0;
                        let ex = slice.explode * mid_angle.cos() * r_screen_x;
                        let ey = slice.explode * mid_angle.sin() * r_screen_y;
                        (cx + ex, cy + ey)
                    } else {
                        (cx, cy)
                    };

                    // Recompute normalised coords relative to (possibly shifted) centre
                    let enx = (screen_x as f64 - ecx) / r_screen_x;
                    let eny = (screen_y as f64 - ecy) / r_screen_y;
                    let er = (enx * enx + eny * eny).sqrt();

                    if er > 1.0 || er < inner_ratio {
                        continue;
                    }

                    let mut ea = eny.atan2(enx);

                    // Normalise angle into the range [a_start, a_start + 2*PI)
                    while ea < a_start {
                        ea += 2.0 * std::f64::consts::PI;
                    }
                    while ea >= a_start + 2.0 * std::f64::consts::PI {
                        ea -= 2.0 * std::f64::consts::PI;
                    }

                    if ea >= a_start && ea < a_end {
                        let sc = slice_colors[si];
                        pb.set_cell(screen_x, screen_y, '█', sc, sc, Z_DATA);
                        break;
                    }
                }
            }
        }

        // Draw slice border outlines (use thin ring on the outer and inner edges)
        // Draw radial borders between slices
        for &(a_start, _a_end) in &angles {
            let steps = (r_screen_x.max(r_screen_y) * 1.5) as usize;
            for s in 0..=steps {
                let frac = inner_ratio + (1.0 - inner_ratio) * s as f64 / steps as f64;
                let sx = cx + frac * r_screen_x * a_start.cos();
                let sy = cy + frac * r_screen_y * a_start.sin();
                let xi = sx.round() as u16;
                let yi = sy.round() as u16;
                if xi >= area.x && xi < area.x + area.width && yi >= py && yi < py + ph {
                    pb.set_char(xi, yi, '▪', self.theme.muted, Z_CHROME);
                }
            }
        }

        // When unicode-extended is enabled, draw arc quadrant characters along the
        // outer rim for smoother circular edges.
        #[cfg(feature = "unicode-extended")]
        {
            let n_arc = (2.0 * std::f64::consts::PI * r_screen_x.max(r_screen_y))
                .round()
                .max(24.0) as usize;
            for step in 0..n_arc {
                let theta = 2.0 * std::f64::consts::PI * step as f64 / n_arc as f64;
                let sx = cx + r_screen_x * theta.cos();
                let sy = cy + r_screen_y * theta.sin();
                let xi = sx.round() as u16;
                let yi = sy.round() as u16;
                if xi >= area.x && xi < area.x + area.width && yi >= py && yi < py + ph {
                    // Choose arc quadrant character based on which quadrant of
                    // the circle this point falls in:
                    //   ◜ upper-left   ◝ upper-right
                    //   ◟ lower-left   ◞ lower-right
                    let arc_ch = match (theta.cos() >= 0.0, theta.sin() < 0.0) {
                        (false, true) => '◜',  // upper-left quadrant
                        (true, true) => '◝',   // upper-right quadrant
                        (true, false) => '◞',  // lower-right quadrant
                        (false, false) => '◟', // lower-left quadrant
                    };
                    pb.set_char(xi, yi, arc_ch, self.theme.muted, Z_CHROME);
                }
            }
        }

        // Draw labels and/or percentages outside the pie
        if self.show_labels || self.show_percentages {
            for (i, slice) in self.slices.iter().enumerate() {
                let (a_start, a_end) = angles[i];
                let mid_angle = (a_start + a_end) / 2.0;
                let fraction = slice.value.max(0.0) / total;

                // Build the label text
                let mut text = String::new();
                if self.show_labels {
                    text.push_str(&slice.label);
                }
                if self.show_percentages {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(&format!("{:.1}%", fraction * 100.0));
                }

                if text.is_empty() {
                    continue;
                }

                // Position label outside the pie
                let label_r = 1.15 + slice.explode;
                let lx = cx + label_r * r_screen_x * mid_angle.cos();
                let ly = cy + label_r * r_screen_y * mid_angle.sin();

                // Adjust horizontal position: if on the left side, right-align
                let xi = if mid_angle.cos() < 0.0 {
                    (lx - text.len() as f64).round() as i32
                } else {
                    lx.round() as i32
                };
                let yi = ly.round() as u16;

                if yi >= py && yi < py + ph {
                    for (j, ch) in text.chars().enumerate() {
                        let x = xi + j as i32;
                        if x >= area.x as i32 && x < (area.x + area.width) as i32 {
                            pb.set_char(x as u16, yi, ch, slice_colors[i], Z_CHROME);
                        }
                    }
                }
            }
        }

        pb.composite(buf);
    }
}
