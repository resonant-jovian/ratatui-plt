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
        let theme = self.theme.clone();

        self.render_fn = Some(Box::new(move |data, row, col, cycle, area, buf| {
            let cfg = CellConfig {
                data,
                row,
                col,
                cycle,
                share_x,
                share_y,
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
        let theme = self.theme.clone();

        self.render_fn = Some(Box::new(move |data, row, col, cycle, area, buf| {
            let cfg = CellConfig {
                data,
                row,
                col,
                cycle,
                share_x,
                share_y,
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

impl Widget for &FacetGrid {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        let row_keys = self.data.row_keys();
        let col_keys = self.data.col_keys();

        let n_rows = row_keys.len();
        let n_cols = col_keys.len();
        if n_rows == 0 || n_cols == 0 {
            return;
        }

        // Reserve space for suptitle (2 rows if present)
        let suptitle_height: u16 = if self.suptitle.is_some() { 2 } else { 0 };
        // Reserve space for column titles (1 row if enabled)
        let col_title_height: u16 = if self.col_titles { 1 } else { 0 };
        // Reserve space for row titles (right side)
        let row_title_width: u16 = if self.row_titles {
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
        let total_h_gap = self.gap.saturating_mul(n_rows.saturating_sub(1) as u16);
        let total_w_gap = self.gap.saturating_mul(n_cols.saturating_sub(1) as u16);
        let cell_height = grid_height.saturating_sub(total_h_gap) / n_rows as u16;
        let cell_width = grid_width.saturating_sub(total_w_gap) / n_cols as u16;

        if cell_width < 2 || cell_height < 2 {
            return;
        }

        // Draw column titles
        if self.col_titles {
            let title_y = area.y + suptitle_height;
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

        // Draw row titles
        if self.row_titles {
            let title_x = area.x + grid_width + 1;
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

        // Render each cell
        if let Some(ref render_fn) = self.render_fn {
            for (r, row_key) in row_keys.iter().enumerate() {
                for (c, col_key) in col_keys.iter().enumerate() {
                    let cell_x = area.x + (c as u16) * (cell_width + self.gap);
                    let cell_y = grid_y + (r as u16) * (cell_height + self.gap);

                    let cell_area = Rect::new(
                        cell_x,
                        cell_y,
                        cell_width.min(area.x + grid_width - cell_x),
                        cell_height.min(area.y + area.height - cell_y),
                    );

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
}

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

/// Configuration passed to cell render helpers to avoid too many arguments.
struct CellConfig<'a> {
    data: &'a FacetData,
    row: &'a str,
    col: &'a str,
    cycle: &'a ColorCycle,
    share_x: bool,
    share_y: bool,
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

    if cfg.share_x || cfg.share_y {
        let (gx_min, gx_max, gy_min, gy_max) = global_bounds(cfg.data);
        if cfg.share_x {
            plot = plot.x_axis(Axis::new().bounds(Bounds::Manual(gx_min, gx_max)));
        }
        if cfg.share_y {
            plot = plot.y_axis(Axis::new().bounds(Bounds::Manual(gy_min, gy_max)));
        }
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

    if cfg.share_x || cfg.share_y {
        let (gx_min, gx_max, gy_min, gy_max) = global_bounds(cfg.data);
        if cfg.share_x {
            plot = plot.x_axis(Axis::new().bounds(Bounds::Manual(gx_min, gx_max)));
        }
        if cfg.share_y {
            plot = plot.y_axis(Axis::new().bounds(Bounds::Manual(gy_min, gy_max)));
        }
    }

    (&plot).render(area, buf);
}
