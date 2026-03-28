//! Seaborn-style faceted subplot grid.
//!
//! `FacetGrid` partitions data by row and column categorical variables and
//! renders a separate sub-plot in each cell. An optional hue variable adds
//! colour-coded grouping within each cell.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::facet_grid::{FacetData, FacetGrid, FacetRecord};
//!
//! let data = FacetData::new()
//!     .record(FacetRecord { row_key: "A".into(), col_key: "X".into(), hue_key: None, x: 1.0, y: 2.0 })
//!     .record(FacetRecord { row_key: "A".into(), col_key: "X".into(), hue_key: None, x: 2.0, y: 3.0 });
//!
//! let grid = FacetGrid::new(data).map_scatter();
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::axis::{Axis, Bounds};
use crate::color_cycle::ColorCycle;
use crate::series::Series;
use crate::theme::Theme;
use crate::widgets::line_plot::LinePlot;
use crate::widgets::scatter_plot::ScatterPlot;

/// Controls how axis bounds are shared across facet panels.
///
/// By default all panels share the same axis bounds (`Shared`). The free-scale
/// variants allow each row or column (or every individual panel) to compute its
/// own bounds, which is useful when the data ranges differ widely across facets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum FacetScales {
    /// All panels share the same axis bounds (default).
    #[default]
    Shared,
    /// Each column can have independent x-axis bounds; y-axis is shared.
    FreeX,
    /// Each row can have independent y-axis bounds; x-axis is shared.
    FreeY,
    /// Both axes are independent per panel.
    Free,
}

/// A single record in a facet dataset.
#[derive(Clone, Debug)]
pub struct FacetRecord {
    /// Row facet category key.
    pub row_key: String,
    /// Column facet category key.
    pub col_key: String,
    /// Optional hue category key for within-cell grouping.
    pub hue_key: Option<String>,
    /// X data value.
    pub x: f64,
    /// Y data value.
    pub y: f64,
}

/// Grouped data container for [`FacetGrid`].
///
/// Holds a flat list of [`FacetRecord`]s and provides helpers to extract
/// unique keys and filtered series data.
#[derive(Clone, Debug)]
pub struct FacetData {
    records: Vec<FacetRecord>,
}

impl Default for FacetData {
    fn default() -> Self {
        Self::new()
    }
}

impl FacetData {
    /// Create an empty `FacetData`.
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Add a single record (builder style).
    pub fn record(mut self, rec: FacetRecord) -> Self {
        self.records.push(rec);
        self
    }

    /// Add many records at once (builder style).
    pub fn records(mut self, recs: Vec<FacetRecord>) -> Self {
        self.records.extend(recs);
        self
    }

    /// Build `FacetData` from parallel column slices.
    ///
    /// All slices must have the same length. If `hue_var` is `Some`, it must
    /// also have the same length.
    pub fn from_columns(
        x: &[f64],
        y: &[f64],
        row_var: &[String],
        col_var: &[String],
        hue_var: Option<&[String]>,
    ) -> Self {
        let n = x.len().min(y.len()).min(row_var.len()).min(col_var.len());
        let mut records = Vec::with_capacity(n);
        for i in 0..n {
            records.push(FacetRecord {
                row_key: row_var[i].clone(),
                col_key: col_var[i].clone(),
                hue_key: hue_var.and_then(|h| h.get(i).cloned()),
                x: x[i],
                y: y[i],
            });
        }
        Self { records }
    }

    /// Unique row keys in order of first appearance.
    pub fn row_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        for rec in &self.records {
            if !keys.contains(&rec.row_key) {
                keys.push(rec.row_key.clone());
            }
        }
        keys
    }

    /// Unique column keys in order of first appearance.
    pub fn col_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        for rec in &self.records {
            if !keys.contains(&rec.col_key) {
                keys.push(rec.col_key.clone());
            }
        }
        keys
    }

    /// Unique hue keys in order of first appearance.
    pub fn hue_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        for rec in &self.records {
            if let Some(ref hk) = rec.hue_key
                && !keys.contains(hk)
            {
                keys.push(hk.clone());
            }
        }
        keys
    }

    /// Return `(x, y)` pairs matching the given row, column, and optional hue.
    pub fn series_data(&self, row: &str, col: &str, hue: Option<&str>) -> Vec<(f64, f64)> {
        self.records
            .iter()
            .filter(|r| {
                r.row_key == row
                    && r.col_key == col
                    && match hue {
                        Some(h) => r.hue_key.as_deref() == Some(h),
                        None => true,
                    }
            })
            .map(|r| (r.x, r.y))
            .collect()
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the data is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// Render function type for each facet cell.
///
/// Called with `(data, row_key, col_key, color_cycle, cell_rect, buffer)`.
pub type FacetRenderFn = Box<dyn Fn(&FacetData, &str, &str, &ColorCycle, Rect, &mut Buffer)>;

/// Seaborn-style faceted subplot grid.
///
/// Partitions data by row and column categorical variables, rendering a
/// separate sub-plot in each grid cell.
pub struct FacetGrid {
    data: FacetData,
    render_fn: Option<FacetRenderFn>,
    gap: u16,
    share_x: bool,
    share_y: bool,
    scales: FacetScales,
    col_wrap: Option<usize>,
    margin_titles: bool,
    suptitle: Option<String>,
    row_titles: bool,
    col_titles: bool,
    theme: Theme,
    color_cycle: ColorCycle,
}

impl FacetGrid {
    /// Create a new `FacetGrid` from facet data.
    pub fn new(data: FacetData) -> Self {
        let theme = Theme::get_default();
        let color_cycle = theme.color_cycle.clone();
        Self {
            data,
            render_fn: None,
            gap: 1,
            share_x: false,
            share_y: false,
            scales: FacetScales::default(),
            col_wrap: None,
            margin_titles: false,
            suptitle: None,
            row_titles: true,
            col_titles: true,
            theme,
            color_cycle,
        }
    }

    /// Set a custom render function for each cell.
    pub fn map(mut self, render_fn: FacetRenderFn) -> Self {
        self.render_fn = Some(render_fn);
        self
    }

    /// Use a built-in line plot renderer for each cell.
    ///
    /// If hue keys exist, one [`Series`] per hue is created in each cell.
    pub fn map_line(mut self) -> Self {
        let share_x = self.share_x;
        let share_y = self.share_y;
        let scales = self.scales.clone();
        let theme = self.theme.clone();

        self.render_fn = Some(Box::new(move |data, row, col, cycle, area, buf| {
            let (x_bounds, y_bounds) =
                resolve_cell_bounds(data, row, col, share_x, share_y, &scales);
            let cfg = CellConfig {
                data,
                row,
                col,
                cycle,
                x_bounds,
                y_bounds,
                theme: &theme,
            };
            render_line_cell(&cfg, area, buf);
        }));
        self
    }

    /// Use a built-in scatter plot renderer for each cell.
    ///
    /// If hue keys exist, one [`Series`] per hue is created in each cell.
    pub fn map_scatter(mut self) -> Self {
        let share_x = self.share_x;
        let share_y = self.share_y;
        let scales = self.scales.clone();
        let theme = self.theme.clone();

        self.render_fn = Some(Box::new(move |data, row, col, cycle, area, buf| {
            let (x_bounds, y_bounds) =
                resolve_cell_bounds(data, row, col, share_x, share_y, &scales);
            let cfg = CellConfig {
                data,
                row,
                col,
                cycle,
                x_bounds,
                y_bounds,
                theme: &theme,
            };
            render_scatter_cell(&cfg, area, buf);
        }));
        self
    }

    /// Set the gap between cells in terminal characters.
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    /// Share x-axis bounds across all cells.
    pub fn share_x(mut self, shared: bool) -> Self {
        self.share_x = shared;
        self
    }

    /// Share y-axis bounds across all cells.
    pub fn share_y(mut self, shared: bool) -> Self {
        self.share_y = shared;
        self
    }

    /// Set the axis scale sharing mode.
    ///
    /// This provides finer control than [`share_x`](Self::share_x) /
    /// [`share_y`](Self::share_y). When set to anything other than
    /// [`FacetScales::Shared`], the per-axis sharing flags are ignored in
    /// favour of the scales mode.
    ///
    /// - [`FacetScales::Shared`] — all panels use global bounds (equivalent to
    ///   `share_x(true).share_y(true)`).
    /// - [`FacetScales::FreeX`] — each column computes its own x-bounds;
    ///   y-bounds are shared globally.
    /// - [`FacetScales::FreeY`] — each row computes its own y-bounds; x-bounds
    ///   are shared globally.
    /// - [`FacetScales::Free`] — each panel is fully independent.
    pub fn scales(mut self, scales: FacetScales) -> Self {
        self.scales = scales;
        self
    }

    /// Wrap facets into multiple rows after this many columns.
    ///
    /// When set, facets are laid out left-to-right and wrap to a new row after
    /// every `n` columns. For example, `col_wrap(3)` with 7 facets produces
    /// rows of 3, 3, and 1. A value of 0 is treated as no wrapping.
    pub fn col_wrap(mut self, n: usize) -> Self {
        if n == 0 {
            self.col_wrap = None;
        } else {
            self.col_wrap = Some(n);
        }
        self
    }

    /// Enable or disable margin titles.
    ///
    /// When `true`, row labels are rendered once in the right margin and column
    /// labels once above the grid, rather than repeating as individual subplot
    /// titles. This saves vertical space when there are many panels.
    pub fn margin_titles(mut self, enabled: bool) -> Self {
        self.margin_titles = enabled;
        self
    }

    /// Set a super-title displayed above the entire grid.
    pub fn suptitle(mut self, title: impl Into<String>) -> Self {
        self.suptitle = Some(title.into());
        self
    }

    /// Show or hide row titles on the right side.
    pub fn row_titles(mut self, show: bool) -> Self {
        self.row_titles = show;
        self
    }

    /// Show or hide column titles above each column.
    pub fn col_titles(mut self, show: bool) -> Self {
        self.col_titles = show;
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.color_cycle = theme.color_cycle.clone();
        self.theme = theme;
        self
    }

    /// Set the color cycle.
    pub fn color_cycle(mut self, cycle: ColorCycle) -> Self {
        self.color_cycle = cycle;
        self
    }
}

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for FacetGrid {
    fn render_plotters(
        &self,
        area: ratatui::layout::Rect,
        buf: &mut ratatui::buffer::Buffer,
        theme: &crate::theme::Theme,
    ) {
        use crate::plotters_render::{bridge, theme_bridge};
        bridge::render_plotters_to_buf(
            area, buf, theme_bridge::theme_bg_rgb(theme),
            |_root| { /* Minimal stub - full plotters rendering TBD */ },
        );
    }
}

impl Widget for &FacetGrid {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        let row_keys = self.data.row_keys();
        let col_keys = self.data.col_keys();

        if row_keys.is_empty() || col_keys.is_empty() {
            return;
        }

        // Build a flat list of (row_key, col_key) cells. When col_wrap is
        // active we iterate all unique (row, col) combinations in order and
        // lay them out with wrapping; otherwise we use the original row x col
        // grid.
        let (grid_cells, n_grid_rows, n_grid_cols) = if let Some(wrap) = self.col_wrap {
            // Collect all unique (row, col) pairs preserving insertion order.
            let mut cells: Vec<(String, String)> = Vec::new();
            for rk in &row_keys {
                for ck in &col_keys {
                    // Only include cells that have data.
                    if self
                        .data
                        .records
                        .iter()
                        .any(|r| r.row_key == *rk && r.col_key == *ck)
                    {
                        cells.push((rk.clone(), ck.clone()));
                    }
                }
            }
            let total = cells.len();
            let cols = wrap.min(total).max(1);
            let rows = if total == 0 { 0 } else { total.div_ceil(cols) };
            (cells, rows, cols)
        } else {
            // Standard grid: every row x col combination.
            let mut cells: Vec<(String, String)> = Vec::new();
            for rk in &row_keys {
                for ck in &col_keys {
                    cells.push((rk.clone(), ck.clone()));
                }
            }
            (cells, row_keys.len(), col_keys.len())
        };

        if n_grid_rows == 0 || n_grid_cols == 0 {
            return;
        }

        // ── Margin titles ────────────────────────────────────────────────
        // When margin_titles is enabled, we render row/col labels in the
        // margins and skip the per-cell title rows, saving vertical space.
        let show_col_titles = self.col_titles;
        let show_row_titles = self.row_titles;
        // margin_titles only applies when both row and col titles are enabled
        let use_margin = self.margin_titles && (show_row_titles || show_col_titles);

        // Reserve space for suptitle (2 rows if present)
        let suptitle_height: u16 = if self.suptitle.is_some() { 2 } else { 0 };
        // Reserve space for column titles (1 row if enabled).
        // When margin_titles is on the same single row is reused for margin
        // column labels, so the height is identical in both modes.
        let col_title_height: u16 = if show_col_titles { 1 } else { 0 };
        // Reserve space for row titles (right side)
        let row_title_width: u16 = if show_row_titles {
            let max_len = row_keys.iter().map(|k| k.len()).max().unwrap_or(0) as u16;
            max_len.saturating_add(2).min(area.width / 4)
        } else {
            0
        };

        let label_style = Style::default().fg(self.theme.foreground);

        // Draw suptitle centered at top
        if let Some(ref title) = self.suptitle {
            let text_len = title.len() as u16;
            let start = area.x + area.width.saturating_sub(text_len) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_style(label_style);
                }
            }
        }

        // Compute grid area (below suptitle and col titles, left of row titles)
        let grid_y = area.y + suptitle_height + col_title_height;
        let grid_width = area.width.saturating_sub(row_title_width);
        let grid_height = area
            .height
            .saturating_sub(suptitle_height)
            .saturating_sub(col_title_height);

        if grid_width < 2 || grid_height < 2 {
            return;
        }

        // Compute cell sizes
        let total_h_gap = self
            .gap
            .saturating_mul(n_grid_rows.saturating_sub(1) as u16);
        let total_w_gap = self
            .gap
            .saturating_mul(n_grid_cols.saturating_sub(1) as u16);
        let cell_height = grid_height.saturating_sub(total_h_gap) / n_grid_rows as u16;
        let cell_width = grid_width.saturating_sub(total_w_gap) / n_grid_cols as u16;

        if cell_width < 2 || cell_height < 2 {
            return;
        }

        // ── Draw column titles ───────────────────────────────────────────
        if show_col_titles {
            let title_y = area.y + suptitle_height;
            if use_margin {
                // Margin mode: render each unique column label once, centred
                // over the column position. When col_wrap is active we label
                // each grid column index 0..n_grid_cols using the
                // corresponding col_key (wrapping may reuse positions).
                let labels: Vec<&str> = if self.col_wrap.is_some() {
                    // Use the first n_grid_cols col_keys (they repeat).
                    (0..n_grid_cols)
                        .map(|c| grid_cells.get(c).map(|(_, ck)| ck.as_str()).unwrap_or(""))
                        .collect()
                } else {
                    col_keys.iter().map(|s| s.as_str()).collect()
                };
                for (c, label) in labels.iter().enumerate() {
                    let cell_x = area.x + (c as u16) * (cell_width + self.gap);
                    let text_len = label.len() as u16;
                    let start = cell_x + cell_width.saturating_sub(text_len) / 2;
                    for (i, ch) in label.chars().enumerate() {
                        let x = start + i as u16;
                        if x < area.x + grid_width {
                            buf[(x, title_y)].set_char(ch).set_style(label_style);
                        }
                    }
                }
            } else {
                for (c, col_key) in col_keys.iter().enumerate() {
                    let cell_x = area.x + (c as u16) * (cell_width + self.gap);
                    let text_len = col_key.len() as u16;
                    let start = cell_x + cell_width.saturating_sub(text_len) / 2;
                    for (i, ch) in col_key.chars().enumerate() {
                        let x = start + i as u16;
                        if x < area.x + grid_width {
                            buf[(x, title_y)].set_char(ch).set_style(label_style);
                        }
                    }
                }
            }
        }

        // ── Draw row titles ──────────────────────────────────────────────
        if show_row_titles {
            let title_x = area.x + grid_width + 1;
            if use_margin {
                // Margin mode: render each unique row label once, centred
                // vertically over the row position.
                let labels: Vec<&str> = if self.col_wrap.is_some() {
                    // With wrapping the grid rows may not correspond to
                    // data row_keys. Label each grid row with the row_key
                    // of the first cell in that row.
                    (0..n_grid_rows)
                        .map(|r| {
                            let idx = r * n_grid_cols;
                            grid_cells.get(idx).map(|(rk, _)| rk.as_str()).unwrap_or("")
                        })
                        .collect()
                } else {
                    row_keys.iter().map(|s| s.as_str()).collect()
                };
                for (r, label) in labels.iter().enumerate() {
                    let cell_y = grid_y + (r as u16) * (cell_height + self.gap);
                    let mid_y = cell_y + cell_height / 2;
                    for (i, ch) in label.chars().enumerate() {
                        let x = title_x + i as u16;
                        if x < area.x + area.width && mid_y < area.y + area.height {
                            buf[(x, mid_y)].set_char(ch).set_style(label_style);
                        }
                    }
                }
            } else {
                for (r, row_key) in row_keys.iter().enumerate() {
                    let cell_y = grid_y + (r as u16) * (cell_height + self.gap);
                    let mid_y = cell_y + cell_height / 2;
                    for (i, ch) in row_key.chars().enumerate() {
                        let x = title_x + i as u16;
                        if x < area.x + area.width && mid_y < area.y + area.height {
                            buf[(x, mid_y)].set_char(ch).set_style(label_style);
                        }
                    }
                }
            }
        }

        // Render each cell
        if let Some(ref render_fn) = self.render_fn {
            for (idx, (row_key, col_key)) in grid_cells.iter().enumerate() {
                let grid_row = idx / n_grid_cols;
                let grid_col = idx % n_grid_cols;

                let cell_x = area.x + (grid_col as u16) * (cell_width + self.gap);
                let cell_y = grid_y + (grid_row as u16) * (cell_height + self.gap);

                // Clamp so cell does not extend beyond the grid area.
                let clamped_w = cell_width.min((area.x + grid_width).saturating_sub(cell_x));
                let clamped_h = cell_height.min((area.y + area.height).saturating_sub(cell_y));

                if clamped_w < 2 || clamped_h < 2 {
                    continue;
                }

                let cell_area = Rect::new(cell_x, cell_y, clamped_w, clamped_h);

                render_fn(
                    &self.data,
                    row_key,
                    col_key,
                    &self.color_cycle,
                    cell_area,
                    buf,
                );
            }
        }
    }
}

/// Optional axis range: `(min, max)` or `None` for auto-compute.
type OptBounds = Option<(f64, f64)>;

/// Compute global x/y bounds across all records.
fn global_bounds(data: &FacetData) -> (f64, f64, f64, f64) {
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for rec in &data.records {
        if rec.x.is_finite() {
            if rec.x < x_min {
                x_min = rec.x;
            }
            if rec.x > x_max {
                x_max = rec.x;
            }
        }
        if rec.y.is_finite() {
            if rec.y < y_min {
                y_min = rec.y;
            }
            if rec.y > y_max {
                y_max = rec.y;
            }
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
    (x_min, x_max, y_min, y_max)
}

/// Compute x-bounds for records matching a given column key.
fn column_x_bounds(data: &FacetData, col: &str) -> (f64, f64) {
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    for rec in &data.records {
        if rec.col_key == col && rec.x.is_finite() {
            if rec.x < x_min {
                x_min = rec.x;
            }
            if rec.x > x_max {
                x_max = rec.x;
            }
        }
    }
    if x_min.is_infinite() {
        (0.0, 1.0)
    } else {
        (x_min, x_max)
    }
}

/// Compute y-bounds for records matching a given row key.
fn row_y_bounds(data: &FacetData, row: &str) -> (f64, f64) {
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for rec in &data.records {
        if rec.row_key == row && rec.y.is_finite() {
            if rec.y < y_min {
                y_min = rec.y;
            }
            if rec.y > y_max {
                y_max = rec.y;
            }
        }
    }
    if y_min.is_infinite() {
        (0.0, 1.0)
    } else {
        (y_min, y_max)
    }
}

/// Compute x/y bounds for a single cell.
fn cell_bounds(data: &FacetData, row: &str, col: &str) -> (f64, f64, f64, f64) {
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for rec in &data.records {
        if rec.row_key == row && rec.col_key == col {
            if rec.x.is_finite() {
                if rec.x < x_min {
                    x_min = rec.x;
                }
                if rec.x > x_max {
                    x_max = rec.x;
                }
            }
            if rec.y.is_finite() {
                if rec.y < y_min {
                    y_min = rec.y;
                }
                if rec.y > y_max {
                    y_max = rec.y;
                }
            }
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
    (x_min, x_max, y_min, y_max)
}

/// Determine the effective x/y bounds for a cell based on the legacy
/// `share_x`/`share_y` flags and the new [`FacetScales`] mode.
///
/// Returns `(Option<(x_min, x_max)>, Option<(y_min, y_max)>)`. `None` means
/// let the widget auto-compute its bounds from the cell data.
fn resolve_cell_bounds(
    data: &FacetData,
    row: &str,
    col: &str,
    share_x: bool,
    share_y: bool,
    scales: &FacetScales,
) -> (OptBounds, OptBounds) {
    // When scales is not the default Shared, it takes precedence over the
    // legacy share_x / share_y booleans.
    match scales {
        FacetScales::Shared => {
            // Fall back to the legacy booleans.
            if !share_x && !share_y {
                return (None, None);
            }
            let (gx_min, gx_max, gy_min, gy_max) = global_bounds(data);
            let xb = if share_x {
                Some((gx_min, gx_max))
            } else {
                None
            };
            let yb = if share_y {
                Some((gy_min, gy_max))
            } else {
                None
            };
            (xb, yb)
        }
        FacetScales::FreeX => {
            // X is free per column, Y is shared globally.
            let (_, _, gy_min, gy_max) = global_bounds(data);
            let (cx_min, cx_max) = column_x_bounds(data, col);
            (Some((cx_min, cx_max)), Some((gy_min, gy_max)))
        }
        FacetScales::FreeY => {
            // Y is free per row, X is shared globally.
            let (gx_min, gx_max, _, _) = global_bounds(data);
            let (ry_min, ry_max) = row_y_bounds(data, row);
            (Some((gx_min, gx_max)), Some((ry_min, ry_max)))
        }
        FacetScales::Free => {
            // Fully independent — use per-cell bounds.
            let (cx_min, cx_max, cy_min, cy_max) = cell_bounds(data, row, col);
            (Some((cx_min, cx_max)), Some((cy_min, cy_max)))
        }
    }
}

/// Configuration passed to cell render helpers to avoid too many arguments.
struct CellConfig<'a> {
    data: &'a FacetData,
    row: &'a str,
    col: &'a str,
    cycle: &'a ColorCycle,
    /// Pre-computed x-axis bounds, or `None` for auto.
    x_bounds: OptBounds,
    /// Pre-computed y-axis bounds, or `None` for auto.
    y_bounds: OptBounds,
    theme: &'a Theme,
}

/// Build series for a single cell, grouping by hue if present.
fn build_cell_series(cfg: &CellConfig<'_>) -> Vec<Series> {
    let hue_keys = cfg.data.hue_keys();
    let mut series_vec = Vec::new();

    if hue_keys.is_empty() {
        let pts = cfg.data.series_data(cfg.row, cfg.col, None);
        let color = cfg.cycle.at(0);
        series_vec.push(
            Series::new(format!("{}/{}", cfg.row, cfg.col))
                .data(pts)
                .color(color),
        );
    } else {
        for (i, hk) in hue_keys.iter().enumerate() {
            let pts = cfg.data.series_data(cfg.row, cfg.col, Some(hk));
            let color = cfg.cycle.at(i);
            series_vec.push(Series::new(hk.clone()).data(pts).color(color));
        }
    }

    series_vec
}

/// Render a line plot in a single facet cell.
fn render_line_cell(cfg: &CellConfig<'_>, area: Rect, buf: &mut Buffer) {
    let series_vec = build_cell_series(cfg);

    let mut plot = LinePlot::new()
        .series_vec(series_vec)
        .show_legend(false)
        .theme(cfg.theme.clone());

    if let Some((x_min, x_max)) = cfg.x_bounds {
        plot = plot.x_axis(Axis::new().bounds(Bounds::Manual(x_min, x_max)));
    }
    if let Some((y_min, y_max)) = cfg.y_bounds {
        plot = plot.y_axis(Axis::new().bounds(Bounds::Manual(y_min, y_max)));
    }

    (&plot).render(area, buf);
}

/// Render a scatter plot in a single facet cell.
fn render_scatter_cell(cfg: &CellConfig<'_>, area: Rect, buf: &mut Buffer) {
    let series_vec = build_cell_series(cfg);

    let mut plot = ScatterPlot::new()
        .show_legend(false)
        .theme(cfg.theme.clone());

    for s in series_vec {
        plot = plot.series(s);
    }

    if let Some((x_min, x_max)) = cfg.x_bounds {
        plot = plot.x_axis(Axis::new().bounds(Bounds::Manual(x_min, x_max)));
    }
    if let Some((y_min, y_max)) = cfg.y_bounds {
        plot = plot.y_axis(Axis::new().bounds(Bounds::Manual(y_min, y_max)));
    }

    (&plot).render(area, buf);
}
