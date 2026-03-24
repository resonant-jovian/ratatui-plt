//! Funnel chart widget for sales/conversion funnel visualization.
//!
//! Renders horizontally centered decreasing-width bars, commonly used to show
//! drop-off rates in sales or conversion pipelines.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::funnel::{FunnelChart, FunnelEntry};
//! use ratatui::style::Color;
//!
//! let chart = FunnelChart::new()
//!     .entry(FunnelEntry::new("Visitors", 1000.0).color(Color::Cyan))
//!     .entry(FunnelEntry::new("Leads", 600.0).color(Color::Yellow))
//!     .entry(FunnelEntry::new("Prospects", 300.0).color(Color::Green))
//!     .entry(FunnelEntry::new("Sales", 100.0).color(Color::Red))
//!     .title("Sales Funnel");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::color_cycle::ColorCycle;
use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA};
use crate::theme::Theme;

/// A single entry in a funnel chart.
#[derive(Clone, Debug)]
pub struct FunnelEntry {
    /// Label displayed to the left of the bar.
    pub label: String,
    /// Numeric value (determines bar width).
    pub value: f64,
    /// Bar color (if None, auto-assigned from color cycle).
    pub color: Option<Color>,
}

impl FunnelEntry {
    /// Create a new funnel entry with the given label and value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
        }
    }

    /// Set the bar color.
    pub fn color(mut self, c: Color) -> Self {
        self.color = Some(c);
        self
    }
}

/// A funnel chart widget showing decreasing-width horizontal bars.
///
/// Commonly used for sales funnels, conversion pipelines, and
/// other visualizations showing progressive reduction.
pub struct FunnelChart {
    entries: Vec<FunnelEntry>,
    show_percentages: bool,
    show_values: bool,
    title: Option<String>,
    theme: Theme,
}

impl Default for FunnelChart {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            show_percentages: true,
            show_values: true,
            title: None,
            theme: Theme::get_default(),
        }
    }
}

impl FunnelChart {
    /// Create a new empty funnel chart.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single entry.
    pub fn entry(mut self, entry: FunnelEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Add multiple entries at once.
    pub fn entries(mut self, entries: Vec<FunnelEntry>) -> Self {
        self.entries.extend(entries);
        self
    }

    /// Whether to show percentage labels (relative to first entry).
    pub fn show_percentages(mut self, show: bool) -> Self {
        self.show_percentages = show;
        self
    }

    /// Whether to show value labels.
    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for &FunnelChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 8 || area.height < 4 || self.entries.is_empty() {
            return;
        }

        // Reserve space for title
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };

        let mut pb = PlotBuffer::new(area);

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

        let draw_y = area.y + title_height;
        let draw_h = area.height.saturating_sub(title_height);
        if draw_h < 2 {
            return;
        }

        // Find max label length for left margin
        let max_label_len = self
            .entries
            .iter()
            .map(|e| e.label.len())
            .max()
            .unwrap_or(0) as u16;
        let label_margin = max_label_len + 1;

        // Space for value/percentage text on the right
        let right_margin: u16 = if self.show_values || self.show_percentages {
            12
        } else {
            1
        };

        let bar_area_x = area.x + label_margin;
        let bar_area_w = area
            .width
            .saturating_sub(label_margin + right_margin)
            .max(2);

        let n = self.entries.len();
        let entry_height = draw_h / n as u16;
        if entry_height == 0 {
            return;
        }

        // Compute max value
        let max_value = self.entries.iter().map(|e| e.value).fold(0.0f64, f64::max);
        if max_value <= 0.0 {
            return;
        }

        let first_value = self
            .entries
            .first()
            .map(|e| e.value)
            .unwrap_or(1.0)
            .max(f64::MIN_POSITIVE);

        let mut cycle = ColorCycle::default();

        for (i, entry) in self.entries.iter().enumerate() {
            let row_y = draw_y + i as u16 * entry_height;
            let bar_center_y = row_y + entry_height / 2;

            // Compute bar width proportional to value
            let bar_width = ((entry.value / max_value) * bar_area_w as f64).round() as u16;
            let bar_width = bar_width.max(1).min(bar_area_w);

            // Center horizontally within bar area
            let bar_x = bar_area_x + (bar_area_w.saturating_sub(bar_width)) / 2;

            // Get color
            let color = entry.color.unwrap_or_else(|| cycle.next_color());
            if entry.color.is_some() {
                // Advance the cycle even when color is set, for consistency
                let _ = cycle.next_color();
            }

            // Draw bar (fill the height of the entry row)
            let bar_y_start = row_y;
            let bar_y_end = (row_y + entry_height).min(draw_y + draw_h);
            for y in bar_y_start..bar_y_end {
                for x in bar_x..bar_x + bar_width {
                    if x < area.x + area.width && y < area.y + area.height {
                        pb.set_cell(x, y, self.theme.chars.fill.solid, color, color, Z_DATA);
                    }
                }
            }

            // Draw label to the left
            let label_x = area.x;
            let label_y = bar_center_y;
            if label_y < area.y + area.height {
                for (j, ch) in entry.label.chars().enumerate() {
                    let lx = label_x + j as u16;
                    if lx < bar_area_x {
                        pb.set_char(lx, label_y, ch, self.theme.foreground, Z_CHROME);
                    }
                }
            }

            // Build right-side text
            let mut right_text = String::new();
            if self.show_values {
                // Format value compactly
                if entry.value == entry.value.floor() {
                    right_text = format!("{}", entry.value as i64);
                } else {
                    right_text = format!("{:.1}", entry.value);
                }
            }
            if self.show_percentages {
                let pct = (entry.value / first_value) * 100.0;
                if !right_text.is_empty() {
                    right_text.push(' ');
                }
                right_text.push_str(&format!("{:.0}%", pct));
            }

            // Draw right-side text: inside bar if wide enough, else to the right
            if !right_text.is_empty() && bar_center_y < area.y + area.height {
                let text_len = right_text.len() as u16;
                let text_x = if bar_width > text_len + 2 {
                    // Inside bar, centered
                    bar_x + (bar_width.saturating_sub(text_len)) / 2
                } else {
                    // To the right of bar
                    bar_x + bar_width + 1
                };

                for (j, ch) in right_text.chars().enumerate() {
                    let lx = text_x + j as u16;
                    if lx < area.x + area.width {
                        let fg = if bar_width > text_len + 2 {
                            self.theme.background
                        } else {
                            self.theme.foreground
                        };
                        pb.set_char(lx, bar_center_y, ch, fg, Z_CHROME);
                    }
                }
            }
        }

        pb.composite(buf);
    }
}
