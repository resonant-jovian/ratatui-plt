//! Horizon graph widget for compact time-series visualization.
//!
//! A horizon graph divides the y-range into color bands and folds them into
//! the same vertical space. Higher bands use progressively darker/more intense
//! colors, compressing N-times more data into the same screen height.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//!
//! let s = Series::new("Temperature")
//!     .data((0..200).map(|i| {
//!         let x = i as f64 * 0.05;
//!         (x, (x * 0.7).sin() * 3.0 + (x * 2.3).cos())
//!     }).collect())
//!     .color(Color::Cyan);
//!
//! let horizon = HorizonGraph::new()
//!     .series(s)
//!     .n_bands(3)
//!     .title("Temperature Anomaly");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBackend, create_backend, Z_DATA};
use crate::series::Series;
use crate::spines::Spines;
use crate::theme::Theme;

/// A horizon graph widget for compact time-series visualization.
///
/// Horizon graphs fold multiple band-levels of data into the same vertical
/// space, using color intensity to distinguish bands. This allows dense
/// comparison of many time series in minimal screen area.
#[derive(Clone)]
pub struct HorizonGraph {
    series: Vec<Series>,
    n_bands: usize,
    x_axis: Axis,
    title: Option<String>,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
}

impl Default for HorizonGraph {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            n_bands: 3,
            x_axis: Axis::new(),
            title: None,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
        }
    }
}

impl HorizonGraph {
    /// Create an empty horizon graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a data series.
    pub fn series(mut self, s: Series) -> Self {
        self.series.push(s);
        self
    }

    /// Add multiple series.
    pub fn series_vec(mut self, s: Vec<Series>) -> Self {
        self.series.extend(s);
        self
    }

    /// Set the number of color bands (default: 3).
    ///
    /// More bands compress more data range into the same vertical space,
    /// but require finer color discrimination.
    pub fn n_bands(mut self, n: usize) -> Self {
        self.n_bands = n.max(1);
        self
    }

    /// Set the X axis.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the plot title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the theme.
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
}

/// Compute band color by scaling a base color's brightness by an alpha factor.
///
/// `alpha` ranges from 0.0 (fully dark) to 1.0 (full base color).
/// For non-RGB colors, falls back to fixed intensity tiers.
fn band_color(base: Color, alpha: f64) -> Color {
    let a = alpha.clamp(0.0, 1.0);
    match base {
        Color::Rgb(r, g, b) => {
            let r2 = (r as f64 * a).round().min(255.0) as u8;
            let g2 = (g as f64 * a).round().min(255.0) as u8;
            let b2 = (b as f64 * a).round().min(255.0) as u8;
            Color::Rgb(r2, g2, b2)
        }
        _ => {
            // For named colors, approximate intensity with shade characters
            // by blending toward the base color
            if a < 0.33 {
                Color::DarkGray
            } else if a < 0.66 {
                Color::Gray
            } else {
                base
            }
        }
    }
}

/// Linearly interpolate a y value from sorted (x, y) data at a given x.
///
/// Returns `f64::NAN` when `data` is empty. Clamps to endpoint values
/// when `x` falls outside the data range.
fn interp_y_at(data: &[(f64, f64)], x: f64) -> f64 {
    if data.is_empty() {
        return f64::NAN;
    }
    if data.len() == 1 {
        return data[0].1;
    }
    if x <= data[0].0 {
        return data[0].1;
    }
    let last = data.len() - 1;
    if x >= data[last].0 {
        return data[last].1;
    }
    for i in 0..last {
        let (x0, y0) = data[i];
        let (x1, y1) = data[i + 1];
        if x >= x0 && x <= x1 {
            let dx = x1 - x0;
            if dx.abs() < 1e-15 {
                return y0;
            }
            let t = (x - x0) / dx;
            return y0 + t * (y1 - y0);
        }
    }
    data[last].1
}

impl HorizonGraph {
    /// Compute x-bounds across all series.
    fn compute_x_bounds(&self) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for s in &self.series {
            if let Some((lo, hi)) = s.x_bounds() {
                min = min.min(lo);
                max = max.max(hi);
            }
        }
        if min.is_infinite() {
            (0.0, 1.0)
        } else {
            (min, max)
        }
    }

    /// Compute y-bounds across all series.
    fn compute_y_bounds(&self) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for s in &self.series {
            if let Some((lo, hi)) = s.y_bounds() {
                min = min.min(lo);
                max = max.max(hi);
            }
        }
        if min.is_infinite() {
            (0.0, 1.0)
        } else {
            (min, max)
        }
    }
}

impl Widget for &HorizonGraph {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.series.is_empty() || area.width < 4 || area.height < 3 {
            return;
        }

        let (data_x_min, data_x_max) = self.compute_x_bounds();
        let (data_y_min, data_y_max) = self.compute_y_bounds();

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(data_x_min, data_x_max);

        // For horizon graphs, the y-axis is not displayed (bands encode it),
        // but we need the full range for band computation.
        let y_lo = data_y_min;
        let y_hi = data_y_max;
        let y_range = y_hi - y_lo;

        if y_range.abs() < 1e-15 {
            return;
        }

        let n_bands = self.n_bands.max(1);
        let band_height = y_range / n_bands as f64;

        // Create a dummy y-axis with no ticks/labels for frame rendering
        let y_axis_hidden = Axis::new();

        let mut pb = create_backend(area);

        let frame = PlotFrame::new(&self.x_axis, &y_axis_hidden, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .y_label_width(0)
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render_to_pb(
            &mut pb,
            area,
            DataBounds {
                x_lo,
                x_hi,
                y_lo: 0.0,
                y_hi: 1.0,
            },
        ) else {
            return;
        };

        // Resolve series colors from explicit color or theme cycle
        let mut color_cycle = self.theme.color_cycle.clone();
        let resolved_colors: Vec<Color> = self
            .series
            .iter()
            .map(|s| {
                if let Some(c) = s.color {
                    c
                } else {
                    color_cycle.next_color()
                }
            })
            .collect();

        let n_series = self.series.len();

        // Compute vertical space per series in the plot area (using half-blocks
        // for 2x vertical resolution)
        let effective_height = pa.height as usize * 2;
        let rows_per_series = effective_height.checked_div(n_series).unwrap_or(0);

        if rows_per_series == 0 {
            pb.composite(buf);
            frame.draw_end_labels(buf, area, &pa);
            return;
        }

        for (si, s) in self.series.iter().enumerate() {
            let base_color = resolved_colors[si];
            let lane_top_half = si * rows_per_series;

            // Filter valid data and sort by x for interpolation
            let mut valid_data: Vec<(f64, f64)> = s
                .data
                .iter()
                .copied()
                .filter(|(x, y)| x.is_finite() && y.is_finite())
                .collect();
            valid_data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

            if valid_data.is_empty() {
                continue;
            }

            // Render each screen column
            for col_offset in 0..pa.width {
                let screen_x = pa.x + col_offset;

                // Map screen column to data x
                let frac = col_offset as f64 / (pa.width.saturating_sub(1)).max(1) as f64;
                let data_x = x_lo + frac * (x_hi - x_lo);

                // Interpolate y at this x
                let data_y = interp_y_at(&valid_data, data_x);
                if !data_y.is_finite() {
                    continue;
                }

                // Clamp to data range
                let clamped_y = data_y.clamp(y_lo, y_hi);

                // Determine which band and fractional position within it
                let offset_from_min = clamped_y - y_lo;
                let band_idx_f = offset_from_min / band_height;
                let band_idx = (band_idx_f.floor() as usize).min(n_bands - 1);
                let frac_in_band = band_idx_f - band_idx as f64;

                // Compute fill height in half-rows within this lane
                let fill_half_rows = (frac_in_band * rows_per_series as f64).round() as usize;

                // Color intensity increases with band index
                let alpha = (band_idx as f64 + 1.0) / n_bands as f64;
                let color = band_color(base_color, alpha);

                // Fill from the bottom of the lane upward
                for hr in 0..fill_half_rows.min(rows_per_series) {
                    let half_row = lane_top_half + (rows_per_series - 1 - hr);
                    let cell_y = pa.y + (half_row / 2) as u16;
                    let is_top_half = half_row.is_multiple_of(2);

                    if cell_y < pa.y || cell_y >= pa.y + pa.height {
                        continue;
                    }
                    if screen_x < pa.x || screen_x >= pa.x + pa.width {
                        continue;
                    }

                    if is_top_half {
                        // Set fg color for the top-half character
                        pb.set_cell(
                            screen_x,
                            cell_y,
                            self.theme.chars.fill.half_upper,
                            color,
                            self.theme.background,
                            Z_DATA + si as u8,
                        );
                    } else {
                        // Set bg color for the bottom-half
                        pb.set_cell(
                            screen_x,
                            cell_y,
                            self.theme.chars.fill.half_upper,
                            self.theme.background,
                            color,
                            Z_DATA + si as u8,
                        );
                    }
                }
            }
        }

        pb.composite(buf);
        frame.draw_end_labels(buf, area, &pa);
    }
}
