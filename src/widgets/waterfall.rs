//! Waterfall chart widget for cumulative positive/negative value visualization.
//!
//! Shows the cumulative effect of positive and negative values. Each bar starts
//! where the previous bar ended, making it easy to see how values add up.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::waterfall::{WaterfallChart, WaterfallEntry};
//! use ratatui::style::Color;
//!
//! let chart = WaterfallChart::new()
//!     .entry(WaterfallEntry::new("Revenue", 100.0))
//!     .entry(WaterfallEntry::new("COGS", -40.0))
//!     .entry(WaterfallEntry::new("Expenses", -30.0))
//!     .entry(WaterfallEntry::total("Profit", 30.0))
//!     .title("P&L Waterfall");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBackend, Z_CHROME, Z_DATA, create_backend};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::ticker::NullLocator;
use crate::transform::data_to_screen;

/// A single entry in a waterfall chart.
#[derive(Clone, Debug)]
pub struct WaterfallEntry {
    /// Label displayed below the bar.
    pub label: String,
    /// Value of this entry (positive or negative).
    pub value: f64,
    /// Whether this entry represents a total (bar drawn from 0).
    pub is_total: bool,
}

impl WaterfallEntry {
    /// Create a new waterfall entry with the given label and value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            is_total: false,
        }
    }

    /// Create a total entry (bar drawn from 0 to value).
    pub fn total(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            is_total: true,
        }
    }
}

/// A waterfall chart widget showing cumulative positive/negative values.
///
/// Each bar starts where the previous bar ended. Total entries are drawn from 0.
/// Connector lines optionally link consecutive bars.
pub struct WaterfallChart {
    entries: Vec<WaterfallEntry>,
    positive_color: Color,
    negative_color: Color,
    total_color: Color,
    connector_line: bool,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for WaterfallChart {
    fn default() -> Self {
        let theme = Theme::get_default();
        Self {
            positive_color: theme.positive_color,
            negative_color: theme.negative_color,
            total_color: theme.neutral_color,
            entries: Vec::new(),
            connector_line: true,
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            theme,
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl WaterfallChart {
    /// Create a new empty waterfall chart.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single entry.
    pub fn entry(mut self, entry: WaterfallEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Add multiple entries at once.
    pub fn entries(mut self, entries: Vec<WaterfallEntry>) -> Self {
        self.entries.extend(entries);
        self
    }

    /// Set the color for positive values.
    pub fn positive_color(mut self, color: Color) -> Self {
        self.positive_color = color;
        self
    }

    /// Set the color for negative values.
    pub fn negative_color(mut self, color: Color) -> Self {
        self.negative_color = color;
        self
    }

    /// Set the color for total entries.
    pub fn total_color(mut self, color: Color) -> Self {
        self.total_color = color;
        self
    }

    /// Enable or disable connector lines between bars.
    pub fn connector_line(mut self, enabled: bool) -> Self {
        self.connector_line = enabled;
        self
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the x-axis configuration.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the y-axis configuration.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
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


impl Widget for &WaterfallChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.entries.is_empty() {
            return;
        }

        // Compute running totals and bar positions (bottom, top) for each entry
        let mut running = 0.0f64;
        let mut bars: Vec<(f64, f64)> = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            if entry.is_total {
                let bottom = 0.0f64.min(entry.value);
                let top = 0.0f64.max(entry.value);
                bars.push((bottom, top));
            } else {
                let bottom = running.min(running + entry.value);
                let top = running.max(running + entry.value);
                bars.push((bottom, top));
                running += entry.value;
            }
        }

        // Determine y-bounds from all bar tops/bottoms
        let mut y_min = 0.0f64;
        let mut y_max = 0.0f64;
        for &(bottom, top) in &bars {
            if bottom < y_min {
                y_min = bottom;
            }
            if top > y_max {
                y_max = top;
            }
        }
        // Add some padding
        let y_range = y_max - y_min;
        let padding = if y_range == 0.0 { 1.0 } else { y_range * 0.1 };
        let y_lo = y_min - padding;
        let y_hi = y_max + padding;

        let n = self.entries.len();
        let x_lo = 0.0;
        let x_hi = n as f64;

        // Use NullLocator for x-axis (category labels drawn manually)
        let x_axis = Axis::new().locator(NullLocator);

        let mut pb = create_backend(area);

        let frame = PlotFrame::new(&x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render_to_pb(
            &mut pb,
            area,
            DataBounds {
                x_lo,
                x_hi,
                y_lo,
                y_hi,
            },
        ) else {
            return;
        };

        let group_width = pa.width / n as u16;
        let bar_gap: u16 = 1;

        // Compute running total positions for connector lines
        let mut running_vals: Vec<f64> = Vec::with_capacity(self.entries.len());
        let mut run = 0.0f64;
        for entry in &self.entries {
            if entry.is_total {
                running_vals.push(entry.value);
            } else {
                run += entry.value;
                running_vals.push(run);
            }
        }

        for (i, entry) in self.entries.iter().enumerate() {
            let (bar_bottom, bar_top) = bars[i];
            let group_x = pa.x + i as u16 * group_width;
            let bar_x = group_x + bar_gap;
            let bar_w = group_width.saturating_sub(bar_gap * 2).max(1);

            let screen_top = data_to_screen(
                bar_top,
                y_lo,
                y_hi,
                (pa.y + pa.height - 1) as f64,
                pa.y as f64,
            )
            .round() as u16;
            let screen_bottom = data_to_screen(
                bar_bottom,
                y_lo,
                y_hi,
                (pa.y + pa.height - 1) as f64,
                pa.y as f64,
            )
            .round() as u16;

            let color = if entry.is_total {
                self.total_color
            } else if entry.value >= 0.0 {
                self.positive_color
            } else {
                self.negative_color
            };

            // Draw bar
            let draw_top = screen_top.min(screen_bottom);
            let draw_bottom = screen_top.max(screen_bottom);
            for x in bar_x..bar_x + bar_w {
                for y in draw_top..=draw_bottom {
                    if pa.contains(x, y) {
                        pb.set_cell(x, y, ' ', color, color, Z_DATA);
                    }
                }
            }

            // Draw connector line to next bar
            if self.connector_line && i + 1 < n {
                let conn_y_val = running_vals[i];
                let conn_screen_y = data_to_screen(
                    conn_y_val,
                    y_lo,
                    y_hi,
                    (pa.y + pa.height - 1) as f64,
                    pa.y as f64,
                )
                .round() as u16;

                let conn_x_start = bar_x + bar_w;
                let next_group_x = pa.x + (i as u16 + 1) * group_width;
                let conn_x_end = next_group_x + bar_gap;

                if conn_screen_y >= pa.y && conn_screen_y < pa.y + pa.height {
                    for x in conn_x_start..conn_x_end {
                        if pa.contains(x, conn_screen_y) {
                            pb.set_char(
                                x,
                                conn_screen_y,
                                self.theme.chars.border.horizontal,
                                self.theme.axis_color,
                                Z_DATA,
                            );
                        }
                    }
                }
            }

            // Draw category label
            let label = &entry.label;
            let label_x = group_x + group_width / 2;
            let label_start = label_x.saturating_sub(label.len() as u16 / 2);
            let label_y = pa.y + pa.height;
            if label_y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        pb.set_char(lx, label_y, ch, self.theme.axis_color, Z_CHROME);
                    }
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);
    }
}
