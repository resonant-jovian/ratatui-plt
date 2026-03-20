//! Enhanced bar chart widget with grouped and stacked modes.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::ticker::{TickLocator, TickFormatter};
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
/// use ratatui_sim::widgets::bar_chart::{BarChart, BarDataset, BarMode};
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
}

impl Widget for &BarChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.categories.is_empty() || self.datasets.is_empty() {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let y_label_width: u16 = 8;
        let cat_height: u16 = 1;

        let px = area.x + y_label_width;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(y_label_width + 1);
        let ph = area.height.saturating_sub(title_height + cat_height);

        if pw < 2 || ph < 2 {
            return;
        }

        // Draw title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(Color::White);
                }
            }
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

        // Draw axes
        for x in px..px + pw {
            buf[(x, py + ph)].set_char('─').set_fg(Color::DarkGray);
        }
        for y in py..py + ph {
            buf[(px.saturating_sub(1), y)].set_char('│').set_fg(Color::DarkGray);
        }

        let n_cats = self.categories.len();
        let n_ds = self.datasets.len();
        let group_width = pw / n_cats as u16;

        for (cat_i, _cat) in self.categories.iter().enumerate() {
            let group_x = px + cat_i as u16 * group_width;

            match self.mode {
                BarMode::Grouped => {
                    let bar_width = (group_width.saturating_sub(self.bar_gap * 2))
                        / n_ds as u16;
                    for (ds_i, ds) in self.datasets.iter().enumerate() {
                        let val = ds.values.get(cat_i).copied().unwrap_or(0.0);
                        let bar_x = group_x + self.bar_gap + ds_i as u16 * bar_width;
                        let bar_top = data_to_screen(val, 0.0, y_hi, (py + ph - 1) as f64, py as f64)
                            .round() as u16;

                        for x in bar_x..bar_x + bar_width.max(1) {
                            for y in bar_top..py + ph {
                                if x >= px && x < px + pw && y >= py && y < py + ph {
                                    buf[(x, y)].set_char('█').set_fg(ds.color);
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
                        let y_bot = data_to_screen(bottom, 0.0, y_hi, (py + ph - 1) as f64, py as f64)
                            .round() as u16;
                        let y_top = data_to_screen(bottom + val, 0.0, y_hi, (py + ph - 1) as f64, py as f64)
                            .round() as u16;

                        for x in bar_x..bar_x + bar_width {
                            for y in y_top..y_bot {
                                if x >= px && x < px + pw && y >= py && y < py + ph {
                                    buf[(x, y)].set_char('█').set_fg(ds.color);
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
            let label_y = py + ph;
            if label_y < area.y + area.height {
                for (j, ch) in cat.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, label_y)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        }

        // Y axis tick labels
        let y_ticks = crate::ticker::MaxNLocator::new(5).tick_values(0.0, y_hi);
        for &tv in &y_ticks {
            let sy = data_to_screen(tv, 0.0, y_hi, (py + ph - 1) as f64, py as f64);
            let label = crate::ticker::ScalarFormatter.format(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ph {
                let start = px.saturating_sub(label.len() as u16 + 1);
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < px {
                        buf[(lx, yi)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        }
    }
}
