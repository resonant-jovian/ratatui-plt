//! Legend widget for displaying series identification.
//!
//! Renders a legend box showing series names with their corresponding
//! colors and marker styles.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::series::Series;
use crate::theme::Theme;

/// Legend position within the plot area.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum LegendPosition {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
    /// Custom position (x, y) in characters from top-left of plot area.
    Custom(u16, u16),
}

/// Legend entry for a single series.
#[derive(Clone, Debug)]
pub struct LegendEntry {
    /// Series name.
    pub name: String,
    /// Series color.
    pub color: Color,
    /// Marker character (if any).
    pub marker: Option<char>,
}

impl LegendEntry {
    /// Create from a Series.
    pub fn from_series(series: &Series) -> Self {
        Self {
            name: series.name.clone(),
            color: series.color,
            marker: series.marker.map(|m| m.char()),
        }
    }
}

/// Legend widget showing series identification.
///
/// # Example
///
/// ```
/// use ratatui_plt::legend::{Legend, LegendPosition, LegendEntry};
/// use ratatui::style::Color;
///
/// let legend = Legend::new(vec![
///     LegendEntry { name: "Series A".into(), color: Color::Red, marker: Some('●') },
///     LegendEntry { name: "Series B".into(), color: Color::Blue, marker: Some('■') },
/// ]).position(LegendPosition::TopRight);
/// ```
#[derive(Clone, Debug)]
pub struct Legend {
    /// Legend entries.
    pub entries: Vec<LegendEntry>,
    /// Position within the plot area.
    pub position: LegendPosition,
    /// Whether to draw a border.
    pub border: bool,
    /// Theme for styling.
    pub theme: Theme,
}

impl Legend {
    /// Create a new legend from entries.
    pub fn new(entries: Vec<LegendEntry>) -> Self {
        Self {
            entries,
            position: LegendPosition::default(),
            border: true,
            theme: Theme::get_default(),
        }
    }

    /// Create a legend from a slice of Series.
    pub fn from_series(series: &[Series]) -> Self {
        let entries = series.iter().map(LegendEntry::from_series).collect();
        Self::new(entries)
    }

    /// Set the position.
    pub fn position(mut self, pos: LegendPosition) -> Self {
        self.position = pos;
        self
    }

    /// Enable or disable the border.
    pub fn border(mut self, show: bool) -> Self {
        self.border = show;
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Compute the required size (width, height) for this legend.
    pub fn size(&self) -> (u16, u16) {
        if self.entries.is_empty() {
            return (0, 0);
        }
        let max_name_len = self.entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
        let width = max_name_len as u16 + 4; // "● Name" + padding
        let height = self.entries.len() as u16 + if self.border { 2 } else { 0 };
        let width = width + if self.border { 2 } else { 0 };
        (width, height)
    }

    /// Compute the position rect within the given area.
    pub fn position_rect(&self, area: Rect) -> Rect {
        let (w, h) = self.size();
        let w = w.min(area.width);
        let h = h.min(area.height);

        let (x, y) = match &self.position {
            LegendPosition::TopLeft => (area.x + 1, area.y + 1),
            LegendPosition::TopRight => (area.x + area.width.saturating_sub(w + 1), area.y + 1),
            LegendPosition::BottomLeft => (area.x + 1, area.y + area.height.saturating_sub(h + 1)),
            LegendPosition::BottomRight => (
                area.x + area.width.saturating_sub(w + 1),
                area.y + area.height.saturating_sub(h + 1),
            ),
            LegendPosition::Custom(cx, cy) => (area.x + cx, area.y + cy),
        };

        Rect::new(x, y, w, h)
    }
}

impl Widget for &Legend {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rect = self.position_rect(area);
        if rect.width < 3 || rect.height < 1 {
            return;
        }

        // Clear background
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                if x < area.x + area.width && y < area.y + area.height {
                    buf[(x, y)].set_char(' ').set_style(Style::default());
                }
            }
        }

        // Draw border
        if self.border {
            let r = rect;
            let bc = self.theme.axis_color;
            if r.width >= 2 && r.height >= 2 {
                buf[(r.x, r.y)].set_char('┌').set_fg(bc);
                buf[(r.x + r.width - 1, r.y)].set_char('┐').set_fg(bc);
                buf[(r.x, r.y + r.height - 1)].set_char('└').set_fg(bc);
                buf[(r.x + r.width - 1, r.y + r.height - 1)].set_char('┘').set_fg(bc);
                for x in r.x + 1..r.x + r.width - 1 {
                    buf[(x, r.y)].set_char('─').set_fg(bc);
                    buf[(x, r.y + r.height - 1)].set_char('─').set_fg(bc);
                }
                for y in r.y + 1..r.y + r.height - 1 {
                    buf[(r.x, y)].set_char('│').set_fg(bc);
                    buf[(r.x + r.width - 1, y)].set_char('│').set_fg(bc);
                }
            }
        }

        // Draw entries
        let start_y = rect.y + if self.border { 1 } else { 0 };
        let start_x = rect.x + if self.border { 1 } else { 0 };
        let inner_width = rect.width.saturating_sub(if self.border { 2 } else { 0 });

        for (i, entry) in self.entries.iter().enumerate() {
            let y = start_y + i as u16;
            if y >= rect.y + rect.height - if self.border { 1 } else { 0 } {
                break;
            }

            // Draw marker/color indicator
            let marker_char = entry.marker.unwrap_or('━');
            if start_x < area.x + area.width {
                buf[(start_x, y)].set_char(marker_char).set_fg(entry.color);
            }

            // Draw name
            let name_x = start_x + 2;
            for (j, ch) in entry.name.chars().enumerate() {
                let x = name_x + j as u16;
                if x < start_x + inner_width && x < area.x + area.width {
                    buf[(x, y)]
                        .set_char(ch)
                        .set_style(Style::default().fg(self.theme.foreground));
                }
            }
        }
    }
}
