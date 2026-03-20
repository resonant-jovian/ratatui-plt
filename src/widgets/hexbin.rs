//! Hexagonal binning plot for large datasets.

use std::collections::HashMap;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::colormap::{Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::theme::Theme;
use crate::transform::data_to_screen;

/// Aggregation function for hexbin.
#[derive(Clone, Debug)]
pub enum HexAggregation {
    /// Count points in each bin.
    Count,
    /// Mean of weights.
    Mean,
    /// Sum of weights.
    Sum,
}

/// A hexagonal binning plot widget.
///
/// Efficient for visualizing 10⁴+ data points by aggregating into hex bins.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::hexbin::HexbinPlot;
///
/// let data: Vec<(f64, f64)> = (0..1000)
///     .map(|i| (i as f64 * 0.01, (i as f64 * 0.1).sin()))
///     .collect();
/// let plot = HexbinPlot::new(data).gridsize(15).title("Density");
/// ```
pub struct HexbinPlot {
    data: Vec<(f64, f64)>,
    weights: Option<Vec<f64>>,
    gridsize: usize,
    aggregation: HexAggregation,
    colormap: Box<dyn Colormap>,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    theme: Theme,
}

impl HexbinPlot {
    pub fn new(data: Vec<(f64, f64)>) -> Self {
        Self {
            data,
            weights: None,
            gridsize: 15,
            aggregation: HexAggregation::Count,
            colormap: Box::new(Viridis),
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            theme: Theme::get_default(),
        }
    }

    pub fn gridsize(mut self, n: usize) -> Self {
        self.gridsize = n.max(3);
        self
    }

    pub fn weights(mut self, w: Vec<f64>) -> Self {
        self.weights = Some(w);
        self
    }

    pub fn aggregation(mut self, a: HexAggregation) -> Self {
        self.aggregation = a;
        self
    }

    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    pub fn theme(mut self, t: Theme) -> Self {
        self.theme = t;
        self
    }
}

fn axial_round(q: f64, r: f64) -> (i32, i32) {
    let s = -q - r;
    let (rq, rr, rs) = (q.round(), r.round(), s.round());
    let (dq, dr, ds) = ((rq - q).abs(), (rr - r).abs(), (rs - s).abs());
    if dq > dr && dq > ds {
        ((-rr - rs) as i32, rr as i32)
    } else if dr > ds {
        (rq as i32, (-rq - rs) as i32)
    } else {
        (rq as i32, rr as i32)
    }
}

fn pixel_to_hex(dx: f64, dy: f64, s: f64) -> (i32, i32) {
    let sqrt3 = 3.0_f64.sqrt();
    let q_frac = (sqrt3 / 3.0 * dx - 1.0 / 3.0 * dy) / s;
    let r_frac = (2.0 / 3.0 * dy) / s;
    axial_round(q_frac, r_frac)
}

impl Widget for &HexbinPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.data.is_empty() {
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
                    buf[(x, area.y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        // Compute bounds
        let x_min = self.data.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x_max = self
            .data
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let y_min = self.data.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let y_max = self
            .data
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max);

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Draw grid
        let x_grid = self.x_axis.grid || self.theme.grid_visible;
        let y_grid = self.y_axis.grid || self.theme.grid_visible;
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

        // Hexagonal binning using axial coordinates
        let sqrt3 = 3.0_f64.sqrt();
        let hex_w = (x_hi - x_lo) / self.gridsize as f64;
        let s = hex_w / sqrt3;

        let mut counts: HashMap<(i32, i32), f64> = HashMap::new();
        let mut weight_sums: HashMap<(i32, i32), f64> = HashMap::new();

        for (idx, &(x, y)) in self.data.iter().enumerate() {
            let key = pixel_to_hex(x - x_lo, y - y_lo, s);
            *counts.entry(key).or_insert(0.0) += 1.0;
            if let Some(ref w) = self.weights {
                *weight_sums.entry(key).or_insert(0.0) += w.get(idx).copied().unwrap_or(1.0);
            }
        }

        // Compute display values
        let values: HashMap<(i32, i32), f64> = match self.aggregation {
            HexAggregation::Count => counts.clone(),
            HexAggregation::Sum => weight_sums.clone(),
            HexAggregation::Mean => counts
                .iter()
                .map(|(k, &c)| {
                    let w = weight_sums.get(k).copied().unwrap_or(0.0);
                    (*k, if c > 0.0 { w / c } else { 0.0 })
                })
                .collect(),
        };

        let val_max = values.values().cloned().fold(0.0_f64, f64::max);
        let norm = LinearNorm::new(0.0, if val_max == 0.0 { 1.0 } else { val_max });

        // Half-block rasterization
        let effective_height = ph as usize * 2;

        for cy in 0..ph {
            for cx in 0..pw {
                let screen_x = px + cx;
                let screen_y = py + cy;
                if screen_x >= area.x + area.width || screen_y >= area.y + area.height {
                    continue;
                }

                let data_x = x_lo + (cx as f64 / pw as f64) * (x_hi - x_lo);

                // Top half-pixel
                let top_frac_y = (cy as usize * 2) as f64 / effective_height as f64;
                let top_data_y = y_hi - top_frac_y * (y_hi - y_lo);
                let top_key = pixel_to_hex(data_x - x_lo, top_data_y - y_lo, s);
                let top_val = values.get(&top_key).copied().unwrap_or(0.0);
                let top_color = self.colormap.color_at(norm.normalize(top_val));

                // Bottom half-pixel
                let bot_frac_y = (cy as usize * 2 + 1) as f64 / effective_height as f64;
                let bot_data_y = y_hi - bot_frac_y * (y_hi - y_lo);
                let bot_key = pixel_to_hex(data_x - x_lo, bot_data_y - y_lo, s);
                let bot_val = values.get(&bot_key).copied().unwrap_or(0.0);
                let bot_color = self.colormap.color_at(norm.normalize(bot_val));

                buf[(screen_x, screen_y)]
                    .set_char('▀')
                    .set_style(Style::default().fg(top_color).bg(bot_color));
            }
        }

        // Draw axes
        for x in px..px + pw {
            buf[(x, py + ph)]
                .set_char('─')
                .set_fg(self.theme.axis_color);
        }
        for y in py..py + ph {
            buf[(px.saturating_sub(1), y)]
                .set_char('│')
                .set_fg(self.theme.axis_color);
        }
    }
}
