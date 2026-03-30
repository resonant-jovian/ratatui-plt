//! Funnel area chart widget with proportional trapezoid sections.
//!
//! Unlike [`FunnelChart`](super::funnel::FunnelChart) which uses equal-height bars,
//! `FunnelArea` renders each stage as a trapezoid whose width is proportional to
//! its value, creating a smooth tapering funnel shape.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::funnel_area::{FunnelArea, FunnelAreaEntry};
//! use ratatui::style::Color;
//!
//! let chart = FunnelArea::new()
//!     .entry(FunnelAreaEntry::new("Visitors", 1000.0).color(Color::Cyan))
//!     .entry(FunnelAreaEntry::new("Leads", 600.0).color(Color::Yellow))
//!     .entry(FunnelAreaEntry::new("Sales", 100.0).color(Color::Red))
//!     .title("Conversion Funnel");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::color_cycle::ColorCycle;
use crate::plot_buffer::{PlotBackend, Z_CHROME, Z_DATA, create_backend};
use crate::spines::Spines;
use crate::theme::Theme;

/// A single entry in a funnel area chart.
#[derive(Clone, Debug)]
pub struct FunnelAreaEntry {
    /// Label displayed within the trapezoid section.
    pub label: String,
    /// Numeric value (determines trapezoid width).
    pub value: f64,
    /// Section color (if None, auto-assigned from color cycle).
    pub color: Option<Color>,
}

impl FunnelAreaEntry {
    /// Create a new funnel area entry with the given label and value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
        }
    }

    /// Set the section color.
    pub fn color(mut self, c: Color) -> Self {
        self.color = Some(c);
        self
    }
}

/// A funnel area chart widget showing proportional trapezoid sections.
///
/// Each stage is a trapezoid whose top and bottom widths are proportional to
/// the current and next entry values, creating a smooth tapering funnel.
pub struct FunnelArea {
    entries: Vec<FunnelAreaEntry>,
    title: Option<String>,
    show_values: bool,
    show_percentages: bool,
    theme: Theme,
    spines: Spines,
}

impl Default for FunnelArea {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            title: None,
            show_values: true,
            show_percentages: true,
            theme: Theme::get_default(),
            spines: Spines::default(),
        }
    }
}

impl FunnelArea {
    /// Create a new empty funnel area chart.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single entry.
    pub fn entry(mut self, entry: FunnelAreaEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Add multiple entries at once.
    pub fn entries(mut self, entries: Vec<FunnelAreaEntry>) -> Self {
        self.entries.extend(entries);
        self
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Whether to show value labels.
    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Whether to show percentage labels (relative to first entry).
    pub fn show_percentages(mut self, show: bool) -> Self {
        self.show_percentages = show;
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set spine visibility.
    pub fn spines(mut self, spines: Spines) -> Self {
        self.spines = spines;
        self
    }
}


impl Widget for &FunnelArea {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 4 || self.entries.is_empty() {
            return;
        }

        let mut pb = create_backend(area);

        // Reserve space for title
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };

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

        let n = self.entries.len();
        let entry_height = draw_h / n as u16;
        if entry_height == 0 {
            return;
        }

        // Compute max value for width scaling
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

        // Usable width for the funnel (leave some margin)
        let margin = 2u16;
        let funnel_width = area.width.saturating_sub(margin * 2);
        if funnel_width < 4 {
            return;
        }
        let center_x = area.x + area.width / 2;

        // Precompute half-widths for each entry (proportional to value)
        let half_widths: Vec<u16> = self
            .entries
            .iter()
            .map(|e| {
                let w = ((e.value / max_value) * (funnel_width as f64) / 2.0).round() as u16;
                w.max(1)
            })
            .collect();

        let mut cycle = ColorCycle::default();

        // Draw border top line if spines.top is set
        if self.spines.top {
            let border = &self.theme.chars.border;
            for x in area.x..area.x + area.width {
                pb.set_char(
                    x,
                    draw_y,
                    border.horizontal,
                    self.theme.axis_color,
                    Z_CHROME,
                );
            }
        }

        for (i, entry) in self.entries.iter().enumerate() {
            let row_y = draw_y + i as u16 * entry_height;

            // Top half-width for this section
            let top_hw = half_widths[i];
            // Bottom half-width: use next entry's width, or taper to a small point
            let bottom_hw = if i + 1 < n {
                half_widths[i + 1]
            } else {
                // Last section tapers to a narrow bottom
                (top_hw / 3).max(1)
            };

            // Get color
            let color = entry.color.unwrap_or_else(|| cycle.next_color());
            if entry.color.is_some() {
                let _ = cycle.next_color();
            }

            // Draw the trapezoid row by row
            let section_end = (row_y + entry_height).min(draw_y + draw_h);
            let section_h = section_end.saturating_sub(row_y);
            if section_h == 0 {
                continue;
            }

            for dy in 0..section_h {
                // Interpolate half-width from top to bottom across the section height
                let t = if section_h > 1 {
                    dy as f64 / (section_h - 1) as f64
                } else {
                    0.0
                };
                let hw = top_hw as f64 * (1.0 - t) + bottom_hw as f64 * t;
                let hw_int = hw.round() as u16;

                let left = center_x.saturating_sub(hw_int);
                let right = (center_x + hw_int).min(area.x + area.width);
                let y = row_y + dy;

                if y >= area.y + area.height {
                    break;
                }

                for x in left..right {
                    if x >= area.x && x < area.x + area.width {
                        pb.set_cell(x, y, self.theme.chars.fill.solid, color, color, Z_DATA);
                    }
                }

                // Draw angled edges with theme border chars
                if left > area.x {
                    pb.set_char(
                        left,
                        y,
                        self.theme.chars.border.vertical,
                        self.theme.axis_color,
                        Z_CHROME,
                    );
                }
                if right > 0 && right - 1 < area.x + area.width {
                    pb.set_char(
                        right.saturating_sub(1),
                        y,
                        self.theme.chars.border.vertical,
                        self.theme.axis_color,
                        Z_CHROME,
                    );
                }
            }

            // Draw centered label within the trapezoid
            let label_y = row_y + section_h / 2;
            if label_y < area.y + area.height {
                // Build label text
                let mut label_text = entry.label.clone();
                if self.show_values {
                    let value_str = if entry.value == entry.value.floor() {
                        format!(" {}", entry.value as i64)
                    } else {
                        format!(" {:.1}", entry.value)
                    };
                    label_text.push_str(&value_str);
                }
                if self.show_percentages {
                    let pct = (entry.value / first_value) * 100.0;
                    label_text.push_str(&format!(" ({:.0}%)", pct));
                }

                let label_len = label_text.len() as u16;
                // Compute the half-width at label_y
                let t_label = if section_h > 1 {
                    (section_h / 2) as f64 / (section_h - 1) as f64
                } else {
                    0.0
                };
                let hw_at_label =
                    (top_hw as f64 * (1.0 - t_label) + bottom_hw as f64 * t_label).round() as u16;
                let available = hw_at_label * 2;

                if label_len <= available {
                    // Center within the trapezoid
                    let label_x = center_x.saturating_sub(label_len / 2);
                    for (j, ch) in label_text.chars().enumerate() {
                        let lx = label_x + j as u16;
                        if lx < area.x + area.width {
                            pb.set_char(lx, label_y, ch, self.theme.background, Z_CHROME);
                        }
                    }
                } else {
                    // Truncate and place what fits
                    let fit_chars = available.saturating_sub(1) as usize;
                    let label_x = center_x.saturating_sub(available / 2);
                    for (j, ch) in label_text.chars().take(fit_chars).enumerate() {
                        let lx = label_x + j as u16;
                        if lx < area.x + area.width {
                            pb.set_char(lx, label_y, ch, self.theme.background, Z_CHROME);
                        }
                    }
                }
            }
        }

        pb.composite(buf);
    }
}
