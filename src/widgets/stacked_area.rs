//! Stacked area chart widget (matplotlib's stackplot equivalent).
//!
//! Renders multiple series as stacked filled areas, with each series
//! cumulatively added on top of the previous. Uses half-block fill
//! characters for visual distinction between layers.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::series::Series;
use crate::spines::Spines;
use crate::theme::Theme;
use crate::transform::data_to_screen;

/// Default color cycle for stacked series.
const COLOR_CYCLE: &[Color] = &[
    Color::Cyan,
    Color::Yellow,
    Color::Magenta,
    Color::Green,
    Color::Red,
    Color::Blue,
    Color::LightCyan,
    Color::LightYellow,
    Color::LightMagenta,
    Color::LightGreen,
];

/// Fill density characters from lightest to heaviest.
const FILL_CHARS: &[char] = &['░', '▒', '▓', '█'];

/// A stacked area chart widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::stacked_area::StackedArea;
/// use ratatui_plt::series::Series;
/// use ratatui::style::Color;
///
/// let chart = StackedArea::new()
///     .series(Series::new("A").data(vec![(0.0, 1.0), (1.0, 2.0), (2.0, 1.5)]).color(Color::Cyan))
///     .series(Series::new("B").data(vec![(0.0, 2.0), (1.0, 1.0), (2.0, 2.5)]).color(Color::Yellow))
///     .title("Stacked Areas");
/// ```
pub struct StackedArea {
    series: Vec<Series>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
    show_legend: bool,
    legend_position: LegendPosition,
}

impl Default for StackedArea {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
            show_legend: true,
            legend_position: LegendPosition::TopRight,
        }
    }
}

impl StackedArea {
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

impl Widget for &StackedArea {
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

        // Compute cumulative sums: cumulative[si][xi] = sum of y_values[0..=si][xi]
        let mut cumulative: Vec<Vec<f64>> = vec![vec![0.0; n_x]; n_series];
        for xi in 0..n_x {
            let mut running = 0.0;
            for si in 0..n_series {
                running += y_values[si][xi].max(0.0);
                cumulative[si][xi] = running;
            }
        }

        // Determine axis bounds
        let x_lo_data = all_x[0];
        let x_hi_data = *all_x.last().unwrap_or(&0.0);
        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_lo_data, x_hi_data);

        let y_max_data = cumulative
            .last()
            .map(|row| row.iter().cloned().fold(0.0f64, f64::max))
            .unwrap_or(1.0);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(0.0, y_max_data);

        // Create and render the plot frame
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, DataBounds { x_lo, x_hi, y_lo, y_hi }) else {
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

            // Interpolate cumulative values at this x for each series
            let mut cum_at_x: Vec<f64> = Vec::with_capacity(n_series);
            for cum_row in cumulative.iter().take(n_series) {
                let interp = interpolate_from_arrays(&all_x, cum_row, data_x);
                cum_at_x.push(interp);
            }

            // For each series (drawn back to front), fill between lower and upper boundary
            for si in (0..n_series).rev() {
                let upper = cum_at_x[si];
                let lower = if si > 0 { cum_at_x[si - 1] } else { 0.0 };

                let sy_upper = data_to_screen(
                    upper,
                    y_lo,
                    y_hi,
                    (pa.y + pa.height - 1) as f64,
                    pa.y as f64,
                )
                .round() as u16;
                let sy_lower = data_to_screen(
                    lower,
                    y_lo,
                    y_hi,
                    (pa.y + pa.height - 1) as f64,
                    pa.y as f64,
                )
                .round() as u16;

                let color = if filtered[si].color != Color::White {
                    filtered[si].color
                } else {
                    COLOR_CYCLE[si % COLOR_CYCLE.len()]
                };

                // Pick fill character based on series index for visual distinction
                let fill_ch = FILL_CHARS[si % FILL_CHARS.len()];

                let y_top = sy_upper.max(pa.y);
                let y_bot = sy_lower.min(pa.y + pa.height - 1);

                for y in y_top..=y_bot {
                    if pa.contains(screen_x, y) {
                        buf[(screen_x, y)]
                            .set_char(fill_ch)
                            .set_style(Style::default().fg(color));
                    }
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations(&pa, &self.annotations, buf);

        // Draw legend
        if self.show_legend && !filtered.is_empty() {
            let entries: Vec<LegendEntry> = filtered
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let color = if s.color != Color::White {
                        s.color
                    } else {
                        COLOR_CYCLE[i % COLOR_CYCLE.len()]
                    };
                    let fill_ch = FILL_CHARS[i % FILL_CHARS.len()];
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
