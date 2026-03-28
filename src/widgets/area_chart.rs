//! Area chart widget (matplotlib's stackplot / fill_between equivalent).
//!
//! Renders series as filled areas with multiple modes: plain (individual
//! fills), stacked (cumulative), normalized (100% stacked), and streamgraph
//! (symmetric baseline). Uses half-block fill characters for visual distinction.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::plot_buffer::{PlotBackend, Z_FILL, create_backend};
use crate::series::Series;
use crate::spines::Spines;
use crate::theme::Theme;
use crate::transform::data_to_screen;

/// Fill density characters from lightest to heaviest, pulled from theme.
fn fill_chars(theme: &Theme) -> [char; 4] {
    let f = &theme.chars.fill;
    [f.light, f.medium, f.dense, f.solid]
}

/// How areas are combined when multiple series are present.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AreaMode {
    /// Each series is filled independently from its line down to the baseline.
    /// Overlapping regions use painter's algorithm (later series on top).
    Plain,
    /// Series are cumulatively stacked on top of each other (default).
    /// Each series' area starts where the previous one ends.
    #[default]
    Stacked,
    /// Like Stacked, but normalized so the total always fills to 100%.
    Normalized,
    /// Symmetric baseline (streamgraph / ThemeRiver). The stack is centered
    /// around zero with a wiggle-minimizing baseline.
    StreamGraph,
}

/// An area chart widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::area_chart::{AreaChart, AreaMode};
/// use ratatui_plt::series::Series;
/// use ratatui::style::Color;
///
/// let chart = AreaChart::new()
///     .series(Series::new("A").data(vec![(0.0, 1.0), (1.0, 2.0), (2.0, 1.5)]).color(Color::Cyan))
///     .series(Series::new("B").data(vec![(0.0, 2.0), (1.0, 1.0), (2.0, 2.5)]).color(Color::Yellow))
///     .mode(AreaMode::Stacked)
///     .title("Areas");
/// ```
pub struct AreaChart {
    series: Vec<Series>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    mode: AreaMode,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
    show_legend: bool,
    legend_position: LegendPosition,
}

impl Default for AreaChart {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            mode: AreaMode::default(),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
            show_legend: true,
            legend_position: LegendPosition::TopRight,
        }
    }
}

impl AreaChart {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn series(mut self, s: Series) -> Self {
        self.series.push(s);
        self
    }

    /// Set the area mode (Plain, Stacked, Normalized, StreamGraph).
    pub fn mode(mut self, mode: AreaMode) -> Self {
        self.mode = mode;
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

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn theme(mut self, t: Theme) -> Self {
        self.theme = t;
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


impl Widget for &AreaChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.series.is_empty() {
            return;
        }

        // Filter NaN from each series
        let filtered: Vec<Series> = self.series.iter().map(|s| s.filter_nan()).collect();

        // Collect all unique x positions across all series, sorted
        let mut all_x: Vec<f64> = Vec::new();
        for s in &filtered {
            for &(x, _) in &s.data {
                all_x.push(x);
            }
        }
        all_x.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        all_x.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

        if all_x.is_empty() {
            return;
        }

        let n_x = all_x.len();
        let n_series = filtered.len();

        // For each x position, interpolate each series' y value
        let mut y_values: Vec<Vec<f64>> = vec![vec![0.0; n_x]; n_series];
        for (si, s) in filtered.iter().enumerate() {
            for (xi, &xv) in all_x.iter().enumerate() {
                y_values[si][xi] = interpolate_y(&s.data, xv);
            }
        }

        // Compute upper and lower boundaries per series based on mode
        let (upper, lower, y_lo_data, y_hi_data) = match self.mode {
            AreaMode::Plain => compute_plain(&y_values, n_x, n_series),
            AreaMode::Stacked => compute_stacked(&y_values, n_x, n_series),
            AreaMode::Normalized => compute_normalized(&y_values, n_x, n_series),
            AreaMode::StreamGraph => compute_streamgraph(&y_values, n_x, n_series),
        };

        // Determine axis bounds
        let x_lo_data = all_x[0];
        let x_hi_data = *all_x.last().unwrap_or(&0.0);
        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_lo_data, x_hi_data);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_lo_data, y_hi_data);

        let mut pb = create_backend(area);

        // Create and render the plot frame
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render_to_pb(
            &mut pb,
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

        // Render filled areas from top series to bottom (painter's algorithm)
        for col_offset in 0..pa.width {
            let screen_x = pa.x + col_offset;
            if screen_x >= area.x + area.width {
                break;
            }

            // Map screen column to data x
            let data_x = x_lo + (col_offset as f64 / (pa.width - 1).max(1) as f64) * (x_hi - x_lo);

            // Interpolate upper/lower at this x for each series
            let mut upper_at_x: Vec<f64> = Vec::with_capacity(n_series);
            let mut lower_at_x: Vec<f64> = Vec::with_capacity(n_series);
            for si in 0..n_series {
                upper_at_x.push(interpolate_from_arrays(&all_x, &upper[si], data_x));
                lower_at_x.push(interpolate_from_arrays(&all_x, &lower[si], data_x));
            }

            // For each series (drawn back to front), fill between lower and upper boundary
            for si in (0..n_series).rev() {
                let sy_upper = data_to_screen(
                    upper_at_x[si],
                    y_lo,
                    y_hi,
                    (pa.y + pa.height - 1) as f64,
                    pa.y as f64,
                )
                .round() as u16;
                let sy_lower = data_to_screen(
                    lower_at_x[si],
                    y_lo,
                    y_hi,
                    (pa.y + pa.height - 1) as f64,
                    pa.y as f64,
                )
                .round() as u16;

                let color = filtered[si].color.unwrap_or(self.theme.color_cycle.at(si));

                let y_top = sy_upper.max(pa.y);
                let y_bot = sy_lower.min(pa.y + pa.height - 1);

                for y in y_top..=y_bot {
                    if pa.contains(screen_x, y) {
                        pb.set_bg(screen_x, y, color, Z_FILL);
                    }
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite before legend
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Draw legend
        if self.show_legend && !filtered.is_empty() {
            let entries: Vec<LegendEntry> = filtered
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let color = s.color.unwrap_or(self.theme.color_cycle.at(i));
                    let fc = fill_chars(&self.theme);
                    let fill_ch = fc[i % fc.len()];
                    LegendEntry {
                        name: s.name.clone(),
                        color,
                        marker: Some(fill_ch),
                    }
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

/// Plain mode: each series filled independently from baseline (y=0).
fn compute_plain(
    y_values: &[Vec<f64>],
    n_x: usize,
    n_series: usize,
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>, f64, f64) {
    let mut upper = vec![vec![0.0; n_x]; n_series];
    let mut lower = vec![vec![0.0; n_x]; n_series];
    let mut y_max = 0.0f64;
    for si in 0..n_series {
        for xi in 0..n_x {
            let val = y_values[si][xi].max(0.0);
            upper[si][xi] = val;
            lower[si][xi] = 0.0;
            y_max = y_max.max(val);
        }
    }

    (upper, lower, 0.0, y_max)
}

/// Stacked mode: cumulative stacking.
fn compute_stacked(
    y_values: &[Vec<f64>],
    n_x: usize,
    n_series: usize,
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>, f64, f64) {
    let mut upper = vec![vec![0.0; n_x]; n_series];
    let mut lower = vec![vec![0.0; n_x]; n_series];

    for xi in 0..n_x {
        let mut running = 0.0;
        for si in 0..n_series {
            lower[si][xi] = running;
            running += y_values[si][xi].max(0.0);
            upper[si][xi] = running;
        }
    }

    let y_max = upper
        .last()
        .map(|row| row.iter().cloned().fold(0.0f64, f64::max))
        .unwrap_or(1.0);

    (upper, lower, 0.0, y_max)
}

/// Normalized mode: stacked to 100%.
fn compute_normalized(
    y_values: &[Vec<f64>],
    n_x: usize,
    n_series: usize,
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>, f64, f64) {
    let mut upper = vec![vec![0.0; n_x]; n_series];
    let mut lower = vec![vec![0.0; n_x]; n_series];

    for xi in 0..n_x {
        let total: f64 = (0..n_series).map(|si| y_values[si][xi].max(0.0)).sum();
        if total <= 0.0 {
            continue;
        }
        let mut running = 0.0;
        for si in 0..n_series {
            lower[si][xi] = running;
            running += y_values[si][xi].max(0.0) / total;
            upper[si][xi] = running;
        }
    }

    (upper, lower, 0.0, 1.0)
}

/// Streamgraph mode: wiggle-minimizing baseline (Byron & Wattenberg 2008).
///
/// Implements the weighted-wiggle algorithm from "Stacked Graphs --
/// Geometry & Aesthetics" (Byron & Wattenberg, 2008), equivalent to
/// D3's `stackOffsetWiggle`. The baseline offset at each x column is:
///
/// ```text
/// offset = sum_i( (n - i) * thickness_i ) / -(n + 1)
/// ```
///
/// where n is the number of layers and thickness_i is the value of
/// layer i at that column. This minimises the sum of squared wiggle
/// across all layers, producing a smoother streamgraph.
fn compute_streamgraph(
    y_values: &[Vec<f64>],
    n_x: usize,
    n_series: usize,
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>, f64, f64) {
    let mut upper = vec![vec![0.0; n_x]; n_series];
    let mut lower = vec![vec![0.0; n_x]; n_series];
    let mut y_min = 0.0f64;
    let mut y_max = 0.0f64;

    let n = n_series as f64;

    for xi in 0..n_x {
        // Byron & Wattenberg weighted-wiggle offset.
        let mut weighted_sum = 0.0;
        for (si, row) in y_values.iter().enumerate().take(n_series) {
            let thickness = row[xi].max(0.0);
            weighted_sum += (n - si as f64) * thickness;
        }
        let offset = weighted_sum / -(n + 1.0);

        let mut running = offset;
        for si in 0..n_series {
            lower[si][xi] = running;
            running += y_values[si][xi].max(0.0);
            upper[si][xi] = running;
        }
        y_min = y_min.min(offset);
        y_max = y_max.max(running);
    }

    (upper, lower, y_min, y_max)
}

/// Linearly interpolate y at a given x from sorted (x, y) data points.
/// If x is outside the data range, clamp to the nearest endpoint value.
fn interpolate_y(data: &[(f64, f64)], x: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    if data.len() == 1 {
        return data[0].1;
    }
    if x <= data[0].0 {
        return data[0].1;
    }
    if x >= data[data.len() - 1].0 {
        return data[data.len() - 1].1;
    }
    for i in 0..data.len() - 1 {
        let (x0, y0) = data[i];
        let (x1, y1) = data[i + 1];
        if x >= x0 && x <= x1 {
            if (x1 - x0).abs() < 1e-15 {
                return y0;
            }
            let t = (x - x0) / (x1 - x0);
            return y0 + t * (y1 - y0);
        }
    }
    data[data.len() - 1].1
}

/// Linearly interpolate from parallel x and y arrays at a given x value.
fn interpolate_from_arrays(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    if xs.is_empty() || ys.is_empty() {
        return 0.0;
    }
    let n = xs.len().min(ys.len());
    if n == 1 {
        return ys[0];
    }
    if x <= xs[0] {
        return ys[0];
    }
    if x >= xs[n - 1] {
        return ys[n - 1];
    }
    for i in 0..n - 1 {
        if x >= xs[i] && x <= xs[i + 1] {
            let dx = xs[i + 1] - xs[i];
            if dx.abs() < 1e-15 {
                return ys[i];
            }
            let t = (x - xs[i]) / dx;
            return ys[i] + t * (ys[i + 1] - ys[i]);
        }
    }
    ys[n - 1]
}
