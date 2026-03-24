//! Enhanced bar chart widget with grouped and stacked modes.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA};
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
}

impl BarDataset {
    pub fn new(name: impl Into<String>, values: Vec<f64>, color: Color) -> Self {
        Self {
            name: name.into(),
            values,
            color,
        }
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

        // Compute max value
        let max_val = match self.mode {
            BarMode::Grouped => self
                .datasets
                .iter()
                .flat_map(|d| d.values.iter())
                .cloned()
                .fold(0.0f64, f64::max),
            BarMode::Stacked => {
                let n = self.categories.len();
                (0..n)
                    .map(|i| {
                        self.datasets
                            .iter()
                            .map(|d| d.values.get(i).copied().unwrap_or(0.0))
                            .sum::<f64>()
                    })
                    .fold(0.0f64, f64::max)
            }
        };
        let y_hi = if max_val == 0.0 { 1.0 } else { max_val * 1.1 };

        // Use NullLocator for x-axis to suppress x tick labels (categories drawn manually)
        let x_axis = Axis::new().locator(NullLocator);
        let y_axis = Axis::new();
        let n_cats = self.categories.len();
        let x_lo = 0.0;
        let x_hi = n_cats as f64;

        let mut pb = PlotBuffer::new(area);

        // Create and render the plot frame
        let frame = PlotFrame::new(&x_axis, &y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render_to_pb(
            &mut pb,
            area,
            DataBounds {
                x_lo,
                x_hi,
                y_lo: 0.0,
                y_hi,
            },
        ) else {
            return;
        };

        // Draw bars
        let n_ds = self.datasets.len();
        let group_width = pa.width / n_cats as u16;

        for (cat_i, _cat) in self.categories.iter().enumerate() {
            let group_x = pa.x + cat_i as u16 * group_width;

            match self.mode {
                BarMode::Grouped => {
                    let bar_width = (group_width.saturating_sub(self.bar_gap * 2)) / n_ds as u16;
                    for (ds_i, ds) in self.datasets.iter().enumerate() {
                        let val = ds.values.get(cat_i).copied().unwrap_or(0.0);
                        let bar_x = group_x + self.bar_gap + ds_i as u16 * bar_width;
                        let bar_top = data_to_screen(
                            val,
                            0.0,
                            y_hi,
                            (pa.y + pa.height - 1) as f64,
                            pa.y as f64,
                        )
                        .round() as u16;

                        for x in bar_x..bar_x + bar_width.max(1) {
                            for y in bar_top..pa.y + pa.height {
                                if pa.contains(x, y) {
                                    pb.set_cell(x, y, ' ', ds.color, ds.color, Z_DATA);
                                }
                            }
                        }
                    }
                }
                BarMode::Stacked => {
                    let bar_x = group_x + self.bar_gap;
                    let bar_width = group_width.saturating_sub(self.bar_gap * 2).max(1);
                    let mut bottom = 0.0f64;

                    for ds in &self.datasets {
                        let val = ds.values.get(cat_i).copied().unwrap_or(0.0);
                        let y_bot = data_to_screen(
                            bottom,
                            0.0,
                            y_hi,
                            (pa.y + pa.height - 1) as f64,
                            pa.y as f64,
                        )
                        .round() as u16;
                        let y_top = data_to_screen(
                            bottom + val,
                            0.0,
                            y_hi,
                            (pa.y + pa.height - 1) as f64,
                            pa.y as f64,
                        )
                        .round() as u16;

                        for x in bar_x..bar_x + bar_width {
                            for y in y_top..y_bot {
                                if pa.contains(x, y) {
                                    pb.set_cell(x, y, ' ', ds.color, ds.color, Z_DATA);
                                }
                            }
                        }

                        bottom += val;
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
}
