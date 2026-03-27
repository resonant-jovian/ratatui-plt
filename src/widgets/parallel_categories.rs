//! Parallel categories (categorical parallel coordinates) widget.
//!
//! Visualizes categorical data flows across multiple dimensions. Each dimension
//! is a vertical axis with category blocks stacked proportionally, and colored
//! ribbons connect categories between adjacent dimensions with width proportional
//! to frequency.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::parallel_categories::{
//!     CategoricalDimension, CategoricalRecord, ParallelCategories,
//! };
//!
//! let plot = ParallelCategories::new()
//!     .dimension(CategoricalDimension::new("Class", vec!["First", "Second", "Third"]))
//!     .dimension(CategoricalDimension::new("Survived", vec!["No", "Yes"]))
//!     .record(CategoricalRecord::new(vec![0, 0]).count(145))
//!     .record(CategoricalRecord::new(vec![0, 1]).count(203))
//!     .record(CategoricalRecord::new(vec![1, 0]).count(233))
//!     .record(CategoricalRecord::new(vec![1, 1]).count(118))
//!     .record(CategoricalRecord::new(vec![2, 0]).count(496))
//!     .record(CategoricalRecord::new(vec![2, 1]).count(178))
//!     .title("Titanic Survival by Class");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::plot_buffer::{PlotBackend, Z_CHROME, Z_DATA, create_backend};
use crate::spines::Spines;
use crate::theme::Theme;

/// A single categorical dimension (vertical axis) in the parallel categories plot.
#[derive(Clone, Debug)]
pub struct CategoricalDimension {
    /// Axis label displayed above the dimension.
    pub name: String,
    /// Ordered list of category labels for this dimension.
    pub categories: Vec<String>,
}

impl CategoricalDimension {
    /// Create a new dimension with the given name and category labels.
    pub fn new(name: impl Into<String>, categories: Vec<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            categories: categories.into_iter().map(Into::into).collect(),
        }
    }
}

/// A data record: one combination of category indices with a frequency count.
#[derive(Clone, Debug)]
pub struct CategoricalRecord {
    /// Category index per dimension (one entry per dimension).
    pub indices: Vec<usize>,
    /// Frequency/count of this combination (default: 1).
    pub count: usize,
}

impl CategoricalRecord {
    /// Create a new record with the given category indices.
    pub fn new(indices: Vec<usize>) -> Self {
        Self { indices, count: 1 }
    }

    /// Set the frequency count for this record.
    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }
}

/// Parallel categories widget for categorical flow visualization.
///
/// Draws multiple vertical axes (one per dimension), each displaying category
/// blocks stacked proportionally to their total count. Colored ribbons between
/// adjacent axes show flows with width proportional to frequency.
#[derive(Clone)]
pub struct ParallelCategories {
    dimensions: Vec<CategoricalDimension>,
    records: Vec<CategoricalRecord>,
    title: Option<String>,
    theme: Theme,
    spines: Spines,
}

impl Default for ParallelCategories {
    fn default() -> Self {
        Self {
            dimensions: Vec::new(),
            records: Vec::new(),
            title: None,
            theme: Theme::get_default(),
            spines: Spines::default(),
        }
    }
}

impl ParallelCategories {
    /// Create an empty parallel categories plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single dimension.
    pub fn dimension(mut self, dim: CategoricalDimension) -> Self {
        self.dimensions.push(dim);
        self
    }

    /// Set all dimensions at once.
    pub fn dimensions(mut self, dims: Vec<CategoricalDimension>) -> Self {
        self.dimensions = dims;
        self
    }

    /// Add a single record.
    pub fn record(mut self, rec: CategoricalRecord) -> Self {
        self.records.push(rec);
        self
    }

    /// Set all records at once.
    pub fn records(mut self, records: Vec<CategoricalRecord>) -> Self {
        self.records = records;
        self
    }

    /// Set the plot title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
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
}

// ── Internal helpers ────────────────────────────────────────────────────────

/// Aggregated flow between two categories in adjacent dimensions.
struct Flow {
    src_cat: usize,
    dst_cat: usize,
    total: usize,
    color: Color,
}

/// Compute per-category totals for a single dimension.
fn category_totals(
    dim_idx: usize,
    dim: &CategoricalDimension,
    records: &[CategoricalRecord],
) -> Vec<usize> {
    let n = dim.categories.len();
    let mut totals = vec![0usize; n];
    for rec in records {
        if let Some(&ci) = rec.indices.get(dim_idx)
            && ci < n
        {
            totals[ci] += rec.count;
        }
    }
    totals
}

/// Aggregate flows between two adjacent dimensions, assigning colours via `get_color`.
fn compute_flows(
    left: usize,
    right: usize,
    ln: usize,
    rn: usize,
    records: &[CategoricalRecord],
    get_color: &dyn Fn(&CategoricalRecord) -> Color,
) -> Vec<Flow> {
    let mut grid = vec![vec![0usize; rn]; ln];
    let mut colors: Vec<Vec<Option<Color>>> = vec![vec![None; rn]; ln];
    for rec in records {
        let src = rec.indices.get(left).copied();
        let dst = rec.indices.get(right).copied();
        if let (Some(s), Some(d)) = (src, dst)
            && s < ln
            && d < rn
        {
            grid[s][d] += rec.count;
            if colors[s][d].is_none() {
                colors[s][d] = Some(get_color(rec));
            }
        }
    }
    let mut out = Vec::new();
    for s in 0..ln {
        for d in 0..rn {
            if grid[s][d] > 0 {
                out.push(Flow {
                    src_cat: s,
                    dst_cat: d,
                    total: grid[s][d],
                    color: colors[s][d].unwrap_or(Color::White),
                });
            }
        }
    }
    out
}

/// Vertical layout: per-category (y_start, height) given totals.
fn layout_categories(totals: &[usize], top: u16, height: u16, pad: f64) -> Vec<(f64, f64)> {
    let grand: usize = totals.iter().sum();
    if grand == 0 {
        return totals.iter().map(|_| (top as f64, 1.0)).collect();
    }
    let n = totals.len();
    let avail = (height as f64 - pad * (n as f64 - 1.0).max(0.0)).max(1.0);
    let mut rects = Vec::with_capacity(n);
    let mut y = top as f64;
    for &t in totals {
        let h = (t as f64 / grand as f64 * avail).max(1.0);
        rects.push((y, h));
        y += h + pad;
    }
    rects
}

// ── Widget implementation ───────────────────────────────────────────────────

impl Widget for &ParallelCategories {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let n_dims = self.dimensions.len();
        if n_dims < 2 || area.width < 12 || area.height < 6 {
            return;
        }

        let mut pb = create_backend(area);
        let title_h: u16 = if self.title.is_some() { 1 } else { 0 };
        let label_row = area.y + title_h;
        let plot_top = label_row + 1;
        let plot_bottom = (area.y + area.height - 1).saturating_sub(1);
        if plot_bottom <= plot_top {
            return;
        }
        let plot_h = plot_bottom - plot_top + 1;

        // Title
        if let Some(ref title) = self.title {
            let sx = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = sx + i as u16;
                if x < area.x + area.width {
                    pb.set_char(x, area.y, ch, self.theme.foreground, Z_CHROME);
                }
            }
        }

        // Horizontal positions
        let margin: u16 = 4;
        let bw: u16 = 3; // block width
        let usable = area.width.saturating_sub(margin * 2);
        if usable < (n_dims as u16) * bw {
            return;
        }

        let dim_x: Vec<u16> = (0..n_dims)
            .map(|i| {
                let t = i as f64 / (n_dims as f64 - 1.0).max(1.0);
                area.x + margin + (t * usable.saturating_sub(bw) as f64).round() as u16
            })
            .collect();

        // Vertical layouts per dimension
        let pad = 1.0;
        let layouts: Vec<Vec<(f64, f64)>> = self
            .dimensions
            .iter()
            .enumerate()
            .map(|(i, dim)| {
                let totals = category_totals(i, dim, &self.records);
                layout_categories(&totals, plot_top, plot_h, pad)
            })
            .collect();

        // --- Ribbons (drawn first, behind blocks) ---
        let color_fn = |rec: &CategoricalRecord| -> Color {
            let first = rec.indices.first().copied().unwrap_or(0);
            self.theme.color_cycle.at(first)
        };

        for di in 0..n_dims - 1 {
            let (ld, rd) = (&self.dimensions[di], &self.dimensions[di + 1]);
            let (ln, rn) = (ld.categories.len(), rd.categories.len());
            let flows = compute_flows(di, di + 1, ln, rn, &self.records, &color_fn);

            let mut s_off = vec![0.0f64; ln];
            let mut d_off = vec![0.0f64; rn];
            let lt = category_totals(di, ld, &self.records);
            let rt = category_totals(di + 1, rd, &self.records);

            for flow in &flows {
                let (s, d) = (flow.src_cat, flow.dst_cat);
                if s >= layouts[di].len() || d >= layouts[di + 1].len() {
                    continue;
                }

                let (sy0, sh) = layouts[di][s];
                let (dy0, dh) = layouts[di + 1][d];
                let st = lt.get(s).copied().unwrap_or(1).max(1);
                let dt = rt.get(d).copied().unwrap_or(1).max(1);

                let rsh = flow.total as f64 / st as f64 * sh;
                let rdh = flow.total as f64 / dt as f64 * dh;
                let s_top = sy0 + s_off[s];
                let d_top = dy0 + d_off[d];
                s_off[s] += rsh;
                d_off[d] += rdh;

                let sx = dim_x[di] + bw;
                let tx = dim_x[di + 1];
                if tx <= sx {
                    continue;
                }
                let bw_px = tx - sx;

                for dx in 0..bw_px {
                    let rf = dx as f64 / bw_px as f64;
                    let f = 3.0 * rf * rf - 2.0 * rf * rf * rf; // ease in-out
                    let x = sx + dx;
                    let top = s_top + (d_top - s_top) * f;
                    let bot = (s_top + rsh) + ((d_top + rdh) - (s_top + rsh)) * f;
                    let y0 = top.floor() as u16;
                    let y1 = bot.ceil().max(top.floor() + 1.0) as u16;

                    for y in y0..y1 {
                        if x >= area.x + area.width || y < plot_top || y > plot_bottom {
                            continue;
                        }
                        if y == y0 {
                            if top - (y as f64) > 0.5 {
                                pb.set_char(
                                    x,
                                    y,
                                    self.theme.chars.fill.half_lower,
                                    flow.color,
                                    Z_DATA,
                                );
                            } else {
                                pb.set_cell(
                                    x,
                                    y,
                                    self.theme.chars.fill.solid,
                                    flow.color,
                                    flow.color,
                                    Z_DATA,
                                );
                            }
                        } else if y + 1 >= y1 {
                            if bot - (y as f64) < 0.5 {
                                pb.set_char(
                                    x,
                                    y,
                                    self.theme.chars.fill.half_upper,
                                    flow.color,
                                    Z_DATA,
                                );
                            } else {
                                pb.set_cell(
                                    x,
                                    y,
                                    self.theme.chars.fill.solid,
                                    flow.color,
                                    flow.color,
                                    Z_DATA,
                                );
                            }
                        } else {
                            pb.set_cell(
                                x,
                                y,
                                self.theme.chars.fill.solid,
                                flow.color,
                                flow.color,
                                Z_DATA,
                            );
                        }
                    }
                }
            }
        }

        // --- Category blocks and labels ---
        let block_z = Z_DATA + 10;
        for (di, dim) in self.dimensions.iter().enumerate() {
            let dx = dim_x[di];

            // Dimension name
            let ns = dx.saturating_sub(dim.name.len() as u16 / 2);
            for (j, ch) in dim.name.chars().enumerate() {
                let x = ns + j as u16;
                if x >= area.x && x < area.x + area.width {
                    pb.set_char(x, label_row, ch, self.theme.foreground, Z_CHROME);
                }
            }

            for (ci, label) in dim.categories.iter().enumerate() {
                if ci >= layouts[di].len() {
                    continue;
                }
                let (cy, ch) = layouts[di][ci];
                let cy16 = cy.round() as u16;
                let ch16 = ch.round().max(1.0) as u16;
                let cc = self.theme.color_cycle.at(ci);

                // Filled block
                for dy in 0..ch16 {
                    for ddx in 0..bw {
                        let x = dx + ddx;
                        let y = cy16 + dy;
                        if x < area.x + area.width && y >= plot_top && y <= plot_bottom {
                            pb.set_cell(x, y, self.theme.chars.fill.solid, cc, cc, block_z);
                        }
                    }
                }

                // Optional spine border on outer edges
                if self.spines.left && di == 0 || self.spines.right && di == n_dims - 1 {
                    let ex = if di == 0 { dx } else { dx + bw - 1 };
                    for dy in 0..ch16 {
                        let y = cy16 + dy;
                        if ex < area.x + area.width && y >= plot_top && y <= plot_bottom {
                            pb.set_char(
                                ex,
                                y,
                                self.theme.chars.border.vertical,
                                self.theme.axis_color,
                                Z_CHROME,
                            );
                        }
                    }
                }

                // Category label
                let ly = cy16 + ch16 / 2;
                if ly >= plot_top && ly <= plot_bottom {
                    let lx = if di == 0 && dx > area.x + label.len() as u16 {
                        dx.saturating_sub(label.len() as u16 + 1)
                    } else {
                        dx + bw + 1
                    };
                    for (j, ch) in label.chars().enumerate() {
                        let x = lx + j as u16;
                        if x >= area.x && x < area.x + area.width {
                            pb.set_char(x, ly, ch, self.theme.foreground, Z_CHROME);
                        }
                    }
                }
            }
        }

        pb.composite(buf);
    }
}
