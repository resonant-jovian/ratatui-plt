//! Enhanced bar chart widget with grouped and stacked modes.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::chars::PatternChars;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::plot_buffer::{PlotBackend, Z_ANNOTATION, Z_CHROME, Z_DATA, create_backend};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::ticker::NullLocator;
use crate::transform::data_to_screen;

/// Bar chart orientation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

/// Bar chart mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BarMode {
    /// Side-by-side bars for each category.
    Grouped,
    /// Bars stacked on top of each other.
    Stacked,
    /// Diverging stacked: positive values stack upward from zero, negative downward.
    DivergingStacked,
}

/// Pattern fill style for bar chart datasets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FillPattern {
    #[default]
    Solid,
    DiagonalRight,
    DiagonalLeft,
    CrossHatch,
    Horizontal,
    Vertical,
    Dot,
}

/// Resolve a `FillPattern` to its theme character.
///
/// Returns `None` for `Solid` (use default fill), or `Some(char)`
/// for patterned fills.
fn resolve_pattern(chars: &PatternChars, pattern: &FillPattern) -> Option<char> {
    match pattern {
        FillPattern::Solid => None,
        FillPattern::DiagonalRight => Some(chars.diagonal_right),
        FillPattern::DiagonalLeft => Some(chars.diagonal_left),
        FillPattern::CrossHatch => Some(chars.cross_hatch),
        FillPattern::Horizontal => Some(chars.horizontal),
        FillPattern::Vertical => Some(chars.vertical),
        FillPattern::Dot => Some(chars.dot),
    }
}

/// A single bar group dataset.
#[derive(Clone, Debug)]
pub struct BarDataset {
    /// Dataset name (for legend).
    pub name: String,
    /// Values (one per category).
    pub values: Vec<f64>,
    /// Bar color.
    pub color: Color,
    /// Fill pattern (default: Solid).
    pub pattern: FillPattern,
}

impl BarDataset {
    pub fn new(name: impl Into<String>, values: Vec<f64>, color: Color) -> Self {
        Self {
            name: name.into(),
            values,
            color,
            pattern: FillPattern::Solid,
        }
    }

    /// Set the fill pattern for this dataset.
    pub fn pattern(mut self, p: FillPattern) -> Self {
        self.pattern = p;
        self
    }
}

/// An enhanced bar chart widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::bar_chart::{BarChart, BarDataset, BarMode};
/// use ratatui::style::Color;
///
/// let chart = BarChart::new()
///     .categories(vec!["A", "B", "C"])
///     .dataset(BarDataset::new("2024", vec![10.0, 20.0, 15.0], Color::Cyan))
///     .dataset(BarDataset::new("2025", vec![12.0, 18.0, 22.0], Color::Yellow))
///     .mode(BarMode::Grouped)
///     .title("Comparison");
/// ```
pub struct BarChart {
    categories: Vec<String>,
    datasets: Vec<BarDataset>,
    mode: BarMode,
    orientation: Orientation,
    title: Option<String>,
    bar_gap: u16,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
    show_legend: bool,
    legend_position: LegendPosition,
    show_values: bool,
}

impl Default for BarChart {
    fn default() -> Self {
        Self {
            categories: Vec::new(),
            datasets: Vec::new(),
            mode: BarMode::Grouped,
            orientation: Orientation::Vertical,
            title: None,
            bar_gap: 1,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
            show_legend: true,
            legend_position: LegendPosition::TopRight,
            show_values: false,
        }
    }
}

impl BarChart {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn categories(mut self, cats: Vec<impl Into<String>>) -> Self {
        self.categories = cats.into_iter().map(Into::into).collect();
        self
    }

    pub fn dataset(mut self, ds: BarDataset) -> Self {
        self.datasets.push(ds);
        self
    }

    pub fn mode(mut self, mode: BarMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn orientation(mut self, o: Orientation) -> Self {
        self.orientation = o;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn bar_gap(mut self, gap: u16) -> Self {
        self.bar_gap = gap;
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

    /// Show or hide numeric value labels on each bar.
    ///
    /// When enabled, the value of each bar is rendered as text above the bar
    /// (vertical orientation) or to the right (horizontal orientation).
    /// Values are formatted with one decimal place.
    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }
}

/// Parameters for drawing a value label on a bar.
struct ValueLabelParams {
    /// The numeric value to display.
    val: f64,
    /// Bar left edge (screen x).
    bar_x: u16,
    /// Bar width in screen columns.
    bar_width: u16,
    /// Screen y of the bar top (for vertical) or the bar start (for horizontal).
    top_y: u16,
    /// Height span of the bar in screen rows (used for horizontal centering).
    bar_height_range: u16,
}

/// Helper: render a value label into the PlotBuffer at `Z_ANNOTATION` level.
///
/// For vertical orientation, the label is placed 1 row above `top_y`, centered
/// on the bar span `[bar_x, bar_x + bar_width)`.
/// For horizontal orientation, the label is placed 1 column to the right of
/// the bar end, vertically centered on the bar span.
fn draw_value_label(
    pb: &mut dyn PlotBackend,
    orientation: &Orientation,
    p: &ValueLabelParams,
    fg: Color,
    area: Rect,
) {
    let label = format!("{:.1}", p.val);
    match orientation {
        Orientation::Vertical => {
            // Place 1 row above bar top
            let label_y = p.top_y.saturating_sub(1);
            let center_x = p.bar_x + p.bar_width / 2;
            let start_x = center_x.saturating_sub(label.len() as u16 / 2);
            for (j, ch) in label.chars().enumerate() {
                let lx = start_x + j as u16;
                if lx >= area.x
                    && lx < area.x + area.width
                    && label_y >= area.y
                    && label_y < area.y + area.height
                {
                    pb.set_char(lx, label_y, ch, fg, Z_ANNOTATION);
                }
            }
        }
        Orientation::Horizontal => {
            // Place 1 column to the right of the bar end
            let label_x = p.bar_x + p.bar_width + 1;
            let center_y = p.top_y + p.bar_height_range / 2;
            for (j, ch) in label.chars().enumerate() {
                let lx = label_x + j as u16;
                if lx >= area.x
                    && lx < area.x + area.width
                    && center_y >= area.y
                    && center_y < area.y + area.height
                {
                    pb.set_char(lx, center_y, ch, fg, Z_ANNOTATION);
                }
            }
        }
    }
}

impl Widget for &BarChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4
            || area.height < 4
            || self.categories.is_empty()
            || self.datasets.is_empty()
        {
            return;
        }

        let n_cats = self.categories.len();

        // Compute y-axis bounds depending on mode
        let (y_lo, y_hi) = match self.mode {
            BarMode::Grouped => {
                let max_val = self
                    .datasets
                    .iter()
                    .flat_map(|d| d.values.iter())
                    .cloned()
                    .fold(0.0f64, f64::max);
                let hi = if max_val == 0.0 { 1.0 } else { max_val * 1.1 };
                (0.0, hi)
            }
            BarMode::Stacked => {
                let max_val = (0..n_cats)
                    .map(|i| {
                        self.datasets
                            .iter()
                            .map(|d| d.values.get(i).copied().unwrap_or(0.0))
                            .sum::<f64>()
                    })
                    .fold(0.0f64, f64::max);
                let hi = if max_val == 0.0 { 1.0 } else { max_val * 1.1 };
                (0.0, hi)
            }
            BarMode::DivergingStacked => {
                // Compute positive and negative stack totals per category
                let mut max_pos = 0.0f64;
                let mut min_neg = 0.0f64;
                for i in 0..n_cats {
                    let mut pos_sum = 0.0f64;
                    let mut neg_sum = 0.0f64;
                    for ds in &self.datasets {
                        let v = ds.values.get(i).copied().unwrap_or(0.0);
                        if v >= 0.0 {
                            pos_sum += v;
                        } else {
                            neg_sum += v;
                        }
                    }
                    if pos_sum > max_pos {
                        max_pos = pos_sum;
                    }
                    if neg_sum < min_neg {
                        min_neg = neg_sum;
                    }
                }
                // Symmetric range with 10% headroom
                let extent = max_pos.abs().max(min_neg.abs());
                let extent = if extent == 0.0 { 1.0 } else { extent * 1.1 };
                (-extent, extent)
            }
        };

        // Use NullLocator for x-axis to suppress x tick labels (categories drawn manually)
        let x_axis = Axis::new().locator(NullLocator);
        let y_axis = Axis::new();
        let x_lo = 0.0;
        let x_hi = n_cats as f64;

        let mut pb = create_backend(area);

        // Build reference lines: include user-supplied ones, plus a zero line for diverging mode
        let mut ref_lines = self.reference_lines.clone();
        if self.mode == BarMode::DivergingStacked {
            ref_lines.push(ReferenceLine::hline(0.0, self.theme.axis_color));
        }

        // Create and render the plot frame
        let frame = PlotFrame::new(&x_axis, &y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&ref_lines);

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

        // Draw bars
        let n_ds = self.datasets.len();
        let group_width = pa.width / n_cats as u16;
        let screen_bottom = (pa.y + pa.height - 1) as f64;
        let screen_top = pa.y as f64;

        for (cat_i, _cat) in self.categories.iter().enumerate() {
            let group_x = pa.x + cat_i as u16 * group_width;

            match self.mode {
                BarMode::Grouped => {
                    let bar_width = (group_width.saturating_sub(self.bar_gap * 2)) / n_ds as u16;
                    for (ds_i, ds) in self.datasets.iter().enumerate() {
                        let val = ds.values.get(cat_i).copied().unwrap_or(0.0);
                        let bar_x = group_x + self.bar_gap + ds_i as u16 * bar_width;
                        let bar_top = data_to_screen(val, y_lo, y_hi, screen_bottom, screen_top)
                            .round() as u16;

                        let effective_width = bar_width.max(1);
                        let pat_ch = resolve_pattern(&self.theme.chars.pattern, &ds.pattern);
                        for x in bar_x..bar_x + effective_width {
                            for y in bar_top..pa.y + pa.height {
                                if pa.contains(x, y) {
                                    if let Some(ch) = pat_ch {
                                        pb.set_cell(x, y, ch, ds.color, Color::Reset, Z_DATA);
                                    } else {
                                        pb.set_cell(x, y, ' ', ds.color, ds.color, Z_DATA);
                                    }
                                }
                            }
                        }

                        // Value labels
                        if self.show_values {
                            draw_value_label(
                                &mut pb,
                                &self.orientation,
                                &ValueLabelParams {
                                    val,
                                    bar_x,
                                    bar_width: effective_width,
                                    top_y: bar_top,
                                    bar_height_range: (pa.y + pa.height).saturating_sub(bar_top),
                                },
                                self.theme.foreground,
                                area,
                            );
                        }
                    }
                }
                BarMode::Stacked => {
                    let bar_x = group_x + self.bar_gap;
                    let bar_width = group_width.saturating_sub(self.bar_gap * 2).max(1);
                    let mut bottom = 0.0f64;
                    let mut last_top_y = pa.y + pa.height;
                    let mut stack_total = 0.0f64;

                    for ds in &self.datasets {
                        let val = ds.values.get(cat_i).copied().unwrap_or(0.0);
                        let y_bot = data_to_screen(bottom, y_lo, y_hi, screen_bottom, screen_top)
                            .round() as u16;
                        let y_top =
                            data_to_screen(bottom + val, y_lo, y_hi, screen_bottom, screen_top)
                                .round() as u16;

                        let pat_ch = resolve_pattern(&self.theme.chars.pattern, &ds.pattern);
                        for x in bar_x..bar_x + bar_width {
                            for y in y_top..y_bot {
                                if pa.contains(x, y) {
                                    if let Some(ch) = pat_ch {
                                        pb.set_cell(x, y, ch, ds.color, Color::Reset, Z_DATA);
                                    } else {
                                        pb.set_cell(x, y, ' ', ds.color, ds.color, Z_DATA);
                                    }
                                }
                            }
                        }

                        bottom += val;
                        stack_total += val;
                        last_top_y = y_top;
                    }

                    // Value labels (show the total stack value above the topmost bar)
                    if self.show_values {
                        draw_value_label(
                            &mut pb,
                            &self.orientation,
                            &ValueLabelParams {
                                val: stack_total,
                                bar_x,
                                bar_width,
                                top_y: last_top_y,
                                bar_height_range: (pa.y + pa.height).saturating_sub(last_top_y),
                            },
                            self.theme.foreground,
                            area,
                        );
                    }
                }
                BarMode::DivergingStacked => {
                    let bar_x = group_x + self.bar_gap;
                    let bar_width = group_width.saturating_sub(self.bar_gap * 2).max(1);
                    let mut pos_bottom = 0.0f64;
                    let mut neg_top = 0.0f64;

                    for ds in &self.datasets {
                        let val = ds.values.get(cat_i).copied().unwrap_or(0.0);
                        let pat_ch = resolve_pattern(&self.theme.chars.pattern, &ds.pattern);
                        if val >= 0.0 {
                            // Stack upward from zero
                            let seg_bot =
                                data_to_screen(pos_bottom, y_lo, y_hi, screen_bottom, screen_top)
                                    .round() as u16;
                            let seg_top = data_to_screen(
                                pos_bottom + val,
                                y_lo,
                                y_hi,
                                screen_bottom,
                                screen_top,
                            )
                            .round() as u16;

                            for x in bar_x..bar_x + bar_width {
                                for y in seg_top..seg_bot {
                                    if pa.contains(x, y) {
                                        if let Some(ch) = pat_ch {
                                            pb.set_cell(x, y, ch, ds.color, Color::Reset, Z_DATA);
                                        } else {
                                            pb.set_cell(x, y, ' ', ds.color, ds.color, Z_DATA);
                                        }
                                    }
                                }
                            }

                            pos_bottom += val;
                        } else {
                            // Stack downward from zero
                            let seg_top = data_to_screen(
                                neg_top + val,
                                y_lo,
                                y_hi,
                                screen_bottom,
                                screen_top,
                            )
                            .round() as u16;
                            let seg_bot =
                                data_to_screen(neg_top, y_lo, y_hi, screen_bottom, screen_top)
                                    .round() as u16;

                            for x in bar_x..bar_x + bar_width {
                                for y in seg_top..seg_bot {
                                    if pa.contains(x, y) {
                                        if let Some(ch) = pat_ch {
                                            pb.set_cell(x, y, ch, ds.color, Color::Reset, Z_DATA);
                                        } else {
                                            pb.set_cell(x, y, ' ', ds.color, ds.color, Z_DATA);
                                        }
                                    }
                                }
                            }

                            neg_top += val;
                        }
                    }

                    // Value labels for diverging: show positive total above, negative below
                    if self.show_values {
                        if pos_bottom > 0.0 {
                            let top_screen =
                                data_to_screen(pos_bottom, y_lo, y_hi, screen_bottom, screen_top)
                                    .round() as u16;
                            draw_value_label(
                                &mut pb,
                                &self.orientation,
                                &ValueLabelParams {
                                    val: pos_bottom,
                                    bar_x,
                                    bar_width,
                                    top_y: top_screen,
                                    bar_height_range: 0,
                                },
                                self.theme.foreground,
                                area,
                            );
                        }
                        if neg_top < 0.0 {
                            let bot_screen =
                                data_to_screen(neg_top, y_lo, y_hi, screen_bottom, screen_top)
                                    .round() as u16;
                            // Place below the negative stack: 1 row below bottom
                            let label = format!("{:.1}", neg_top);
                            let center_x = bar_x + bar_width / 2;
                            let start_x = center_x.saturating_sub(label.len() as u16 / 2);
                            let label_y = bot_screen + 1;
                            for (j, ch) in label.chars().enumerate() {
                                let lx = start_x + j as u16;
                                if lx >= area.x
                                    && lx < area.x + area.width
                                    && label_y >= area.y
                                    && label_y < area.y + area.height
                                {
                                    pb.set_char(
                                        lx,
                                        label_y,
                                        ch,
                                        self.theme.foreground,
                                        Z_ANNOTATION,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Draw category label
            let cat = &self.categories[cat_i];
            let label_x = group_x + group_width / 2;
            let label_start = label_x.saturating_sub(cat.len() as u16 / 2);
            let label_y = pa.y + pa.height;
            if label_y < area.y + area.height {
                for (j, ch) in cat.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        pb.set_char(lx, label_y, ch, self.theme.axis_color, Z_CHROME);
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
        if self.show_legend && !self.datasets.is_empty() {
            let entries: Vec<LegendEntry> = self
                .datasets
                .iter()
                .map(|ds| {
                    let marker = resolve_pattern(&self.theme.chars.pattern, &ds.pattern)
                        .unwrap_or(self.theme.chars.fill.solid);
                    LegendEntry {
                        name: ds.name.clone(),
                        color: ds.color,
                        marker: Some(marker),
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
