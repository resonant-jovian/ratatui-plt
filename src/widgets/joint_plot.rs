//! Joint plot widget combining a central scatter plot with marginal distributions.
//!
//! Similar to seaborn's `jointplot` or Plotly's `marginal_x`/`marginal_y`.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::joint_plot::{JointPlot, MarginalType};
//!
//! let s = Series::new("data")
//!     .data(vec![(1.0, 2.0), (2.0, 3.0), (3.0, 1.5)])
//!     .color(Color::Cyan)
//!     .marker(MarkerShape::FilledCircle);
//!
//! let plot = JointPlot::new()
//!     .series(s)
//!     .marginal_x(MarginalType::Histogram)
//!     .marginal_y(MarginalType::Histogram)
//!     .title("Joint Plot");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::chars::CharSet;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendPosition};
use crate::plot_buffer::{PlotBackend, Z_MARKER, create_backend};
use crate::series::Series;
use crate::spines::Spines;
use crate::style::MarkerShape;
use crate::theme::Theme;
use crate::transform::data_to_screen;

/// Type of marginal distribution to display.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum MarginalType {
    /// No marginal distribution.
    #[default]
    None,
    /// Histogram (bar chart).
    Histogram,
    /// Kernel density estimate (filled curve).
    Kde,
    /// Rug marks (tick marks at data positions).
    Rug,
}

/// A joint plot widget combining a central scatter plot with marginal distributions.
///
/// Marginals can be histograms, KDE curves, or rug marks on the top and/or right
/// edges of the plot area.
pub struct JointPlot {
    series: Vec<Series>,
    marginal_x: MarginalType,
    marginal_y: MarginalType,
    marginal_ratio: f64,
    marginal_bins: usize,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    theme: Theme,
    spines: Spines,
    show_legend: bool,
    legend_position: LegendPosition,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for JointPlot {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            marginal_x: MarginalType::None,
            marginal_y: MarginalType::None,
            marginal_ratio: 0.2,
            marginal_bins: 20,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            theme: Theme::get_default(),
            spines: Spines::default(),
            show_legend: true,
            legend_position: LegendPosition::TopRight,
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl JointPlot {
    /// Create a new empty joint plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a data series.
    pub fn series(mut self, s: Series) -> Self {
        self.series.push(s);
        self
    }

    /// Set the top marginal distribution type.
    pub fn marginal_x(mut self, mt: MarginalType) -> Self {
        self.marginal_x = mt;
        self
    }

    /// Set the right marginal distribution type.
    pub fn marginal_y(mut self, mt: MarginalType) -> Self {
        self.marginal_y = mt;
        self
    }

    /// Set the fraction of area used for marginals (default 0.2).
    pub fn marginal_ratio(mut self, ratio: f64) -> Self {
        self.marginal_ratio = ratio;
        self
    }

    /// Set the number of histogram bins for marginals (default 20).
    pub fn marginal_bins(mut self, bins: usize) -> Self {
        self.marginal_bins = bins;
        self
    }

    /// Set the X axis.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y axis.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
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

    /// Show or hide the legend.
    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    /// Set the legend position.
    pub fn legend_position(mut self, pos: LegendPosition) -> Self {
        self.legend_position = pos;
        self
    }

    /// Add a reference line.
    pub fn reference_line(mut self, line: ReferenceLine) -> Self {
        self.reference_lines.push(line);
        self
    }

    /// Add an annotation.
    pub fn annotation(mut self, ann: Annotation) -> Self {
        self.annotations.push(ann);
        self
    }
}

impl Widget for &JointPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 6 || area.height < 6 {
            return;
        }

        let has_top = self.marginal_x != MarginalType::None;
        let has_right = self.marginal_y != MarginalType::None;

        // Compute marginal sizes
        let top_height = if has_top {
            (area.height as f64 * self.marginal_ratio).round() as u16
        } else {
            0
        };
        let right_width = if has_right {
            (area.width as f64 * self.marginal_ratio).round() as u16
        } else {
            0
        };

        // Ensure central area has enough space
        let central_width = area.width.saturating_sub(right_width);
        let central_height = area.height.saturating_sub(top_height);
        if central_width < 6 || central_height < 6 {
            return;
        }

        // Central scatter area
        let central_area = Rect::new(area.x, area.y + top_height, central_width, central_height);

        // Compute data bounds from all series
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

        // Render central scatter plot with PlotFrame
        let mut pb = create_backend(central_area);

        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render_to_pb(
            &mut pb,
            central_area,
            DataBounds {
                x_lo,
                x_hi,
                y_lo,
                y_hi,
            },
        ) else {
            return;
        };

        // Draw scatter points
        for s in &self.series {
            let marker = s.marker.unwrap_or(MarkerShape::Dot);
            for &(x, y) in &s.data {
                if !x.is_finite() || !y.is_finite() {
                    continue;
                }
                let sx = pa.screen_x(x);
                let sy = pa.screen_y(y);
                let xi = sx.round() as u16;
                let yi = sy.round() as u16;
                if pa.contains(xi, yi) {
                    let color = s.color.unwrap_or(self.theme.primary);
                    pb.set_char(xi, yi, marker.char(), color, Z_MARKER);
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Draw legend (after composite)
        if self.show_legend && !self.series.is_empty() {
            let legend = Legend::from_series(&self.series)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(pa.x, pa.y, pa.width, pa.height);
            (&legend).render(legend_area, buf);
        }

        // Collect all data for marginals
        let all_x: Vec<f64> = self
            .series
            .iter()
            .flat_map(|s| s.data.iter().map(|&(x, _)| x))
            .filter(|v| v.is_finite())
            .collect();
        let all_y: Vec<f64> = self
            .series
            .iter()
            .flat_map(|s| s.data.iter().map(|&(_, y)| y))
            .filter(|v| v.is_finite())
            .collect();

        let marginal_color = self
            .series
            .first()
            .and_then(|s| s.color)
            .unwrap_or(self.theme.primary);

        // Top marginal (x-axis distribution)
        if has_top && top_height >= 2 {
            let top_area = Rect::new(
                pa.x,
                area.y,
                pa.width,
                top_height.min(area.y + top_height - area.y),
            );
            let cfg = MarginalConfig {
                data: &all_x,
                marginal_type: &self.marginal_x,
                bins: self.marginal_bins,
                data_lo: pa.x_lo,
                data_hi: pa.x_hi,
                plot_origin: pa.x,
                plot_extent: pa.width,
                color: marginal_color,
                chars: &self.theme.chars,
            };
            render_marginal_top(buf, top_area, &cfg);
        }

        // Right marginal (y-axis distribution)
        if has_right && right_width >= 2 {
            let right_area = Rect::new(area.x + central_width, pa.y, right_width, pa.height);
            let cfg = MarginalConfig {
                data: &all_y,
                marginal_type: &self.marginal_y,
                bins: self.marginal_bins,
                data_lo: pa.y_lo,
                data_hi: pa.y_hi,
                plot_origin: pa.y,
                plot_extent: pa.height,
                color: marginal_color,
                chars: &self.theme.chars,
            };
            render_marginal_right(buf, right_area, &cfg);
        }
    }
}

/// Configuration for rendering a marginal distribution.
struct MarginalConfig<'a> {
    data: &'a [f64],
    marginal_type: &'a MarginalType,
    bins: usize,
    data_lo: f64,
    data_hi: f64,
    /// Screen origin of the aligned axis in the central plot.
    plot_origin: u16,
    /// Screen extent of the aligned axis in the central plot.
    plot_extent: u16,
    color: Color,
    chars: &'a CharSet,
}

/// Render the top marginal distribution.
fn render_marginal_top(buf: &mut Buffer, area: Rect, cfg: &MarginalConfig<'_>) {
    if cfg.data.is_empty() || area.width < 2 || area.height < 1 {
        return;
    }

    match cfg.marginal_type {
        MarginalType::None => {}
        MarginalType::Histogram => {
            let bin_counts = compute_histogram(cfg.data, cfg.bins, cfg.data_lo, cfg.data_hi);
            let max_count = bin_counts.iter().copied().max().unwrap_or(0);
            if max_count == 0 {
                return;
            }
            let bin_width_data = (cfg.data_hi - cfg.data_lo) / cfg.bins as f64;

            for (i, &count) in bin_counts.iter().enumerate() {
                let bin_center = cfg.data_lo + (i as f64 + 0.5) * bin_width_data;
                let sx = data_to_screen(
                    bin_center,
                    cfg.data_lo,
                    cfg.data_hi,
                    cfg.plot_origin as f64,
                    (cfg.plot_origin + cfg.plot_extent - 1) as f64,
                );
                let xi = sx.round() as u16;

                let bar_height =
                    ((count as f64 / max_count as f64) * area.height as f64).round() as u16;

                for dy in 0..bar_height {
                    let y = area.y + area.height - 1 - dy;
                    if xi >= area.x && xi < area.x + area.width && y >= area.y {
                        buf[(xi, y)]
                            .set_char(cfg.chars.fill.solid)
                            .set_fg(cfg.color);
                    }
                }
            }
        }
        MarginalType::Kde => {
            let n_points = area.width as usize;
            let kde = compute_kde(cfg.data, n_points, cfg.data_lo, cfg.data_hi);
            let max_val = kde.iter().map(|&(_, v)| v).fold(0.0_f64, f64::max);

            if max_val <= 0.0 {
                return;
            }

            for &(x_val, density) in &kde {
                let sx = data_to_screen(
                    x_val,
                    cfg.data_lo,
                    cfg.data_hi,
                    cfg.plot_origin as f64,
                    (cfg.plot_origin + cfg.plot_extent - 1) as f64,
                );
                let xi = sx.round() as u16;
                let bar_height = ((density / max_val) * area.height as f64).round() as u16;

                for dy in 0..bar_height {
                    let y = area.y + area.height - 1 - dy;
                    if xi >= area.x && xi < area.x + area.width && y >= area.y {
                        buf[(xi, y)]
                            .set_char(cfg.chars.fill.light)
                            .set_fg(cfg.color);
                    }
                }
            }
        }
        MarginalType::Rug => {
            let y = area.y + area.height - 1;
            for &val in cfg.data {
                let sx = data_to_screen(
                    val,
                    cfg.data_lo,
                    cfg.data_hi,
                    cfg.plot_origin as f64,
                    (cfg.plot_origin + cfg.plot_extent - 1) as f64,
                );
                let xi = sx.round() as u16;
                if xi >= area.x && xi < area.x + area.width && y >= area.y {
                    buf[(xi, y)]
                        .set_char(cfg.chars.border.vertical)
                        .set_fg(cfg.color);
                }
            }
        }
    }
}

/// Render the right marginal distribution.
fn render_marginal_right(buf: &mut Buffer, area: Rect, cfg: &MarginalConfig<'_>) {
    if cfg.data.is_empty() || area.width < 1 || area.height < 2 {
        return;
    }

    match cfg.marginal_type {
        MarginalType::None => {}
        MarginalType::Histogram => {
            let bin_counts = compute_histogram(cfg.data, cfg.bins, cfg.data_lo, cfg.data_hi);
            let max_count = bin_counts.iter().copied().max().unwrap_or(0);
            if max_count == 0 {
                return;
            }
            let bin_width_data = (cfg.data_hi - cfg.data_lo) / cfg.bins as f64;

            for (i, &count) in bin_counts.iter().enumerate() {
                let bin_center = cfg.data_lo + (i as f64 + 0.5) * bin_width_data;
                // y-axis is inverted on screen
                let sy = data_to_screen(
                    bin_center,
                    cfg.data_lo,
                    cfg.data_hi,
                    (cfg.plot_origin + cfg.plot_extent - 1) as f64,
                    cfg.plot_origin as f64,
                );
                let yi = sy.round() as u16;

                let bar_width =
                    ((count as f64 / max_count as f64) * area.width as f64).round() as u16;

                for dx in 0..bar_width {
                    let x = area.x + dx;
                    if x < area.x + area.width && yi >= area.y && yi < area.y + area.height {
                        buf[(x, yi)]
                            .set_char(cfg.chars.fill.solid)
                            .set_fg(cfg.color);
                    }
                }
            }
        }
        MarginalType::Kde => {
            let n_points = area.height as usize;
            let kde = compute_kde(cfg.data, n_points, cfg.data_lo, cfg.data_hi);
            let max_val = kde.iter().map(|&(_, v)| v).fold(0.0_f64, f64::max);

            if max_val <= 0.0 {
                return;
            }

            for &(y_val, density) in &kde {
                let sy = data_to_screen(
                    y_val,
                    cfg.data_lo,
                    cfg.data_hi,
                    (cfg.plot_origin + cfg.plot_extent - 1) as f64,
                    cfg.plot_origin as f64,
                );
                let yi = sy.round() as u16;
                let bar_width = ((density / max_val) * area.width as f64).round() as u16;

                for dx in 0..bar_width {
                    let x = area.x + dx;
                    if x < area.x + area.width && yi >= area.y && yi < area.y + area.height {
                        buf[(x, yi)]
                            .set_char(cfg.chars.fill.light)
                            .set_fg(cfg.color);
                    }
                }
            }
        }
        MarginalType::Rug => {
            let x = area.x;
            for &val in cfg.data {
                let sy = data_to_screen(
                    val,
                    cfg.data_lo,
                    cfg.data_hi,
                    (cfg.plot_origin + cfg.plot_extent - 1) as f64,
                    cfg.plot_origin as f64,
                );
                let yi = sy.round() as u16;
                if yi >= area.y && yi < area.y + area.height && x < area.x + area.width {
                    buf[(x, yi)]
                        .set_char(cfg.chars.border.horizontal)
                        .set_fg(cfg.color);
                }
            }
        }
    }
}

/// Compute a histogram of `data` into `bins` equally-spaced bins over `[lo, hi]`.
fn compute_histogram(data: &[f64], bins: usize, lo: f64, hi: f64) -> Vec<usize> {
    let bins = bins.max(1);
    let mut counts = vec![0usize; bins];
    let range = hi - lo;
    if range <= 0.0 {
        return counts;
    }

    for &val in data {
        if !val.is_finite() {
            continue;
        }
        let t = (val - lo) / range;
        let idx = (t * bins as f64).floor() as isize;
        if idx >= 0 && (idx as usize) < bins {
            counts[idx as usize] += 1;
        } else if idx as usize == bins {
            // Include right edge in last bin
            counts[bins - 1] += 1;
        }
    }
    counts
}

/// Compute a Gaussian KDE of `data` evaluated at `n_points` evenly spaced points in `[lo, hi]`.
///
/// Uses Silverman's rule of thumb for bandwidth selection.
pub fn compute_kde(data: &[f64], n_points: usize, lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let n = data.len();
    if n == 0 || n_points == 0 || hi <= lo {
        return Vec::new();
    }

    let nf = n as f64;

    // Compute mean, std, IQR
    let mean: f64 = data.iter().sum::<f64>() / nf;
    let variance: f64 = data.iter().map(|&v| (v - mean) * (v - mean)).sum::<f64>() / nf;
    let std_dev = variance.sqrt();

    // Compute IQR from sorted data
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q1 = percentile_sorted(&sorted, 25.0);
    let q3 = percentile_sorted(&sorted, 75.0);
    let iqr = q3 - q1;

    // Silverman bandwidth: h = 0.9 * min(std, iqr/1.34) * n^(-1/5)
    let spread = if iqr > 0.0 {
        std_dev.min(iqr / 1.34)
    } else {
        std_dev
    };
    let h = if spread > 0.0 {
        0.9 * spread * nf.powf(-0.2)
    } else {
        // Fallback: use range / bins
        (hi - lo) / n_points as f64
    };

    if h <= 0.0 {
        return Vec::new();
    }

    let step = (hi - lo) / (n_points.max(1) - 1).max(1) as f64;
    let mut result = Vec::with_capacity(n_points);

    for i in 0..n_points {
        let x = lo + i as f64 * step;
        let mut density = 0.0;
        for &d in data {
            let u = (x - d) / h;
            // Gaussian kernel: 1/sqrt(2*pi) * exp(-u^2/2)
            density += (-0.5 * u * u).exp();
        }
        density /= nf * h * (2.0 * std::f64::consts::PI).sqrt();
        result.push((x, density));
    }

    result
}

/// Compute a percentile from a sorted array using linear interpolation.
fn percentile_sorted(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = pct / 100.0 * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = idx - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}
