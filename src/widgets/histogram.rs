//! Histogram widget for distribution visualization.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::plot_buffer::{PlotBuffer, Z_DATA, Z_FILL};
use crate::spines::Spines;
use crate::theme::Theme;

/// Histogram normalization mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistNorm {
    /// Raw counts.
    Count,
    /// Probability density (area sums to 1).
    Density,
    /// Probability (bar heights sum to 1).
    Probability,
}

/// Multi-histogram display mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistMode {
    /// Single dataset (default, legacy behavior).
    Single,
    /// Stacked bars (datasets stacked on top of each other).
    Stacked,
    /// Layered/overlaid bars (drawn with transparency/overlap).
    Layered,
    /// Side-by-side bars within each bin.
    SideBySide,
}

/// Histogram type (bar shape).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistType {
    /// Filled rectangular bars (default).
    Bar,
    /// Unfilled step outline.
    Step,
    /// Filled step outline.
    StepFilled,
}

/// Binning algorithm selection.
#[derive(Clone, Debug, PartialEq)]
pub enum BinMethod {
    /// Fixed number of bins.
    Fixed(usize),
    /// Freedman-Diaconis rule: bin_width = 2 * IQR * n^(-1/3).
    Fd,
    /// Scott's rule: bin_width = 3.5 * std * n^(-1/3).
    Scott,
    /// Sturges' rule: bins = ceil(log2(n)) + 1.
    Sturges,
    /// Square root rule: bins = ceil(sqrt(n)).
    Sqrt,
    /// Auto: pick the method that gives the most bins (between FD and Sturges).
    Auto,
}

/// A single histogram dataset for multi-histogram support.
#[derive(Clone, Debug)]
pub struct HistDataset {
    /// Raw data values.
    pub data: Vec<f64>,
    /// Bar color.
    pub color: Color,
    /// Dataset name (for legend).
    pub name: String,
}

impl HistDataset {
    /// Create a new histogram dataset.
    pub fn new(name: impl Into<String>, data: Vec<f64>, color: Color) -> Self {
        Self {
            data,
            color,
            name: name.into(),
        }
    }
}

/// A histogram widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::histogram::Histogram;
///
/// let hist = Histogram::new(vec![1.0, 1.5, 2.0, 2.5, 3.0, 3.5])
///     .bins(10)
///     .color(ratatui::style::Color::Cyan)
///     .title("Distribution");
/// ```
pub struct Histogram {
    data: Vec<f64>,
    bins: usize,
    range: Option<(f64, f64)>,
    norm_mode: HistNorm,
    color: Option<Color>,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    cumulative: bool,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
    // Multi-dataset support
    datasets: Vec<HistDataset>,
    hist_mode: HistMode,
    histtype: HistType,
    bin_method: Option<BinMethod>,
    rwidth: f64,
    show_legend: bool,
    legend_position: LegendPosition,
}

impl Histogram {
    /// Create a histogram from raw data.
    pub fn new(data: Vec<f64>) -> Self {
        Self {
            data,
            bins: 20,
            range: None,
            norm_mode: HistNorm::Count,
            color: None,
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            cumulative: false,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
            datasets: Vec::new(),
            hist_mode: HistMode::Single,
            histtype: HistType::Bar,
            bin_method: None,
            rwidth: 1.0,
            show_legend: true,
            legend_position: LegendPosition::TopRight,
        }
    }

    /// Set the number of bins.
    pub fn bins(mut self, n: usize) -> Self {
        self.bins = n.max(1);
        self
    }

    /// Set the data range.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.range = Some((min, max));
        self
    }

    /// Set the normalization mode.
    pub fn norm_mode(mut self, mode: HistNorm) -> Self {
        self.norm_mode = mode;
        self
    }

    /// Set the bar color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
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

    /// Enable cumulative histogram.
    pub fn cumulative(mut self, c: bool) -> Self {
        self.cumulative = c;
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

    /// Add a dataset for multi-histogram display.
    pub fn dataset(mut self, ds: HistDataset) -> Self {
        self.datasets.push(ds);
        self
    }

    /// Set the multi-histogram display mode.
    pub fn hist_mode(mut self, mode: HistMode) -> Self {
        self.hist_mode = mode;
        self
    }

    /// Set the histogram type (bar shape).
    pub fn histtype(mut self, ht: HistType) -> Self {
        self.histtype = ht;
        self
    }

    /// Set the binning algorithm.
    pub fn bin_method(mut self, method: BinMethod) -> Self {
        self.bin_method = Some(method);
        self
    }

    /// Set the relative bar width (0.0 to 1.0, default 0.9).
    pub fn rwidth(mut self, rw: f64) -> Self {
        self.rwidth = rw.clamp(0.01, 1.0);
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

    /// Resolve the number of bins from data, considering bin_method and explicit bins setting.
    fn resolve_bin_count(&self, data: &[f64]) -> usize {
        if let Some(ref method) = self.bin_method {
            compute_bin_count(method, data)
        } else {
            self.bins
        }
    }

    /// Compute bin edges and heights for a single dataset. NaN values are filtered out.
    fn compute_bins_for_data(
        &self,
        data: &[f64],
        n_bins: usize,
        lo: f64,
        hi: f64,
    ) -> (Vec<f64>, Vec<f64>) {
        let bin_width = (hi - lo) / n_bins as f64;
        let edges: Vec<f64> = (0..=n_bins).map(|i| lo + i as f64 * bin_width).collect();
        let mut counts = vec![0.0f64; n_bins];

        for &v in data {
            if !v.is_finite() {
                continue;
            }
            if v >= lo && v <= hi {
                let idx = ((v - lo) / bin_width).floor() as usize;
                let idx = idx.min(n_bins - 1);
                counts[idx] += 1.0;
            }
        }

        if self.cumulative {
            for i in 1..counts.len() {
                counts[i] += counts[i - 1];
            }
        }

        let n_total = data.iter().filter(|v| v.is_finite()).count() as f64;
        let heights = match self.norm_mode {
            HistNorm::Count => counts,
            HistNorm::Density => counts.iter().map(|&c| c / (n_total * bin_width)).collect(),
            HistNorm::Probability => counts.iter().map(|&c| c / n_total).collect(),
        };

        (edges, heights)
    }

    /// Compute bin edges and heights (legacy single-dataset API). NaN values are filtered out.
    fn compute_bins(&self) -> (Vec<f64>, Vec<f64>) {
        let n_bins = self.resolve_bin_count(&self.data);
        let (lo, hi) = self.compute_range_for_data(&self.data);
        self.compute_bins_for_data(&self.data, n_bins, lo, hi)
    }

    /// Compute the data range for a single dataset, respecting the explicit range if set.
    fn compute_range_for_data(&self, data: &[f64]) -> (f64, f64) {
        self.range.unwrap_or_else(|| {
            let min = data
                .iter()
                .filter(|v| v.is_finite())
                .cloned()
                .fold(f64::INFINITY, f64::min);
            let max = data
                .iter()
                .filter(|v| v.is_finite())
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            if min == max {
                (min - 1.0, max + 1.0)
            } else {
                (min, max)
            }
        })
    }

    /// Compute the global data range across all datasets.
    fn compute_global_range(&self) -> (f64, f64) {
        if let Some(r) = self.range {
            return r;
        }

        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        // Include primary data
        if !self.data.is_empty() {
            for &v in &self.data {
                if v.is_finite() {
                    min = min.min(v);
                    max = max.max(v);
                }
            }
        }

        // Include datasets
        for ds in &self.datasets {
            for &v in &ds.data {
                if v.is_finite() {
                    min = min.min(v);
                    max = max.max(v);
                }
            }
        }

        if min == max {
            (min - 1.0, max + 1.0)
        } else {
            (min, max)
        }
    }
}

/// Compute the number of bins using the specified binning method.
fn compute_bin_count(method: &BinMethod, data: &[f64]) -> usize {
    let clean: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
    let n = clean.len();
    if n < 2 {
        return 1;
    }

    match method {
        BinMethod::Fixed(count) => (*count).max(1),
        BinMethod::Fd => fd_bins(&clean),
        BinMethod::Scott => scott_bins(&clean),
        BinMethod::Sturges => sturges_bins(n),
        BinMethod::Sqrt => sqrt_bins(n),
        BinMethod::Auto => {
            // Pick the method giving the most bins between FD and Sturges
            let fd = fd_bins(&clean);
            let sturges = sturges_bins(n);
            fd.max(sturges)
        }
    }
}

/// Freedman-Diaconis rule: bin_width = 2 * IQR * n^(-1/3), bins = ceil(range / bin_width).
fn fd_bins(data: &[f64]) -> usize {
    let n = data.len();
    if n < 2 {
        return 1;
    }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let q1 = percentile_sorted(&sorted, 25.0);
    let q3 = percentile_sorted(&sorted, 75.0);
    let iqr = q3 - q1;

    if iqr <= 0.0 {
        return sturges_bins(n); // fallback
    }

    let bin_width = 2.0 * iqr * (n as f64).powf(-1.0 / 3.0);
    let range = sorted[n - 1] - sorted[0];
    ((range / bin_width).ceil() as usize).max(1)
}

/// Scott's rule: bin_width = 3.5 * std * n^(-1/3).
fn scott_bins(data: &[f64]) -> usize {
    let n = data.len();
    if n < 2 {
        return 1;
    }
    let mean = data.iter().sum::<f64>() / n as f64;
    let variance = data.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = variance.sqrt();

    if std_dev <= 0.0 {
        return 1;
    }

    let bin_width = 3.5 * std_dev * (n as f64).powf(-1.0 / 3.0);
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let range = sorted[n - 1] - sorted[0];
    ((range / bin_width).ceil() as usize).max(1)
}

/// Sturges' rule: bins = ceil(log2(n)) + 1.
fn sturges_bins(n: usize) -> usize {
    ((n as f64).log2().ceil() as usize + 1).max(1)
}

/// Square root rule: bins = ceil(sqrt(n)).
fn sqrt_bins(n: usize) -> usize {
    ((n as f64).sqrt().ceil() as usize).max(1)
}

/// Compute a percentile from pre-sorted data.
fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let k = (p / 100.0) * (sorted.len() - 1) as f64;
    let f = k.floor() as usize;
    let c = f.min(sorted.len() - 1);
    let d = k - f as f64;
    if c + 1 < sorted.len() {
        sorted[c] + d * (sorted[c + 1] - sorted[c])
    } else {
        sorted[c]
    }
}

impl Widget for &Histogram {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Determine if we're in multi-dataset mode
        let has_datasets = !self.datasets.is_empty();

        let mut pb = PlotBuffer::new(area);

        if has_datasets {
            self.render_multi(area, buf, &mut pb);
        } else {
            self.render_single(area, buf, &mut pb);
        }
    }
}

impl Histogram {
    /// Render the single-dataset histogram (legacy path).
    fn render_single(&self, area: Rect, buf: &mut Buffer, pb: &mut PlotBuffer) {
        let (edges, heights) = self.compute_bins();
        if edges.len() < 2 || heights.is_empty() {
            return;
        }

        let x_lo = edges[0];
        let x_hi = *edges.last().unwrap_or(&x_lo);
        let y_max = heights.iter().cloned().fold(0.0f64, f64::max);
        let y_lo = 0.0;
        let y_hi = if y_max == 0.0 { 1.0 } else { y_max * 1.1 };

        // Force y-axis to start at 0 — histogram counts are never negative
        let y_axis_fixed = self.y_axis.clone().bounds(crate::axis::Bounds::Manual(y_lo, y_hi));
        let frame = PlotFrame::new(&self.x_axis, &y_axis_fixed, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render_to_pb(
            pb,
            area,
            DataBounds {
                x_lo,
                x_hi,
                y_lo,
                y_hi,
            },
        ) else {
            return;
        };

        // Draw bars
        let resolved_color = self.color.unwrap_or(self.theme.primary);
        self.draw_bars(
            &pa,
            &edges,
            &heights,
            resolved_color,
            pb,
            BarSlotLayout {
                ds_index: 0,
                n_datasets: 1,
            },
        );

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, pb);

        // Composite
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);
    }

    /// Render multiple datasets according to hist_mode.
    fn render_multi(&self, area: Rect, buf: &mut Buffer, pb: &mut PlotBuffer) {
        let (global_lo, global_hi) = self.compute_global_range();

        // Resolve bin count from all data combined
        let all_data: Vec<f64> = self
            .datasets
            .iter()
            .flat_map(|ds| ds.data.iter().copied())
            .collect();
        let n_bins = self.resolve_bin_count(&all_data);

        // Compute bin heights for each dataset
        let mut all_edges = Vec::new();
        let mut all_heights = Vec::new();
        for ds in &self.datasets {
            let (edges, heights) =
                self.compute_bins_for_data(&ds.data, n_bins, global_lo, global_hi);
            all_edges.push(edges);
            all_heights.push(heights);
        }

        if all_edges.is_empty() || all_edges[0].len() < 2 {
            return;
        }

        let edges = &all_edges[0]; // All share same edges

        // Compute y_max depending on mode
        let y_max = match self.hist_mode {
            HistMode::Single | HistMode::Layered | HistMode::SideBySide => all_heights
                .iter()
                .flat_map(|h| h.iter())
                .cloned()
                .fold(0.0f64, f64::max),
            HistMode::Stacked => {
                let n = all_heights.first().map_or(0, |h| h.len());
                (0..n)
                    .map(|i| {
                        all_heights
                            .iter()
                            .map(|h| h.get(i).copied().unwrap_or(0.0))
                            .sum::<f64>()
                    })
                    .fold(0.0f64, f64::max)
            }
        };

        let x_lo = edges[0];
        let x_hi = *edges.last().unwrap_or(&x_lo);
        let y_lo = 0.0;
        let y_hi = if y_max == 0.0 { 1.0 } else { y_max * 1.1 };

        // Force y-axis to start at 0 — histogram counts are never negative
        let y_axis_fixed = self.y_axis.clone().bounds(crate::axis::Bounds::Manual(y_lo, y_hi));
        let frame = PlotFrame::new(&self.x_axis, &y_axis_fixed, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render_to_pb(
            pb,
            area,
            DataBounds {
                x_lo,
                x_hi,
                y_lo,
                y_hi,
            },
        ) else {
            return;
        };

        let n_ds = self.datasets.len();

        match self.hist_mode {
            HistMode::Single => {
                if !all_heights.is_empty() {
                    self.draw_bars(
                        &pa,
                        edges,
                        &all_heights[0],
                        self.datasets[0].color,
                        pb,
                        BarSlotLayout {
                            ds_index: 0,
                            n_datasets: 1,
                        },
                    );
                }
            }
            HistMode::Stacked => {
                let n = all_heights.first().map_or(0, |h| h.len());
                let mut bottoms = vec![0.0f64; n];

                for (ds_i, ds) in self.datasets.iter().enumerate() {
                    let heights = &all_heights[ds_i];
                    // Use incrementing Z so later datasets don't overwrite
                    // earlier ones at shared boundary cells.
                    let z_level = Z_FILL + ds_i as u8;
                    for i in 0..n {
                        let h = heights.get(i).copied().unwrap_or(0.0);
                        if h == 0.0 {
                            continue;
                        }
                        let bottom = bottoms[i];
                        let top = bottom + h;

                        let bar_left = pa.screen_x(edges[i]);
                        let bar_right = pa.screen_x(edges[i + 1]);
                        let bin_width_px = bar_right - bar_left;
                        let offset = bin_width_px * (1.0 - self.rwidth) / 2.0;

                        let bar_top_y = pa.screen_y(top);
                        let bar_bottom_y = pa.screen_y(bottom);

                        let x_start = (bar_left + offset).floor() as u16;
                        let x_end = (bar_right - offset).ceil() as u16;
                        let y_top = bar_top_y.floor() as u16;
                        let y_bot = bar_bottom_y.ceil() as u16;

                        // Draw stacked bar region directly with per-dataset Z
                        for x in x_start..x_end {
                            for y in y_top..y_bot {
                                if pa.contains(x, y) {
                                    pb.set_bg(x, y, ds.color, z_level);
                                }
                            }
                        }
                        bottoms[i] = top;
                    }
                }
            }
            HistMode::Layered => {
                for (ds_i, ds) in self.datasets.iter().enumerate().rev() {
                    self.draw_bars(
                        &pa,
                        edges,
                        &all_heights[ds_i],
                        ds.color,
                        pb,
                        BarSlotLayout {
                            ds_index: 0,
                            n_datasets: 1,
                        },
                    );
                }
            }
            HistMode::SideBySide => {
                for (ds_i, ds) in self.datasets.iter().enumerate() {
                    self.draw_bars(
                        &pa,
                        edges,
                        &all_heights[ds_i],
                        ds.color,
                        pb,
                        BarSlotLayout {
                            ds_index: ds_i,
                            n_datasets: n_ds,
                        },
                    );
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, pb);

        // Composite before legend
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Draw legend
        if self.show_legend && !self.datasets.is_empty() {
            let entries: Vec<LegendEntry> = self
                .datasets
                .iter()
                .map(|ds| LegendEntry {
                    name: ds.name.clone(),
                    color: ds.color,
                    marker: Some(self.theme.chars.fill.solid),
                })
                .collect();
            let legend = Legend::new(entries)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(pa.x, pa.y, pa.width, pa.height);
            (&legend).render(legend_area, buf);
        }
    }

    /// Draw bars for one dataset within a bin set, supporting side-by-side offset.
    fn draw_bars(
        &self,
        pa: &crate::frame::PlotArea,
        edges: &[f64],
        heights: &[f64],
        color: Color,
        pb: &mut PlotBuffer,
        slot: BarSlotLayout,
    ) {
        for i in 0..heights.len() {
            let bar_left = pa.screen_x(edges[i]);
            let bar_right = pa.screen_x(edges[i + 1]);
            let bin_width_px = bar_right - bar_left;

            // Apply rwidth
            let effective_width = bin_width_px * self.rwidth;
            let rwidth_offset = (bin_width_px - effective_width) / 2.0;

            // For side-by-side, subdivide the effective width
            let sub_width = effective_width / slot.n_datasets as f64;
            let x_start_f = bar_left + rwidth_offset + slot.ds_index as f64 * sub_width;
            let x_end_f = x_start_f + sub_width;

            let bar_top = pa.screen_y(heights[i]);
            let bar_bottom = pa.screen_y(0.0);

            // Use floor for left edge and ceil for right edge so that
            // adjacent bins share the boundary pixel without gaps.
            let rect = BarRect {
                x_start: x_start_f.floor() as u16,
                x_end: x_end_f.ceil() as u16,
                y_top: bar_top.floor() as u16,
                y_bot: bar_bottom.round() as u16,
                #[cfg(feature = "unicode-extended")]
                top_frac: bar_top.fract(),
            };

            match self.histtype {
                HistType::Bar => {
                    self.draw_bar_region(pa, &rect, color, pb);
                }
                HistType::Step => {
                    // Draw only the outline (top edge + sides)
                    for x in rect.x_start..rect.x_end {
                        if pa.contains(x, rect.y_top) {
                            pb.set_char(x, rect.y_top, self.theme.chars.border.horizontal, color, Z_DATA);
                        }
                    }
                    for y in rect.y_top..rect.y_bot {
                        if pa.contains(rect.x_start, y) {
                            pb.set_char(rect.x_start, y, self.theme.chars.border.vertical, color, Z_DATA);
                        }
                    }
                    if rect.x_end > 0 {
                        let rx = rect.x_end.saturating_sub(1);
                        for y in rect.y_top..rect.y_bot {
                            if pa.contains(rx, y) {
                                pb.set_char(rx, y, self.theme.chars.border.vertical, color, Z_DATA);
                            }
                        }
                    }
                }
                HistType::StepFilled => {
                    self.draw_bar_region(pa, &rect, color, pb);
                    for x in rect.x_start..rect.x_end {
                        if pa.contains(x, rect.y_top) {
                            pb.set_char(x, rect.y_top, self.theme.chars.fill.half_upper, color, Z_DATA);
                        }
                    }
                }
            }
        }
    }

    /// Fill a rectangular bar region.
    ///
    /// When the `unicode-extended` feature is enabled, the topmost row of each bar
    /// uses a vertical eighth-block character to represent the fractional fill,
    /// giving 8x vertical sub-cell precision at the bar edge.
    fn draw_bar_region(
        &self,
        pa: &crate::frame::PlotArea,
        rect: &BarRect,
        color: Color,
        pb: &mut PlotBuffer,
    ) {
        #[cfg(feature = "unicode-extended")]
        {
            let fill_fraction = 1.0 - rect.top_frac;
            let fill_char = crate::drawing::vertical_fill_char(fill_fraction);

            for x in rect.x_start..rect.x_end {
                // Draw the fractional top row
                if pa.contains(x, rect.y_top) && fill_char != ' ' {
                    pb.set_char(x, rect.y_top, fill_char, color, Z_DATA);
                }
                // Fill solid rows below the top
                for y in (rect.y_top + 1)..rect.y_bot {
                    if pa.contains(x, y) {
                        pb.set_bg(x, y, color, Z_FILL);
                    }
                }
            }
        }

        #[cfg(not(feature = "unicode-extended"))]
        {
            for x in rect.x_start..rect.x_end {
                for y in rect.y_top..rect.y_bot {
                    if pa.contains(x, y) {
                        pb.set_bg(x, y, color, Z_FILL);
                    }
                }
            }
        }
    }
}

/// Pixel bounds for a histogram bar rectangle.
struct BarRect {
    x_start: u16,
    x_end: u16,
    y_top: u16,
    y_bot: u16,
    /// Fractional part of the top position within its cell (0.0 = cell top, 1.0 = cell bottom).
    /// Used by unicode-extended rendering for sub-cell bar-top precision.
    #[cfg(feature = "unicode-extended")]
    top_frac: f64,
}

/// Layout parameters for side-by-side bar positioning.
struct BarSlotLayout {
    ds_index: usize,
    n_datasets: usize,
}
