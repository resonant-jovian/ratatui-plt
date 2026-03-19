//! Vector field (quiver) plot widget.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::{AspectRatio, Axis};
use crate::colormap::{Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::series::VectorFieldData;
use crate::transform::{apply_aspect_ratio, data_to_screen};

/// A 2D vector field (quiver) plot widget.
///
/// Renders arrows at grid points showing vector direction and magnitude.
/// Useful for velocity fields, gravitational fields, etc.
///
/// # Example
///
/// ```
/// use ratatui_sim::prelude::*;
///
/// let field = VectorFieldData::from_fn(
///     (-2.0, 2.0), (-2.0, 2.0), 10, 10,
///     |x, y| (-y, x), // Circular flow
/// );
/// let plot = VectorField::new(field).title("Circular Flow");
/// ```
pub struct VectorField {
    data: VectorFieldData,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    aspect_ratio: AspectRatio,
    color: Color,
    color_by_magnitude: bool,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    arrow_scale: f64,
}

impl VectorField {
    pub fn new(data: VectorFieldData) -> Self {
        let max_mag = data.max_magnitude();
        Self {
            data,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            aspect_ratio: AspectRatio::Auto,
            color: Color::Cyan,
            color_by_magnitude: false,
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(0.0, if max_mag == 0.0 { 1.0 } else { max_mag })),
            arrow_scale: 1.0,
        }
    }

    pub fn x_axis(mut self, axis: Axis) -> Self { self.x_axis = axis; self }
    pub fn y_axis(mut self, axis: Axis) -> Self { self.y_axis = axis; self }
    pub fn title(mut self, t: impl Into<String>) -> Self { self.title = Some(t.into()); self }
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self { self.aspect_ratio = ar; self }
    pub fn color(mut self, c: Color) -> Self { self.color = c; self }

    /// Color arrows by their magnitude using the colormap.
    pub fn color_by_magnitude(mut self, enable: bool) -> Self {
        self.color_by_magnitude = enable;
        self
    }

    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    /// Scale factor for arrow length.
    pub fn arrow_scale(mut self, scale: f64) -> Self {
        self.arrow_scale = scale;
        self
    }
}

/// Choose an arrow character based on the direction angle.
fn arrow_char(dx: f64, dy: f64) -> char {
    if dx == 0.0 && dy == 0.0 {
        return '·';
    }
    let angle = dy.atan2(dx);
    let octant = ((angle + std::f64::consts::PI) / (std::f64::consts::PI / 4.0)).round() as i32 % 8;
    match octant {
        0 => '←',
        1 => '↙',
        2 => '↓',
        3 => '↘',
        4 => '→',
        5 => '↗',
        6 => '↑',
        7 => '↖',
        _ => '→',
    }
}

impl Widget for &VectorField {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.data.vectors.is_empty() {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let y_label_width: u16 = 8;
        let tick_height: u16 = 1;

        let px = area.x + y_label_width;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(y_label_width + 1);
        let ph = area.height.saturating_sub(title_height + tick_height);

        if pw < 2 || ph < 2 {
            return;
        }

        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(Color::White);
                }
            }
        }

        // Compute bounds
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for &(x, y, _, _) in &self.data.vectors {
            x_min = x_min.min(x);
            x_max = x_max.max(x);
            y_min = y_min.min(y);
            y_max = y_max.max(y);
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let (ax_off, ay_off, aw, ah) =
            apply_aspect_ratio(&self.aspect_ratio, x_hi - x_lo, y_hi - y_lo, pw, ph);
        let px = px + ax_off;
        let py = py + ay_off;

        // Draw axes
        for x in px..px + aw {
            if x < area.x + area.width {
                buf[(x, py + ah)].set_char('─').set_fg(Color::DarkGray);
            }
        }
        for y in py..py + ah {
            buf[(px.saturating_sub(1), y)].set_char('│').set_fg(Color::DarkGray);
        }

        // Draw arrows
        let max_mag = self.data.max_magnitude();
        for &(x, y, dx, dy) in &self.data.vectors {
            let sx = data_to_screen(x, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
            let sy = data_to_screen(y, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
            let xi = sx.round() as u16;
            let yi = sy.round() as u16;

            if xi >= px && xi < px + aw && yi >= py && yi < py + ah {
                let mag = (dx * dx + dy * dy).sqrt();
                let color = if self.color_by_magnitude && max_mag > 0.0 {
                    let t = self.norm.normalize(mag);
                    self.colormap.color_at(t)
                } else {
                    self.color
                };

                let ch = arrow_char(dx, -dy); // Negate dy because screen y is inverted
                buf[(xi, yi)].set_char(ch).set_fg(color);
            }
        }

        // Tick labels
        let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let start = xi.saturating_sub(label.len() as u16 / 2);
            let y = py + ah;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        }
    }
}
