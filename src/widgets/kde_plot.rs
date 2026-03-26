//! Kernel density estimation (KDE) plot widget.
//!
//! Renders 1D kernel density estimates as smooth filled curves. Each dataset
//! is independently fit using [`crate::statistics::Kde`] and drawn as a Braille
//! line with optional fill underneath.
//!
//! Requires the `statistics` feature.
//!
//! # Example
//!
//! ```rust
//! use ratatui_plt::prelude::*;
//!
//! let plot = KDEPlot::new()
//!     .dataset(KdeDataset::new("sample", vec![1.0, 2.0, 2.5, 3.0, 4.0]))
//!     .fill(true)
//!     .title("Density");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::drawing::draw_braille_line_pb;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::plot_buffer::{PlotBuffer, Z_DATA, Z_FILL};
use crate::spines::Spines;
use crate::statistics::{BandwidthMethod, Kde};
use crate::theme::Theme;

/// A single dataset for the KDE plot.
#[derive(Clone, Debug)]
pub struct KdeDataset {
    /// Display name for the legend.
    name: String,
    /// Raw 1D observations.
    data: Vec<f64>,
    /// Optional explicit color (auto-assigned from theme cycle if `None`).
    color: Option<Color>,
    /// Bandwidth selection method (defaults to Silverman).
    bandwidth: Option<BandwidthMethod>,
}

impl KdeDataset {
    /// Create a new KDE dataset with a name and raw observations.
    pub fn new(name: impl Into<String>, data: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            data,
            color: None,
            bandwidth: None,
        }
    }

    /// Set an explicit color for this dataset.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the bandwidth selection method.
    pub fn bandwidth(mut self, bw: BandwidthMethod) -> Self {
        self.bandwidth = Some(bw);
        self
    }
}

/// A 1D kernel density estimation plot.
///
/// Computes a KDE for each dataset and renders the resulting density curves
/// as Braille lines with optional filled regions underneath.
#[derive(Clone)]
pub struct KDEPlot {
    datasets: Vec<KdeDataset>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    fill: bool,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
    show_legend: bool,
    legend_position: LegendPosition,
}

impl Default for KDEPlot {
    fn default() -> Self {
        Self {
            datasets: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            fill: true,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
            show_legend: true,
            legend_position: LegendPosition::TopRight,
        }
    }
}

impl KDEPlot {
    /// Create an empty KDE plot with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a dataset to the plot.
    pub fn dataset(mut self, ds: KdeDataset) -> Self {
        self.datasets.push(ds);
        self
    }

    /// Enable or disable fill under the density curves.
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Set the plot title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the X axis configuration.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y axis configuration.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
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

    /// Add an annotation.
    pub fn annotation(mut self, ann: Annotation) -> Self {
        self.annotations.push(ann);
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
}

/// Computed KDE result for a single dataset, ready for rendering.
struct KdeResult {
    x_vals: Vec<f64>,
    y_vals: Vec<f64>,
    color: Color,
    name: String,
}

impl Widget for &KDEPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.datasets.is_empty() {
            return;
        }

        // Compute KDE for each dataset
        let mut color_cycle = self.theme.color_cycle.clone();
        let mut results: Vec<KdeResult> = Vec::with_capacity(self.datasets.len());

        for ds in &self.datasets {
            // Filter to finite values only
            let finite: Vec<f64> = ds.data.iter().copied().filter(|v| v.is_finite()).collect();
            if finite.is_empty() {
                continue;
            }

            let mut kde = Kde::new();
            if let Some(ref bw) = ds.bandwidth {
                kde = kde.bandwidth(bw.clone());
            }

            let (x_vals, y_vals) = kde.fit(&finite);
            if x_vals.is_empty() {
                continue;
            }

            let color = ds.color.unwrap_or_else(|| color_cycle.next_color());

            results.push(KdeResult {
                x_vals,
                y_vals,
                color,
                name: ds.name.clone(),
            });
        }

        if results.is_empty() {
            return;
        }

        // Compute global axis bounds from all KDE outputs
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for r in &results {
            for &x in &r.x_vals {
                if x < x_min {
                    x_min = x;
                }
                if x > x_max {
                    x_max = x;
                }
            }
            for &y in &r.y_vals {
                if y > y_max {
                    y_max = y;
                }
            }
        }

        // Density is always non-negative; start y at 0
        let y_min = 0.0;
        // Small padding on y_max so the curve does not touch the top edge
        let y_max = if y_max > 0.0 { y_max * 1.05 } else { 1.0 };

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let mut pb = PlotBuffer::new(area);

        // Create and render the plot frame (title, axes, grid, ticks, labels, spines)
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let bounds = DataBounds {
            x_lo,
            x_hi,
            y_lo,
            y_hi,
        };

        let Some(pa) = frame.render_to_pb(&mut pb, area, bounds) else {
            return;
        };

        // Render each dataset
        for (si, r) in results.iter().enumerate() {
            // --- Fill under the curve ---
            if self.fill {
                let baseline_screen = pa.screen_y(0.0);

                for col_offset in 0..pa.width {
                    let screen_x = pa.x + col_offset;

                    // Map screen column back to data x
                    let data_x = x_lo
                        + (col_offset as f64 / (pa.width.saturating_sub(1)).max(1) as f64)
                            * (x_hi - x_lo);

                    // Interpolate density at this x from the KDE output
                    let data_y = interp_y_at(&r.x_vals, &r.y_vals, data_x);
                    if !data_y.is_finite() || data_y <= 0.0 {
                        continue;
                    }

                    let sy = pa.screen_y(data_y);
                    let y_top = sy.round().min(baseline_screen.round()) as u16;
                    let y_bot = sy.round().max(baseline_screen.round()) as u16;

                    for y in y_top..=y_bot {
                        if pa.contains(screen_x, y) {
                            pb.set_bg(screen_x, y, r.color, Z_FILL);
                        }
                    }
                }
            }

            // --- Draw the density curve as connected Braille line segments ---
            if r.x_vals.len() >= 2 {
                for i in 0..r.x_vals.len() - 1 {
                    let x0 = r.x_vals[i];
                    let y0 = r.y_vals[i];
                    let x1 = r.x_vals[i + 1];
                    let y1 = r.y_vals[i + 1];

                    if !x0.is_finite() || !y0.is_finite() || !x1.is_finite() || !y1.is_finite() {
                        continue;
                    }

                    let sx0 = pa.screen_x(x0);
                    let sy0 = pa.screen_y(y0);
                    let sx1 = pa.screen_x(x1);
                    let sy1 = pa.screen_y(y1);

                    draw_braille_line_pb(
                        &mut pb,
                        sx0,
                        sy0,
                        sx1,
                        sy1,
                        r.color,
                        &pa,
                        Z_DATA + si as u8,
                    );
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite to buffer before legend
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Draw legend
        if self.show_legend && !results.is_empty() {
            let fill_ch = self.theme.chars.fill.solid;
            let entries: Vec<LegendEntry> = results
                .iter()
                .map(|r| LegendEntry {
                    name: r.name.clone(),
                    color: r.color,
                    marker: Some(fill_ch),
                })
                .collect();
            let legend = Legend::new(entries)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(pa.x, pa.y, pa.width, pa.height);
            (&legend).render(legend_area, buf);
        }
    }
}

/// Linearly interpolate a y value from parallel x and y arrays at a given x.
///
/// Returns `f64::NAN` when arrays are empty. Clamps to endpoint values when
/// `x` falls outside the data range. The x array must be sorted ascending.
fn interp_y_at(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let n = xs.len().min(ys.len());
    if n == 0 {
        return f64::NAN;
    }
    if n == 1 {
        return ys[0];
    }
    if x <= xs[0] {
        return ys[0];
    }
    let last = n - 1;
    if x >= xs[last] {
        return ys[last];
    }
    for i in 0..last {
        if x >= xs[i] && x <= xs[i + 1] {
            let dx = xs[i + 1] - xs[i];
            if dx.abs() < 1e-15 {
                return ys[i];
            }
            let t = (x - xs[i]) / dx;
            return ys[i] + t * (ys[i + 1] - ys[i]);
        }
    }
    ys[last]
}
