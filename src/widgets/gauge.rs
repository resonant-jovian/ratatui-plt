//! Gauge chart widget for KPI/dashboard visualization.
//!
//! Renders a semicircular gauge with a needle indicator, useful for
//! displaying single-value metrics against a range.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::gauge::{GaugeChart, GaugeSector};
//! use ratatui::style::Color;
//!
//! let gauge = GaugeChart::new(72.0)
//!     .min(0.0)
//!     .max(100.0)
//!     .sector(GaugeSector::new(0.0, 33.0, Color::Green))
//!     .sector(GaugeSector::new(33.0, 66.0, Color::Yellow))
//!     .sector(GaugeSector::new(66.0, 100.0, Color::Red))
//!     .title("CPU Usage");
//! ```

use std::f64::consts::PI;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA};
use crate::theme::Theme;

/// A sector (colored segment) of the gauge arc.
#[derive(Clone, Debug)]
pub struct GaugeSector {
    /// Start value of this sector.
    pub start: f64,
    /// End value of this sector.
    pub end: f64,
    /// Color for this sector.
    pub color: Color,
}

impl GaugeSector {
    /// Create a new gauge sector.
    pub fn new(start: f64, end: f64, color: Color) -> Self {
        Self { start, end, color }
    }
}

/// A semicircular gauge chart widget.
///
/// Displays a value on a semicircular arc with optional colored sectors
/// and a needle indicator.
pub struct GaugeChart {
    value: f64,
    min: f64,
    max: f64,
    title: Option<String>,
    label: Option<String>,
    sectors: Vec<GaugeSector>,
    theme: Theme,
}

impl GaugeChart {
    /// Create a new gauge chart with the given value.
    pub fn new(value: f64) -> Self {
        Self {
            value,
            min: 0.0,
            max: 100.0,
            title: None,
            label: None,
            sectors: Vec::new(),
            theme: Theme::get_default(),
        }
    }

    /// Set the minimum value.
    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    /// Set the maximum value.
    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    /// Set the chart title (displayed above the gauge).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the center label (if None, the value is displayed).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add a colored sector to the gauge.
    pub fn sector(mut self, sector: GaugeSector) -> Self {
        self.sectors.push(sector);
        self
    }

    /// Set all sectors at once.
    pub fn sectors(mut self, sectors: Vec<GaugeSector>) -> Self {
        self.sectors = sectors;
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Get the color for a given value based on sectors.
    fn color_for_value(&self, val: f64) -> Color {
        for sector in &self.sectors {
            if val >= sector.start && val < sector.end {
                return sector.color;
            }
        }
        // Check last sector endpoint
        if let Some(last) = self.sectors.last()
            && (val - last.end).abs() < f64::EPSILON
        {
            return last.color;
        }
        // Default green-yellow-red gradient
        let range = self.max - self.min;
        if range <= 0.0 {
            return Color::Green;
        }
        let fraction = ((val - self.min) / range).clamp(0.0, 1.0);
        if fraction < 0.33 {
            Color::Green
        } else if fraction < 0.66 {
            Color::Yellow
        } else {
            Color::Red
        }
    }
}

impl Widget for &GaugeChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 8 || area.height < 4 {
            return;
        }

        // Reserve space for title
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        // Reserve space for value label below arc
        let label_height: u16 = 1;

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
        let draw_h = area.height.saturating_sub(title_height + label_height);
        if draw_h < 3 {
            return;
        }

        // Terminal cell aspect ratio: cells are ~2x tall as wide
        let cell_aspect = 0.5;

        // Center of the arc (bottom-center of drawing area)
        let cx = area.x as f64 + area.width as f64 / 2.0;
        let cy = (draw_y + draw_h) as f64;

        // Radius: fit within the drawing area
        // The semicircle spans from left to right (180 degrees at top)
        let r_x = (area.width as f64 / 2.0 - 2.0).max(2.0);
        let r_y = (draw_h as f64 - 1.0).max(2.0);

        let range = self.max - self.min;
        // Draw the arc
        // Semicircle from angle PI (left) to 0 (right)
        // Use a pixel-based approach: for each cell in the drawing area, check distance from arc
        let arc_thickness = 2.0; // thickness in screen cells

        for screen_y in draw_y..draw_y + draw_h {
            for screen_x in area.x..area.x + area.width {
                let dx = screen_x as f64 - cx;
                let dy = screen_y as f64 - cy;

                // Normalize by radii
                let nx = dx / r_x;
                let ny = dy / r_y;
                let r = (nx * nx + ny * ny).sqrt();

                // Only draw the upper semicircle (y <= cy, so dy <= 0)
                if dy > 0.5 {
                    continue;
                }

                // Check if on the arc ring
                let inner_r = 1.0 - arc_thickness / r_y.max(r_x);
                if r > 1.05 || r < inner_r.max(0.3) {
                    continue;
                }

                // Compute angle (atan2 with inverted y for screen coords)
                let angle = (-dy / r_y).atan2(dx / r_x);
                if !(-0.05..=PI + 0.05).contains(&angle) {
                    continue;
                }

                // Map angle to value: PI -> min, 0 -> max
                let val = if range > 0.0 {
                    self.min + (PI - angle) / PI * range
                } else {
                    self.min
                };

                let color = self.color_for_value(val);

                pb.set_cell(screen_x, screen_y, '\u{2588}', color, color, Z_DATA);
            }
        }

        // Draw needle
        if range > 0.0 {
            let clamped = self.value.clamp(self.min, self.max);
            let fraction = (clamped - self.min) / range;
            // Map fraction to angle: 0.0 -> PI (left), 1.0 -> 0 (right)
            let needle_angle = PI * (1.0 - fraction);

            // Draw needle line from center toward the angle
            let needle_len = r_y.min(r_x) * 0.85;
            let steps = (needle_len * 2.0) as usize;
            for s in 0..=steps {
                let t = s as f64 / steps as f64;
                let nx = cx + t * needle_len * (needle_angle.cos() / cell_aspect);
                let ny = cy - t * needle_len * needle_angle.sin();

                let xi = nx.round() as u16;
                let yi = ny.round() as u16;

                if xi >= area.x
                    && xi < area.x + area.width
                    && yi >= draw_y
                    && yi < draw_y + draw_h
                {
                    pb.set_cell(xi, yi, '\u{2588}', self.theme.foreground, self.theme.foreground, Z_CHROME);
                }
            }

            // Draw needle hub at center
            let hub_x = cx.round() as u16;
            let hub_y = cy.round() as u16;
            if hub_x >= area.x
                && hub_x < area.x + area.width
                && hub_y >= area.y
                && hub_y < area.y + area.height
            {
                pb.set_char(hub_x, hub_y, '\u{25CF}', self.theme.foreground, Z_CHROME);
            }
        }

        // Draw value label centered below the arc
        let label_y = draw_y + draw_h;
        if label_y < area.y + area.height {
            let text = match &self.label {
                Some(l) => l.clone(),
                None => {
                    if self.value == self.value.floor() {
                        format!("{}", self.value as i64)
                    } else {
                        format!("{:.1}", self.value)
                    }
                }
            };
            let start = area.x + (area.width.saturating_sub(text.len() as u16)) / 2;
            for (i, ch) in text.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    pb.set_char(x, label_y, ch, self.theme.foreground, Z_CHROME);
                }
            }
        }

        pb.composite(buf);
    }
}
