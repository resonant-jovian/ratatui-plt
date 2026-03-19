//! Radial (polar) plot widget.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::series::Series;

/// A radial (polar coordinate) plot widget.
///
/// Displays data in polar coordinates with circular grid lines and angular tick marks.
/// Useful for radial density profiles, angular distributions, etc.
///
/// # Example
///
/// ```
/// use ratatui_sim::prelude::*;
///
/// // Cardioid: r = 1 + cos(θ)
/// let data: Vec<(f64, f64)> = (0..360)
///     .map(|i| {
///         let theta = i as f64 * std::f64::consts::PI / 180.0;
///         (theta, 1.0 + theta.cos())
///     })
///     .collect();
/// let plot = RadialPlot::new()
///     .series(Series::new("cardioid").data(data).color(Color::Cyan))
///     .title("Polar Plot");
/// ```
pub struct RadialPlot {
    /// Series data: (theta_radians, r) pairs.
    series: Vec<Series>,
    title: Option<String>,
    n_rings: usize,
    n_spokes: usize,
    r_max: Option<f64>,
}

impl Default for RadialPlot {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            title: None,
            n_rings: 4,
            n_spokes: 8,
            r_max: None,
        }
    }
}

impl RadialPlot {
    pub fn new() -> Self { Self::default() }

    /// Add a series. Data should be (theta, r) pairs where theta is in radians.
    pub fn series(mut self, s: Series) -> Self { self.series.push(s); self }
    pub fn title(mut self, t: impl Into<String>) -> Self { self.title = Some(t.into()); self }
    pub fn n_rings(mut self, n: usize) -> Self { self.n_rings = n; self }
    pub fn n_spokes(mut self, n: usize) -> Self { self.n_spokes = n; self }
    pub fn r_max(mut self, r: f64) -> Self { self.r_max = Some(r); self }
}

impl Widget for &RadialPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 8 || area.height < 6 {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let py = area.y + title_height;
        let ph = area.height.saturating_sub(title_height);
        let pw = area.width;

        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(Color::White);
                }
            }
        }

        // Center of the plot
        let cx = area.x + pw / 2;
        let cy = py + ph / 2;

        // Radius in screen characters (account for cell aspect ratio ~2:1)
        let r_screen_x = (pw / 2).saturating_sub(2) as f64;
        let r_screen_y = (ph / 2).saturating_sub(1) as f64;

        // Maximum data radius
        let r_max = self.r_max.unwrap_or_else(|| {
            self.series
                .iter()
                .flat_map(|s| s.data.iter().map(|&(_, r)| r))
                .fold(0.0f64, f64::max)
        });
        let r_max = if r_max == 0.0 { 1.0 } else { r_max };

        // Draw concentric rings
        for ring in 1..=self.n_rings {
            let r_frac = ring as f64 / self.n_rings as f64;
            let rx = (r_frac * r_screen_x).round();
            let ry = (r_frac * r_screen_y).round();

            // Draw ring using approximation
            let n_points = (2.0 * std::f64::consts::PI * rx.max(ry)).round().max(20.0) as usize;
            for k in 0..n_points {
                let theta = 2.0 * std::f64::consts::PI * k as f64 / n_points as f64;
                let dx = (rx * theta.cos()).round() as i16;
                let dy = (ry * theta.sin()).round() as i16;
                let sx = (cx as i16 + dx) as u16;
                let sy = (cy as i16 + dy) as u16;
                if sx >= area.x && sx < area.x + area.width && sy >= py && sy < py + ph {
                    buf[(sx, sy)].set_char('·').set_fg(Color::DarkGray);
                }
            }

            // Ring label
            let label = format!("{:.1}", r_max * r_frac);
            let lx = cx;
            let ly = (cy as i16 - ry as i16) as u16;
            if ly >= py && ly < py + ph {
                for (j, ch) in label.chars().enumerate() {
                    let x = lx + j as u16 + 1;
                    if x < area.x + area.width {
                        buf[(x, ly)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        }

        // Draw spokes
        for spoke in 0..self.n_spokes {
            let theta = 2.0 * std::f64::consts::PI * spoke as f64 / self.n_spokes as f64;
            let dx = r_screen_x * theta.cos();
            let dy = r_screen_y * theta.sin();
            let steps = (dx.abs().max(dy.abs())).round() as usize;
            for s in 0..=steps {
                let frac = s as f64 / steps.max(1) as f64;
                let sx = (cx as f64 + dx * frac).round() as u16;
                let sy = (cy as f64 + dy * frac).round() as u16;
                if sx >= area.x && sx < area.x + area.width && sy >= py && sy < py + ph {
                    buf[(sx, sy)].set_char('·').set_fg(Color::DarkGray);
                }
            }

            // Angle label
            let deg = (theta.to_degrees()).round() as i32;
            let label = format!("{}°", deg);
            let lx = (cx as f64 + (r_screen_x + 2.0) * theta.cos()).round() as u16;
            let ly = (cy as f64 + (r_screen_y + 1.0) * theta.sin()).round() as u16;
            if lx >= area.x && lx + label.len() as u16 <= area.x + area.width && ly >= py && ly < py + ph {
                for (j, ch) in label.chars().enumerate() {
                    buf[(lx + j as u16, ly)].set_char(ch).set_fg(Color::DarkGray);
                }
            }
        }

        // Draw series data
        for s in &self.series {
            let mut prev: Option<(u16, u16)> = None;
            for &(theta, r) in &s.data {
                let r_frac = (r / r_max).clamp(0.0, 1.0);
                let sx = (cx as f64 + r_frac * r_screen_x * theta.cos()).round() as u16;
                let sy = (cy as f64 + r_frac * r_screen_y * theta.sin()).round() as u16;

                if sx >= area.x && sx < area.x + area.width && sy >= py && sy < py + ph {
                    let ch = s.marker.map_or('●', |m| m.char());
                    buf[(sx, sy)].set_char(ch).set_fg(s.color);
                }

                // Connect to previous point
                if let Some((px, py_prev)) = prev {
                    if sx != px || sy != py_prev {
                        // Simple line between consecutive points
                        let dx = sx as i32 - px as i32;
                        let dy = sy as i32 - py_prev as i32;
                        let steps = dx.abs().max(dy.abs());
                        for step in 1..steps {
                            let frac = step as f64 / steps as f64;
                            let ix = (px as f64 + dx as f64 * frac).round() as u16;
                            let iy = (py_prev as f64 + dy as f64 * frac).round() as u16;
                            if ix >= area.x && ix < area.x + area.width && iy >= py && iy < py + ph {
                                buf[(ix, iy)].set_char('·').set_fg(s.color);
                            }
                        }
                    }
                }
                prev = Some((sx, sy));
            }
        }
    }
}
