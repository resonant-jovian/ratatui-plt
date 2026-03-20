//! Scatter plot widget with color and size mapping.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::axis::{AspectRatio, Axis};
use crate::colormap::{Colormap, Viridis};
use crate::legend::{Legend, LegendPosition};
use crate::norm::{LinearNorm, Normalize};
use crate::series::Series;
use crate::style::MarkerShape;
use crate::theme::Theme;
use crate::transform::{apply_aspect_ratio, data_to_screen};

/// A 2D scatter plot widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let plot = ScatterPlot::new()
///     .series(Series::new("data").data(vec![(1.0, 2.0), (3.0, 4.0)]).marker(MarkerShape::Circle));
/// ```
pub struct ScatterPlot {
    series: Vec<Series>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    aspect_ratio: AspectRatio,
    show_legend: bool,
    legend_position: LegendPosition,
    /// Optional per-point color values (for single-series color mapping).
    color_values: Option<Vec<f64>>,
    /// Colormap for color-mapped points.
    colormap: Box<dyn Colormap>,
    /// Normalizer for color values.
    color_norm: Box<dyn Normalize>,
    theme: Theme,
}

impl Default for ScatterPlot {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            aspect_ratio: AspectRatio::Auto,
            show_legend: true,
            legend_position: LegendPosition::TopRight,
            color_values: None,
            colormap: Box::new(Viridis),
            color_norm: Box::new(LinearNorm::new(0.0, 1.0)),
            theme: Theme::get_default(),
        }
    }
}

impl ScatterPlot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn series(mut self, s: Series) -> Self {
        self.series.push(s);
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

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
        self
    }

    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    pub fn legend_position(mut self, pos: LegendPosition) -> Self {
        self.legend_position = pos;
        self
    }

    /// Set per-point color values for color mapping.
    pub fn color_values(mut self, values: Vec<f64>) -> Self {
        let vmin = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let vmax = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        self.color_norm = Box::new(LinearNorm::new(vmin, vmax));
        self.color_values = Some(values);
        self
    }

    /// Set the colormap for color-mapped points.
    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    /// Set the normalization for color values.
    pub fn color_norm(mut self, norm: impl Normalize + 'static) -> Self {
        self.color_norm = Box::new(norm);
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

impl Widget for &ScatterPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let y_label_width: u16 = 8;
        let tick_height: u16 = 1;
        let x_label_height: u16 = if self.x_axis.label.is_some() { 1 } else { 0 };

        let plot_x = area.x + y_label_width;
        let plot_y = area.y + title_height;
        let plot_width = area.width.saturating_sub(y_label_width + 1);
        let plot_height = area
            .height
            .saturating_sub(title_height + tick_height + x_label_height);

        if plot_width < 2 || plot_height < 2 {
            return;
        }

        // Compute data bounds
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for s in &self.series {
            if let Some((lo, hi)) = s.x_bounds() {
                x_min = x_min.min(lo);
                x_max = x_max.max(hi);
            }
            if let Some((lo, hi)) = s.y_bounds() {
                y_min = y_min.min(lo);
                y_max = y_max.max(hi);
            }
        }
        if x_min.is_infinite() {
            x_min = 0.0;
            x_max = 1.0;
        }
        if y_min.is_infinite() {
            y_min = 0.0;
            y_max = 1.0;
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let (ax_off, ay_off, aw, ah) = apply_aspect_ratio(
            &self.aspect_ratio,
            x_hi - x_lo,
            y_hi - y_lo,
            plot_width,
            plot_height,
        );
        let px = plot_x + ax_off;
        let py = plot_y + ay_off;

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

        // Draw axes
        for x in px..px + aw {
            if x < area.x + area.width {
                buf[(x, py + ah)]
                    .set_char('─')
                    .set_fg(self.theme.axis_color);
            }
        }
        for y in py..py + ah {
            buf[(px.saturating_sub(1), y)]
                .set_char('│')
                .set_fg(self.theme.axis_color);
        }

        // Draw grid
        let x_grid = self.x_axis.grid || self.theme.grid_visible;
        let y_grid = self.y_axis.grid || self.theme.grid_visible;
        if x_grid {
            let gx_ticks = self.x_axis.tick_positions(x_lo, x_hi);
            for &tv in &gx_ticks {
                let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                let xi = sx.round() as u16;
                if xi >= px && xi < px + aw {
                    for y in py..py + ah {
                        buf[(xi, y)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }
        if y_grid {
            let gy_ticks = self.y_axis.tick_positions(y_lo, y_hi);
            for &tv in &gy_ticks {
                let sy = data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ah {
                    for x in px..px + aw {
                        buf[(x, yi)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }

        // Draw tick labels
        let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let label_start = xi.saturating_sub(label.len() as u16 / 2);
            let y = py + ah;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }

        let y_ticks = self.y_axis.tick_positions(y_lo, y_hi);
        for &tv in &y_ticks {
            let sy = data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
            let label = self.y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ah {
                let label_start = px.saturating_sub(label.len() as u16 + 1);
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < px {
                        buf[(lx, yi)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }

        // Draw scatter points (skip NaN)
        let mut global_point_idx = 0usize;
        for s in &self.series {
            let marker = s.marker.unwrap_or(MarkerShape::Dot);
            for &(x, y) in &s.data {
                if !x.is_finite() || !y.is_finite() {
                    global_point_idx += 1;
                    continue;
                }
                let sx = data_to_screen(x, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                let sy = data_to_screen(y, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
                let xi = sx.round() as u16;
                let yi = sy.round() as u16;

                if xi >= px && xi < px + aw && yi >= py && yi < py + ah {
                    let color = if let Some(ref cv) = self.color_values {
                        if global_point_idx < cv.len() {
                            let t = self.color_norm.normalize(cv[global_point_idx]);
                            self.colormap.color_at(t)
                        } else {
                            s.color
                        }
                    } else {
                        s.color
                    };
                    buf[(xi, yi)].set_char(marker.char()).set_fg(color);
                }
                global_point_idx += 1;
            }
        }

        // Draw legend
        if self.show_legend && !self.series.is_empty() && self.color_values.is_none() {
            let legend = Legend::from_series(&self.series)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(px, py, aw, ah);
            (&legend).render(legend_area, buf);
        }
    }
}
