//! Dot plot (Wilkinson) widget for distribution visualization.
//!
//! Renders stacked dots at binned positions along an axis, showing the count of
//! observations per bin as vertically stacked marker characters. Like a histogram
//! but with individual dots instead of bars.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::dot_plot::{DotDataset, DotPlot};
//! use ratatui::style::Color;
//!
//! let plot = DotPlot::new()
//!     .dataset(DotDataset::new("sample", vec![1.0, 1.2, 2.0, 2.1, 2.2, 3.0]))
//!     .title("Dot Plot");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::color_cycle::ColorCycle;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::plot_buffer::{PlotBackend, create_backend, Z_MARKER};
use crate::spines::Spines;
use crate::style::MarkerShape;
use crate::theme::Theme;

/// A single dataset for the dot plot.
#[derive(Clone, Debug)]
pub struct DotDataset {
    /// Dataset name (for legend).
    pub name: String,
    /// Raw data values.
    pub data: Vec<f64>,
    /// Optional color override.
    pub color: Option<Color>,
}

impl DotDataset {
    /// Create a new dot dataset.
    pub fn new(name: impl Into<String>, data: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            data,
            color: None,
        }
    }

    /// Set the dot color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A dot plot (Wilkinson) widget.
///
/// Stacks dots at binned positions along a horizontal axis. Each dot
/// represents one observation, making the distribution shape visible
/// while preserving individual data points.
pub struct DotPlot {
    datasets: Vec<DotDataset>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    bin_width: Option<f64>,
    marker: MarkerShape,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    show_legend: bool,
    legend_position: LegendPosition,
}

impl DotPlot {
    /// Create a new empty dot plot.
    pub fn new() -> Self {
        Self {
            datasets: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            bin_width: None,
            marker: MarkerShape::FilledCircle,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            show_legend: true,
            legend_position: LegendPosition::TopRight,
        }
    }

    /// Add a dataset.
    pub fn dataset(mut self, ds: DotDataset) -> Self {
        self.datasets.push(ds);
        self
    }

    /// Set the x-axis configuration.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the y-axis configuration.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the bin width. If `None`, it is auto-computed (~15 bins).
    pub fn bin_width(mut self, width: f64) -> Self {
        self.bin_width = Some(width);
        self
    }

    /// Set the marker shape for dots.
    pub fn marker(mut self, marker: MarkerShape) -> Self {
        self.marker = marker;
        self
    }

    /// Set the visual theme.
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

    /// Compute the global data range across all datasets.
    fn global_range(&self) -> Option<(f64, f64)> {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for ds in &self.datasets {
            for &v in &ds.data {
                if v.is_finite() {
                    min = min.min(v);
                    max = max.max(v);
                }
            }
        }
        if min > max {
            return None;
        }
        if (max - min).abs() < f64::EPSILON {
            Some((min - 1.0, max + 1.0))
        } else {
            Some((min, max))
        }
    }

    /// Resolve the bin width: use explicit value or auto-compute.
    fn resolve_bin_width(&self, lo: f64, hi: f64) -> f64 {
        if let Some(bw) = self.bin_width
            && bw > 0.0
        {
            return bw;
        }
        // Auto: aim for roughly 15 bins
        let range = hi - lo;
        if range <= 0.0 {
            return 1.0;
        }
        range / 15.0
    }

    /// Bin a dataset and return (bin_center, count) pairs.
    fn bin_data(&self, data: &[f64], lo: f64, bin_width: f64, n_bins: usize) -> Vec<(f64, usize)> {
        let mut counts = vec![0usize; n_bins];
        for &v in data {
            if !v.is_finite() {
                continue;
            }
            let idx = ((v - lo) / bin_width).floor() as isize;
            if idx >= 0 && (idx as usize) < n_bins {
                counts[idx as usize] += 1;
            }
        }

        counts
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                let center = lo + (i as f64 + 0.5) * bin_width;
                (center, c)
            })
            .collect()
    }
}

impl Default for DotPlot {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for &DotPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.datasets.is_empty() {
            return;
        }

        // Compute global data range
        let Some((data_lo, data_hi)) = self.global_range() else {
            return;
        };

        let bin_width = self.resolve_bin_width(data_lo, data_hi);
        let n_bins = ((data_hi - data_lo) / bin_width).ceil() as usize;
        let n_bins = n_bins.max(1);

        // Bin all datasets and find the maximum stack height
        let mut all_binned: Vec<Vec<(f64, usize)>> = Vec::new();
        let mut max_count: usize = 0;
        for ds in &self.datasets {
            let binned = self.bin_data(&ds.data, data_lo, bin_width, n_bins);
            for &(_, c) in &binned {
                max_count = max_count.max(c);
            }
            all_binned.push(binned);
        }

        if max_count == 0 {
            return;
        }

        // Compute axis bounds
        let x_lo = data_lo;
        let x_hi = data_lo + n_bins as f64 * bin_width;
        let y_lo = 0.0;
        let y_hi = (max_count as f64) * 1.1 + 0.5;

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_lo, x_hi);
        let y_axis_fixed = self
            .y_axis
            .clone()
            .bounds(crate::axis::Bounds::Manual(y_lo, y_hi));

        let mut pb = create_backend(area);

        // Create and render the plot frame
        let frame = PlotFrame::new(&self.x_axis, &y_axis_fixed, &self.theme)
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

        // Assign colors to datasets
        let mut cycle = ColorCycle::default();
        let colors: Vec<Color> = self
            .datasets
            .iter()
            .map(|ds| ds.color.unwrap_or_else(|| cycle.next_color()))
            .collect();

        // Draw dots for each dataset
        let marker_ch = self.marker.char();
        for (ds_i, binned) in all_binned.iter().enumerate() {
            let color = colors[ds_i];
            for &(center, count) in binned {
                if count == 0 {
                    continue;
                }
                let sx = pa.screen_x(center);
                let xi = sx.round() as u16;

                // Stack dots from y=1 up to y=count
                for dot_idx in 0..count {
                    let y_val = dot_idx as f64 + 1.0;
                    let sy = pa.screen_y(y_val);
                    let yi = sy.round() as u16;

                    if pa.contains(xi, yi) {
                        pb.set_char(xi, yi, marker_ch, color, Z_MARKER);
                    }
                }
            }
        }

        // Composite to buffer
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Draw legend
        if self.show_legend && self.datasets.len() > 1 {
            let entries: Vec<LegendEntry> = self
                .datasets
                .iter()
                .zip(colors.iter())
                .map(|(ds, &c)| LegendEntry {
                    name: ds.name.clone(),
                    color: c,
                    marker: Some(marker_ch),
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
