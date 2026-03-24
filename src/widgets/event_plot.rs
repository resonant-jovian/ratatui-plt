//! Event timing visualization widget, inspired by matplotlib's `eventplot`.
//!
//! Displays groups of events as tick marks along parallel lines. Each group
//! occupies its own lane. Supports both horizontal and vertical orientations.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::event_plot::{EventPlot, EventGroup, Orientation};
//! use ratatui::style::Color;
//!
//! let plot = EventPlot::new()
//!     .group(EventGroup::new("Neuron A", vec![0.5, 1.2, 2.8, 3.1]).color(Color::Cyan))
//!     .group(EventGroup::new("Neuron B", vec![0.8, 1.5, 2.0, 3.5]).color(Color::Yellow))
//!     .title("Spike Raster")
//!     .orientation(Orientation::Horizontal);
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA, Z_GRID};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::ticker::NullLocator;
use crate::transform::data_to_screen;

/// Orientation for the event plot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    /// Events are ticks on horizontal lines (groups stacked vertically).
    #[default]
    Horizontal,
    /// Events are ticks on vertical lines (groups arranged horizontally).
    Vertical,
}

/// A group of events at a single "channel" or "lane".
#[derive(Clone, Debug)]
pub struct EventGroup {
    /// Label for this event group.
    pub label: String,
    /// Event positions along the data axis. NaN values are filtered out.
    pub positions: Vec<f64>,
    /// Colour for the event ticks.
    pub color: Option<Color>,
}

impl EventGroup {
    /// Create a new event group with label and positions.
    pub fn new(label: impl Into<String>, positions: Vec<f64>) -> Self {
        Self {
            label: label.into(),
            positions,
            color: None,
        }
    }

    /// Set the tick colour.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Return positions with NaN values filtered out.
    fn valid_positions(&self) -> Vec<f64> {
        self.positions
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .collect()
    }
}

/// An event timing visualisation widget.
///
/// Renders multiple groups of events as tick marks on parallel lines.
/// Each group gets its own lane; events appear as short perpendicular ticks
/// at their data positions.
pub struct EventPlot {
    /// Event groups to display.
    events: Vec<EventGroup>,
    /// Plot orientation.
    orientation: Orientation,
    /// Chart title.
    title: Option<String>,
    /// X-axis configuration (data axis in horizontal mode).
    x_axis: Axis,
    /// Y-axis configuration (data axis in vertical mode).
    y_axis: Axis,
    /// Visual theme.
    theme: Theme,
    /// Spine visibility control.
    spines: Spines,
    /// Reference lines drawn across the plot area.
    reference_lines: Vec<ReferenceLine>,
    /// Annotations drawn within the plot area.
    annotations: Vec<Annotation>,
}

impl Default for EventPlot {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            orientation: Orientation::Horizontal,
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl EventPlot {
    /// Create a new empty event plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an event group.
    pub fn group(mut self, g: EventGroup) -> Self {
        self.events.push(g);
        self
    }

    /// Add multiple event groups.
    pub fn groups(mut self, groups: Vec<EventGroup>) -> Self {
        self.events.extend(groups);
        self
    }

    /// Set the orientation.
    pub fn orientation(mut self, o: Orientation) -> Self {
        self.orientation = o;
        self
    }

    /// Set the chart title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Set the X-axis configuration.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y-axis configuration.
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

impl Widget for &EventPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.events.is_empty() {
            return;
        }

        match self.orientation {
            Orientation::Horizontal => {
                self.render_horizontal(area, buf);
            }
            Orientation::Vertical => {
                self.render_vertical(area, buf);
            }
        }
    }
}

impl EventPlot {
    /// Render in horizontal orientation: groups as horizontal lines, events as vertical ticks.
    fn render_horizontal(&self, area: Rect, buf: &mut Buffer) {
        let mut pb = PlotBuffer::new(area);
        // Compute data bounds from all event positions
        let mut d_min = f64::INFINITY;
        let mut d_max = f64::NEG_INFINITY;
        for group in &self.events {
            for &pos in &group.positions {
                if pos.is_finite() {
                    d_min = d_min.min(pos);
                    d_max = d_max.max(pos);
                }
            }
        }
        if !d_min.is_finite() || !d_max.is_finite() {
            return;
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(d_min, d_max);

        let n_groups = self.events.len();
        let y_lo = 0.0;
        let y_hi = n_groups as f64;

        // Compute label width for y_label_width (group labels go in the y-axis margin)
        let label_width: u16 = self
            .events
            .iter()
            .map(|g| g.label.len() as u16)
            .max()
            .unwrap_or(0)
            .min(12)
            + 1;

        // Use NullLocator for y-axis (categorical lanes, labels drawn manually)
        let y_axis = Axis::new().locator(NullLocator);

        let frame = PlotFrame::new(&self.x_axis, &y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .y_label_width(label_width)
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

        let lane_height = pa.height as f64 / n_groups as f64;

        // Draw each group
        for (gi, group) in self.events.iter().enumerate() {
            let group_color = group.color.unwrap_or_else(|| self.theme.color_cycle.at(gi));
            let lane_center_y = pa.y as f64 + (gi as f64 + 0.5) * lane_height;
            let lane_y = lane_center_y.round() as u16;

            // Draw the group label in the y-axis margin
            let label = if group.label.len() > label_width as usize - 1 {
                &group.label[..label_width as usize - 1]
            } else {
                &group.label
            };
            let label_start = pa.x.saturating_sub(label_width);
            if lane_y >= pa.y && lane_y < pa.y + pa.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < pa.x.saturating_sub(1) {
                        pb.set_char(lx, lane_y, ch, group_color, Z_CHROME);
                    }
                }

                // Draw the horizontal baseline for this group
                for x in pa.x..pa.x + pa.width {
                    if x < area.x + area.width {
                        pb.set_char(x, lane_y, self.theme.chars.border.horizontal, self.theme.grid_color, Z_GRID);
                    }
                }
            }

            // Draw event ticks
            let tick_half = (lane_height / 3.0).max(1.0).round() as u16;
            let valid = group.valid_positions();
            for &pos in &valid {
                let sx = data_to_screen(pos, x_lo, x_hi, pa.x as f64, (pa.x + pa.width - 1) as f64);
                let xi = sx.round() as u16;
                if xi < pa.x || xi >= pa.x + pa.width {
                    continue;
                }

                let y_top = lane_y.saturating_sub(tick_half);
                let y_bot = (lane_y + tick_half).min(pa.y + pa.height - 1);
                for ty in y_top..=y_bot {
                    if ty >= pa.y && ty < pa.y + pa.height && xi < area.x + area.width {
                        pb.set_char(xi, ty, self.theme.chars.border.vertical, group_color, Z_DATA);
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

    /// Render in vertical orientation: groups as vertical lines, events as horizontal ticks.
    fn render_vertical(&self, area: Rect, buf: &mut Buffer) {
        let mut pb = PlotBuffer::new(area);

        // Compute data bounds from all event positions
        let mut d_min = f64::INFINITY;
        let mut d_max = f64::NEG_INFINITY;
        for group in &self.events {
            for &pos in &group.positions {
                if pos.is_finite() {
                    d_min = d_min.min(pos);
                    d_max = d_max.max(pos);
                }
            }
        }
        if !d_min.is_finite() || !d_max.is_finite() {
            return;
        }

        let (y_lo, y_hi) = self.y_axis.resolve_bounds(d_min, d_max);

        let n_groups = self.events.len();
        let x_lo = 0.0;
        let x_hi = n_groups as f64;

        // Use NullLocator for x-axis (categorical lanes, labels drawn manually)
        let x_axis = Axis::new().locator(NullLocator);

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

        let lane_width = pa.width as f64 / n_groups as f64;

        // Draw each group
        for (gi, group) in self.events.iter().enumerate() {
            let group_color = group.color.unwrap_or_else(|| self.theme.color_cycle.at(gi));
            let lane_center_x = pa.x as f64 + (gi as f64 + 0.5) * lane_width;
            let lane_x = lane_center_x.round() as u16;

            // Draw the group label below the plot area
            let label = if group.label.len() > lane_width as usize {
                &group.label[..lane_width as usize]
            } else {
                &group.label
            };
            let label_y = pa.y + pa.height;
            if label_y < area.y + area.height {
                let label_start = lane_x.saturating_sub(label.len() as u16 / 2);
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= pa.x && lx < pa.x + pa.width {
                        pb.set_char(lx, label_y, ch, group_color, Z_CHROME);
                    }
                }
            }

            // Draw the vertical baseline for this group
            if lane_x >= pa.x && lane_x < pa.x + pa.width {
                for y in pa.y..pa.y + pa.height {
                    pb.set_char(lane_x, y, self.theme.chars.grid.major_v, self.theme.grid_color, Z_GRID);
                }
            }

            // Draw event ticks
            let tick_half = (lane_width / 3.0).max(1.0).round() as u16;
            let valid = group.valid_positions();
            for &pos in &valid {
                let sy =
                    data_to_screen(pos, y_lo, y_hi, (pa.y + pa.height - 1) as f64, pa.y as f64);
                let yi = sy.round() as u16;
                if yi < pa.y || yi >= pa.y + pa.height {
                    continue;
                }

                let x_left = lane_x.saturating_sub(tick_half);
                let x_right = (lane_x + tick_half).min(pa.x + pa.width - 1);
                for tx in x_left..=x_right {
                    if tx >= pa.x && tx < pa.x + pa.width && yi < area.y + area.height {
                        pb.set_char(tx, yi, self.theme.chars.border.horizontal, group_color, Z_DATA);
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
