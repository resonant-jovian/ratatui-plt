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

use crate::axis::Axis;
use crate::theme::Theme;
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
    pub color: Color,
}

impl EventGroup {
    /// Create a new event group with label and positions.
    pub fn new(label: impl Into<String>, positions: Vec<f64>) -> Self {
        Self {
            label: label.into(),
            positions,
            color: Color::White,
        }
    }

    /// Set the tick colour.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
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
}

impl Widget for &EventPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.events.is_empty() {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };

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

        match self.orientation {
            Orientation::Horizontal => {
                self.render_horizontal(area, buf, title_height);
            }
            Orientation::Vertical => {
                self.render_vertical(area, buf, title_height);
            }
        }
    }
}

impl EventPlot {
    /// Render in horizontal orientation: groups as horizontal lines, events as vertical ticks.
    fn render_horizontal(&self, area: Rect, buf: &mut Buffer, title_height: u16) {
        let label_width: u16 = self
            .events
            .iter()
            .map(|g| g.label.len() as u16)
            .max()
            .unwrap_or(0)
            .min(12)
            + 1;
        let tick_height: u16 = 1;

        let px = area.x + label_width;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(label_width + 1);
        let ph = area.height.saturating_sub(title_height + tick_height);

        if pw < 2 || ph < 2 {
            return;
        }

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
        // Each group gets an equal vertical band
        let lane_height = ph as f64 / n_groups as f64;

        // Draw X-axis line
        for x in px..px + pw {
            if x < area.x + area.width {
                buf[(x, py + ph)]
                    .set_char('─')
                    .set_fg(self.theme.axis_color);
            }
        }

        // Draw Y-axis line
        for y in py..py + ph {
            let x = px.saturating_sub(1);
            if x >= area.x {
                buf[(x, y)].set_char('│').set_fg(self.theme.axis_color);
            }
        }

        // Draw grid (x-axis only; y-axis is categorical lanes)
        let x_grid = self.x_axis.grid || self.theme.grid_visible;
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

        // Draw each group
        for (gi, group) in self.events.iter().enumerate() {
            let lane_center_y = py as f64 + (gi as f64 + 0.5) * lane_height;
            let lane_y = lane_center_y.round() as u16;

            // Draw the group label
            let label = if group.label.len() > label_width as usize - 1 {
                &group.label[..label_width as usize - 1]
            } else {
                &group.label
            };
            let label_start = px.saturating_sub(label_width);
            if lane_y >= py && lane_y < py + ph {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < px.saturating_sub(1) {
                        buf[(lx, lane_y)].set_char(ch).set_fg(group.color);
                    }
                }

                // Draw the horizontal baseline for this group
                for x in px..px + pw {
                    if x < area.x + area.width {
                        buf[(x, lane_y)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }

            // Draw event ticks
            let tick_half = (lane_height / 3.0).max(1.0).round() as u16;
            let valid = group.valid_positions();
            for &pos in &valid {
                let sx = data_to_screen(pos, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
                let xi = sx.round() as u16;
                if xi < px || xi >= px + pw {
                    continue;
                }

                // Draw a short vertical tick centred on the lane
                let y_top = lane_y.saturating_sub(tick_half);
                let y_bot = (lane_y + tick_half).min(py + ph - 1);
                for ty in y_top..=y_bot {
                    if ty >= py && ty < py + ph && xi < area.x + area.width {
                        buf[(xi, ty)].set_char('│').set_fg(group.color);
                    }
                }
            }
        }

        // X-axis tick labels
        let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let start = xi.saturating_sub(label.len() as u16 / 2);
            let y = py + ph;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }
    }

    /// Render in vertical orientation: groups as vertical lines, events as horizontal ticks.
    fn render_vertical(&self, area: Rect, buf: &mut Buffer, title_height: u16) {
        let label_height: u16 = 1;
        let y_label_width: u16 = 8;

        let px = area.x + y_label_width;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(y_label_width + 1);
        let ph = area.height.saturating_sub(title_height + label_height);

        if pw < 2 || ph < 2 {
            return;
        }

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
        let lane_width = pw as f64 / n_groups as f64;

        // Draw Y-axis line
        for y in py..py + ph {
            let x = px.saturating_sub(1);
            if x >= area.x {
                buf[(x, y)].set_char('│').set_fg(self.theme.axis_color);
            }
        }

        // Draw X-axis line at bottom
        for x in px..px + pw {
            if x < area.x + area.width {
                buf[(x, py + ph)]
                    .set_char('─')
                    .set_fg(self.theme.axis_color);
            }
        }

        // Draw grid (y-axis only; x-axis is categorical lanes)
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

        // Draw each group
        for (gi, group) in self.events.iter().enumerate() {
            let lane_center_x = px as f64 + (gi as f64 + 0.5) * lane_width;
            let lane_x = lane_center_x.round() as u16;

            // Draw the group label below
            let label = if group.label.len() > lane_width as usize {
                &group.label[..lane_width as usize]
            } else {
                &group.label
            };
            let label_y = py + ph;
            if label_y < area.y + area.height {
                let label_start = lane_x.saturating_sub(label.len() as u16 / 2);
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= px && lx < px + pw {
                        buf[(lx, label_y)].set_char(ch).set_fg(group.color);
                    }
                }
            }

            // Draw the vertical baseline for this group
            if lane_x >= px && lane_x < px + pw {
                for y in py..py + ph {
                    buf[(lane_x, y)].set_char('·').set_fg(self.theme.grid_color);
                }
            }

            // Draw event ticks
            let tick_half = (lane_width / 3.0).max(1.0).round() as u16;
            let valid = group.valid_positions();
            for &pos in &valid {
                let sy = data_to_screen(pos, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi < py || yi >= py + ph {
                    continue;
                }

                // Draw a short horizontal tick centred on the lane
                let x_left = lane_x.saturating_sub(tick_half);
                let x_right = (lane_x + tick_half).min(px + pw - 1);
                for tx in x_left..=x_right {
                    if tx >= px && tx < px + pw && yi < area.y + area.height {
                        buf[(tx, yi)].set_char('─').set_fg(group.color);
                    }
                }
            }
        }

        // Y-axis tick labels
        let y_ticks = self.y_axis.tick_positions(y_lo, y_hi);
        for &tv in &y_ticks {
            let sy = data_to_screen(tv, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
            let label = self.y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ph {
                let label_start = if label.len() < y_label_width as usize {
                    px.saturating_sub(y_label_width) + (y_label_width - label.len() as u16)
                } else {
                    px.saturating_sub(y_label_width)
                };
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < px.saturating_sub(1) {
                        buf[(lx, yi)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }
    }
}
