//! Pair plot widget showing pairwise scatter plots with distribution diagonals.
//!
//! Displays an N×N grid where diagonal cells show the distribution of each
//! variable (histogram or KDE) and off-diagonal cells show pairwise scatter
//! plots. Similar to seaborn's `pairplot` or pandas' `scatter_matrix`.
//!
//! Requires the `statistics` feature for KDE on the diagonal.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::pair_plot::{PairPlot, PairPlotColumn, DiagType};
//!
//! let plot = PairPlot::new()
//!     .column(PairPlotColumn::new("sepal_length", vec![5.1, 4.9, 4.7, 7.0, 6.4]))
//!     .column(PairPlotColumn::new("sepal_width", vec![3.5, 3.0, 3.2, 3.2, 3.2]))
//!     .column(PairPlotColumn::new("petal_length", vec![1.4, 1.4, 1.3, 4.7, 4.5]))
//!     .diag(DiagType::Kde)
//!     .title("Iris Pair Plot");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::frame::PlotArea;
use crate::plot_buffer::{PlotBuffer, Z_DATA, Z_MARKER};
use crate::statistics::Kde;
use crate::style::MarkerShape;
use crate::theme::Theme;
use crate::transform::data_to_screen;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A named column of data for use in a [`PairPlot`].
#[derive(Clone, Debug)]
pub struct PairPlotColumn {
    /// Display name for the column (used as axis label).
    pub name: String,
    /// Data values for this column.
    pub values: Vec<f64>,
}

impl PairPlotColumn {
    /// Create a new column with the given name and values.
    pub fn new(name: impl Into<String>, values: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            values,
        }
    }
}

/// Type of distribution to render on the diagonal cells.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum DiagType {
    /// Histogram (binned bar chart).
    #[default]
    Histogram,
    /// Kernel density estimate (smooth curve via Braille).
    Kde,
}

// ---------------------------------------------------------------------------
// Widget
// ---------------------------------------------------------------------------

/// An N×N pairwise scatter plot matrix with distribution diagonals.
///
/// For N columns, produces an N×N grid of sub-panels:
/// - **Diagonal (i, i):** distribution of column i (histogram or KDE).
/// - **Off-diagonal (i, j):** scatter plot of column j (x) vs column i (y).
///
/// # Builder
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::widgets::pair_plot::{PairPlot, PairPlotColumn, DiagType};
///
/// let plot = PairPlot::new()
///     .column(PairPlotColumn::new("x", vec![1.0, 2.0, 3.0]))
///     .column(PairPlotColumn::new("y", vec![4.0, 5.0, 6.0]))
///     .diag(DiagType::Kde)
///     .marker(MarkerShape::Dot)
///     .title("My Pair Plot");
/// ```
pub struct PairPlot {
    columns: Vec<PairPlotColumn>,
    diag_type: DiagType,
    title: Option<String>,
    marker: MarkerShape,
    color: Option<Color>,
    theme: Theme,
    /// Number of histogram bins for diagonal histograms (default 10).
    hist_bins: usize,
    /// Number of KDE evaluation points per cell width (default 60).
    kde_points: usize,
    /// Gap between sub-panel cells in characters (default 1).
    gap: u16,
}

impl Default for PairPlot {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            diag_type: DiagType::default(),
            title: None,
            marker: MarkerShape::Dot,
            color: None,
            theme: Theme::get_default(),
            hist_bins: 10,
            kde_points: 60,
            gap: 1,
        }
    }
}

impl PairPlot {
    /// Create a new empty pair plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a data column.
    pub fn column(mut self, col: PairPlotColumn) -> Self {
        self.columns.push(col);
        self
    }

    /// Set the diagonal distribution type (default: [`DiagType::Histogram`]).
    pub fn diag(mut self, dt: DiagType) -> Self {
        self.diag_type = dt;
        self
    }

    /// Set the plot title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the marker shape used for scatter points (default: [`MarkerShape::Dot`]).
    pub fn marker(mut self, marker: MarkerShape) -> Self {
        self.marker = marker;
        self
    }

    /// Override the data color. If not set, uses the theme's color cycle.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the number of histogram bins for diagonal cells (default 10).
    pub fn hist_bins(mut self, bins: usize) -> Self {
        self.hist_bins = bins.max(1);
        self
    }

    /// Set the number of KDE evaluation points (default 60).
    pub fn kde_points(mut self, n: usize) -> Self {
        self.kde_points = n.max(4);
        self
    }

    /// Set the gap between sub-panels in characters (default 1).
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

impl Widget for &PairPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let n = self.columns.len();
        if n == 0 || area.width < 4 || area.height < 4 {
            return;
        }

        // Reserve space for the title if present
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };

        // Reserve space for column-name labels on left and bottom edges
        // Left labels: max column name length + 1 for padding
        let label_width = self
            .columns
            .iter()
            .map(|c| c.name.len() as u16)
            .max()
            .unwrap_or(0)
            .min(area.width / 4)
            + 1;
        let label_height: u16 = 1; // bottom labels

        // Compute the grid area (excluding title + labels)
        let grid_x = area.x + label_width;
        let grid_y = area.y + title_height;
        let grid_w = area.width.saturating_sub(label_width);
        let grid_h = area.height.saturating_sub(title_height + label_height);

        if grid_w < n as u16 * 2 || grid_h < n as u16 * 2 {
            return;
        }

        // Draw title
        if let Some(ref title) = self.title {
            let tx = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = tx + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        // Compute per-cell dimensions (equal-sized)
        let total_gap_w = self.gap * (n as u16).saturating_sub(1);
        let total_gap_h = self.gap * (n as u16).saturating_sub(1);
        let cell_w = grid_w.saturating_sub(total_gap_w) / n as u16;
        let cell_h = grid_h.saturating_sub(total_gap_h) / n as u16;

        if cell_w < 2 || cell_h < 2 {
            return;
        }

        // Precompute finite data and bounds for each column
        let col_data: Vec<Vec<f64>> = self
            .columns
            .iter()
            .map(|c| c.values.iter().copied().filter(|v| v.is_finite()).collect())
            .collect();

        let col_bounds: Vec<(f64, f64)> = col_data
            .iter()
            .map(|data| {
                if data.is_empty() {
                    return (0.0, 1.0);
                }
                let lo = data.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                if (hi - lo).abs() < f64::EPSILON {
                    (lo - 1.0, hi + 1.0)
                } else {
                    // Add 5% padding
                    let pad = (hi - lo) * 0.05;
                    (lo - pad, hi + pad)
                }
            })
            .collect();

        // Determine the point color
        let point_color = self.color.unwrap_or(self.theme.primary);

        // Render each cell
        for row in 0..n {
            for col in 0..n {
                let cx = grid_x + col as u16 * (cell_w + self.gap);
                let cy = grid_y + row as u16 * (cell_h + self.gap);
                let cell_area = Rect::new(cx, cy, cell_w, cell_h);

                if row == col {
                    // Diagonal: distribution
                    render_diagonal(
                        buf,
                        cell_area,
                        &col_data[row],
                        col_bounds[row],
                        &self.diag_type,
                        self.hist_bins,
                        self.kde_points,
                        point_color,
                        &self.theme,
                    );
                } else {
                    // Off-diagonal: scatter (col j on x, col i on y)
                    render_scatter(
                        buf,
                        cell_area,
                        &col_data[col],
                        &col_data[row],
                        col_bounds[col],
                        col_bounds[row],
                        self.marker,
                        point_color,
                    );
                }

                // Draw cell border
                draw_cell_border(buf, cell_area, self.theme.axis_color, &self.theme);
            }
        }

        // Draw column names along the left edge (row labels) and bottom edge (column labels)
        for (i, c) in self.columns.iter().enumerate() {
            // Left label: vertically centered in the row's cell
            let cy = grid_y + i as u16 * (cell_h + self.gap) + cell_h / 2;
            let name = &c.name;
            let max_chars = label_width.saturating_sub(1) as usize;
            let display: String = if name.len() > max_chars {
                name.chars().take(max_chars).collect()
            } else {
                name.clone()
            };
            let lx = area.x + label_width.saturating_sub(display.len() as u16 + 1);
            for (j, ch) in display.chars().enumerate() {
                let x = lx + j as u16;
                if x < grid_x && cy < area.y + area.height {
                    buf[(x, cy)].set_char(ch).set_fg(self.theme.foreground);
                }
            }

            // Bottom label: horizontally centered in the column's cell
            let cx = grid_x + i as u16 * (cell_w + self.gap);
            let by = grid_y + grid_h;
            if by < area.y + area.height {
                let max_chars_b = cell_w as usize;
                let display_b: String = if name.len() > max_chars_b {
                    name.chars().take(max_chars_b).collect()
                } else {
                    name.clone()
                };
                let bx = cx + cell_w.saturating_sub(display_b.len() as u16) / 2;
                for (j, ch) in display_b.chars().enumerate() {
                    let x = bx + j as u16;
                    if x < area.x + area.width {
                        buf[(x, by)].set_char(ch).set_fg(self.theme.foreground);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Diagonal rendering (histogram / KDE)
// ---------------------------------------------------------------------------

/// Render a distribution on a diagonal cell.
#[allow(clippy::too_many_arguments)]
fn render_diagonal(
    buf: &mut Buffer,
    area: Rect,
    data: &[f64],
    bounds: (f64, f64),
    diag_type: &DiagType,
    hist_bins: usize,
    kde_points: usize,
    color: Color,
    theme: &Theme,
) {
    if data.is_empty() || area.width < 2 || area.height < 2 {
        return;
    }

    let (d_lo, d_hi) = bounds;
    if d_hi <= d_lo {
        return;
    }

    // Leave 1-char border on each side for the cell frame
    let inner = Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    if inner.width < 1 || inner.height < 1 {
        return;
    }

    match diag_type {
        DiagType::Histogram => {
            render_histogram(buf, inner, data, d_lo, d_hi, hist_bins, color, theme);
        }
        DiagType::Kde => {
            render_kde(buf, inner, data, d_lo, d_hi, kde_points, color, theme);
        }
    }
}

/// Render a histogram in the given area.
#[allow(clippy::too_many_arguments)]
fn render_histogram(
    buf: &mut Buffer,
    area: Rect,
    data: &[f64],
    d_lo: f64,
    d_hi: f64,
    n_bins: usize,
    color: Color,
    theme: &Theme,
) {
    let bins = n_bins.max(1);
    let range = d_hi - d_lo;
    if range <= 0.0 {
        return;
    }

    // Count values in each bin
    let mut counts = vec![0usize; bins];
    for &v in data {
        let t = (v - d_lo) / range;
        let idx = (t * bins as f64).floor() as isize;
        if idx >= 0 && (idx as usize) < bins {
            counts[idx as usize] += 1;
        } else if idx as usize == bins {
            // Right edge goes in last bin
            counts[bins - 1] += 1;
        }
    }

    let max_count = counts.iter().copied().max().unwrap_or(0);
    if max_count == 0 {
        return;
    }

    let bin_w_screen = area.width as f64 / bins as f64;

    for (i, &count) in counts.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let bar_height =
            ((count as f64 / max_count as f64) * area.height as f64).round() as u16;

        let bar_x_start = area.x + (i as f64 * bin_w_screen).round() as u16;
        let bar_x_end = area.x + (((i + 1) as f64) * bin_w_screen).round() as u16;

        for x in bar_x_start..bar_x_end.min(area.x + area.width) {
            for dy in 0..bar_height.min(area.height) {
                let y = area.y + area.height - 1 - dy;
                if y >= area.y {
                    buf[(x, y)]
                        .set_char(theme.chars.fill.solid)
                        .set_fg(color);
                }
            }
        }
    }
}

/// Render a KDE curve using Braille line segments.
#[allow(clippy::too_many_arguments)]
fn render_kde(
    buf: &mut Buffer,
    area: Rect,
    data: &[f64],
    d_lo: f64,
    d_hi: f64,
    n_points: usize,
    color: Color,
    _theme: &Theme,
) {
    if data.len() < 2 {
        return;
    }

    let kde = Kde::new().n_points(n_points);

    // Evaluate the KDE over the column data range
    let step = (d_hi - d_lo) / (n_points.max(2) - 1) as f64;
    let eval_xs: Vec<f64> = (0..n_points).map(|i| d_lo + i as f64 * step).collect();
    let densities = kde.evaluate(data, &eval_xs);

    let max_density = densities.iter().copied().fold(0.0_f64, f64::max);
    if max_density <= 0.0 {
        return;
    }

    // Build a PlotArea for mapping and a PlotBuffer for Braille
    let pa = PlotArea {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height,
        x_lo: d_lo,
        x_hi: d_hi,
        y_lo: 0.0,
        y_hi: max_density,
        area: Rect::new(area.x, area.y, area.width, area.height),
    };

    let mut pb = PlotBuffer::new(area);

    for i in 0..eval_xs.len().saturating_sub(1) {
        let sx0 = pa.screen_x(eval_xs[i]);
        let sy0 = pa.screen_y(densities[i]);
        let sx1 = pa.screen_x(eval_xs[i + 1]);
        let sy1 = pa.screen_y(densities[i + 1]);
        pb.draw_line(sx0, sy0, sx1, sy1, color, &pa, Z_DATA);
    }

    pb.composite(buf);
}

// ---------------------------------------------------------------------------
// Scatter rendering
// ---------------------------------------------------------------------------

/// Render a scatter plot of (x_data, y_data) inside the given cell area.
#[allow(clippy::too_many_arguments)]
fn render_scatter(
    buf: &mut Buffer,
    area: Rect,
    x_data: &[f64],
    y_data: &[f64],
    x_bounds: (f64, f64),
    y_bounds: (f64, f64),
    marker: MarkerShape,
    color: Color,
) {
    if x_data.is_empty() || y_data.is_empty() || area.width < 3 || area.height < 3 {
        return;
    }

    let (x_lo, x_hi) = x_bounds;
    let (y_lo, y_hi) = y_bounds;
    if x_hi <= x_lo || y_hi <= y_lo {
        return;
    }

    // Inner area (1-char border for frame)
    let inner = Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    if inner.width < 1 || inner.height < 1 {
        return;
    }

    let len = x_data.len().min(y_data.len());
    let mut pb = PlotBuffer::new(inner);

    let marker_ch = marker.char();

    for i in 0..len {
        let x = x_data[i];
        let y = y_data[i];
        if !x.is_finite() || !y.is_finite() {
            continue;
        }

        let sx = data_to_screen(
            x,
            x_lo,
            x_hi,
            inner.x as f64,
            (inner.x + inner.width - 1) as f64,
        );
        let sy = data_to_screen(
            y,
            y_lo,
            y_hi,
            (inner.y + inner.height - 1) as f64,
            inner.y as f64,
        );

        let xi = sx.round() as u16;
        let yi = sy.round() as u16;

        if xi >= inner.x
            && xi < inner.x + inner.width
            && yi >= inner.y
            && yi < inner.y + inner.height
        {
            pb.set_char(xi, yi, marker_ch, color, Z_MARKER);
        }
    }

    pb.composite(buf);
}

// ---------------------------------------------------------------------------
// Cell border
// ---------------------------------------------------------------------------

/// Draw a thin border around a sub-panel cell.
fn draw_cell_border(buf: &mut Buffer, area: Rect, color: Color, theme: &Theme) {
    if area.width < 2 || area.height < 2 {
        return;
    }

    let chars = &theme.chars;
    let x0 = area.x;
    let y0 = area.y;
    let x1 = area.x + area.width - 1;
    let y1 = area.y + area.height - 1;

    // Corners
    if in_buf(buf, x0, y0) {
        buf[(x0, y0)]
            .set_char(chars.border.top_left)
            .set_fg(color);
    }
    if in_buf(buf, x1, y0) {
        buf[(x1, y0)]
            .set_char(chars.border.top_right)
            .set_fg(color);
    }
    if in_buf(buf, x0, y1) {
        buf[(x0, y1)]
            .set_char(chars.border.bottom_left)
            .set_fg(color);
    }
    if in_buf(buf, x1, y1) {
        buf[(x1, y1)]
            .set_char(chars.border.bottom_right)
            .set_fg(color);
    }

    // Top and bottom edges
    for x in (x0 + 1)..x1 {
        if in_buf(buf, x, y0) {
            buf[(x, y0)]
                .set_char(chars.border.horizontal)
                .set_fg(color);
        }
        if in_buf(buf, x, y1) {
            buf[(x, y1)]
                .set_char(chars.border.horizontal)
                .set_fg(color);
        }
    }

    // Left and right edges
    for y in (y0 + 1)..y1 {
        if in_buf(buf, x0, y) {
            buf[(x0, y)]
                .set_char(chars.border.vertical)
                .set_fg(color);
        }
        if in_buf(buf, x1, y) {
            buf[(x1, y)]
                .set_char(chars.border.vertical)
                .set_fg(color);
        }
    }
}

/// Check that (x, y) is within the buffer's area.
#[inline]
fn in_buf(buf: &Buffer, x: u16, y: u16) -> bool {
    let a = buf.area();
    x >= a.x && x < a.x + a.width && y >= a.y && y < a.y + a.height
}
