//! Hexagonal binning plot for large datasets.

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

        // Simple grid-based hexagonal binning approximation
        // Use rectangular bins as an approximation that works well in terminal cells
        let n_cols = self.gridsize;
        let n_rows = (self.gridsize as f64 * ph as f64 / pw as f64)
            .round()
            .max(3.0) as usize;

        let bin_w = (x_hi - x_lo) / n_cols as f64;
        let bin_h = (y_hi - y_lo) / n_rows as f64;

        // Accumulate into bins
        let mut counts = vec![vec![0.0f64; n_cols]; n_rows];
        let mut weights_sum = vec![vec![0.0f64; n_cols]; n_rows];

        for (idx, &(x, y)) in self.data.iter().enumerate() {
            let col = ((x - x_lo) / bin_w).floor() as usize;
            let row = ((y - y_lo) / bin_h).floor() as usize;
            let col = col.min(n_cols - 1);
            let row = row.min(n_rows - 1);
            counts[row][col] += 1.0;
            if let Some(ref w) = self.weights {
                weights_sum[row][col] += w.get(idx).copied().unwrap_or(1.0);
            }
        }

        // Compute display values
        let values: Vec<Vec<f64>> = match self.aggregation {
            HexAggregation::Count => counts.clone(),
            HexAggregation::Sum => weights_sum.clone(),
            HexAggregation::Mean => counts
                .iter()
                .zip(weights_sum.iter())
                .map(|(cr, wr)| {
                    cr.iter()
                        .zip(wr.iter())
                        .map(|(&c, &w)| if c > 0.0 { w / c } else { 0.0 })
                        .collect()
                })
                .collect(),
        };

        let val_max = values
            .iter()
            .flat_map(|r| r.iter())
            .cloned()
            .fold(0.0f64, f64::max);
        let norm = LinearNorm::new(0.0, if val_max == 0.0 { 1.0 } else { val_max });

        // Render bins
        let cell_w = pw as f64 / n_cols as f64;
        let cell_h = ph as f64 / n_rows as f64;

        for (row, row_values) in values.iter().enumerate().take(n_rows) {
            for (col, &val) in row_values.iter().enumerate().take(n_cols) {
                if val == 0.0 {
                    continue;
                }
                let t = norm.normalize(val);
                let color = self.colormap.color_at(t);

                let sx_start = (px as f64 + col as f64 * cell_w).round() as u16;
                let sx_end = (px as f64 + (col + 1) as f64 * cell_w).round() as u16;
                let sy_start = (py as f64 + (n_rows - 1 - row) as f64 * cell_h).round() as u16;
                let sy_end = (py as f64 + (n_rows - row) as f64 * cell_h).round() as u16;

                // Use hexagonal-ish character ⬢ for hex bins
                for sy in sy_start..sy_end {
                    for sx in sx_start..sx_end {
                        if sx >= px && sx < px + pw && sy >= py && sy < py + ph {
                            buf[(sx, sy)]
                                .set_char('⬢')
                                .set_style(Style::default().fg(color));
                        }
                    }
                }
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
