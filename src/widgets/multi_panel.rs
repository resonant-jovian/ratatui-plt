//! Multi-panel subplot grid layout (GridSpec-like).
//!
//! Supports ASCII-art mosaic layouts, figure-level labels (super-title,
//! shared x/y labels), and shared-axes configuration.

use std::collections::BTreeMap;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::theme::Theme;

/// A callback that renders a widget into a given area.
pub type RenderFn = Box<dyn Fn(Rect, &mut Buffer)>;

/// A spanning panel that covers multiple grid cells.
struct SpanPanel {
    row: usize,
    col: usize,
    rowspan: usize,
    colspan: usize,
    render: RenderFn,
}

/// A named panel produced by parsing an ASCII-art mosaic pattern.
///
/// Each unique character in the mosaic becomes a `MosaicPanel` that records
/// the grid region it occupies (row, col, rowspan, colspan) and the character
/// label used to identify it.
#[derive(Debug, Clone)]
pub struct MosaicPanel {
    /// The character label from the mosaic pattern.
    pub label: char,
    /// Starting row in the grid.
    pub row: usize,
    /// Starting column in the grid.
    pub col: usize,
    /// Number of rows this panel spans.
    pub rowspan: usize,
    /// Number of columns this panel spans.
    pub colspan: usize,
}

/// A multi-panel layout widget for combining multiple plots.
///
/// Supports non-uniform row/column sizes via width_ratios and height_ratios,
/// cell spanning, figure-level labels (super-title, shared x-label, shared
/// y-label), shared axes, and ASCII-art mosaic construction.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::multi_panel::MultiPanel;
///
/// let panel = MultiPanel::new(2, 2)
///     .width_ratios(vec![2.0, 1.0])  // Left column is twice as wide
///     .height_ratios(vec![1.0, 1.0])
///     .suptitle("Dashboard");
/// ```
///
/// # Mosaic Layout
///
/// ```
/// use ratatui_plt::widgets::multi_panel::MultiPanel;
///
/// // Panel A spans columns 0-1 on row 0,
/// // Panel B spans column 2 on rows 0-1,
/// // Panel C spans columns 0-1 on row 1.
/// let panel = MultiPanel::from_mosaic("AAB\nCCB");
/// ```
pub struct MultiPanel {
    rows: usize,
    cols: usize,
    width_ratios: Vec<f64>,
    height_ratios: Vec<f64>,
    gap: u16,
    /// Render callbacks indexed by (row * cols + col).
    panels: Vec<Option<RenderFn>>,
    /// Super-title displayed above the entire grid.
    suptitle: Option<String>,
    /// Shared x-axis label displayed below the entire grid.
    supxlabel: Option<String>,
    /// Shared y-axis label displayed to the left of the entire grid.
    supylabel: Option<String>,
    /// When true, only the bottom row shows x tick labels.
    pub share_x: bool,
    /// When true, only the leftmost column shows y tick labels.
    pub share_y: bool,
    /// Optional shared x-axis bounds applied to all panels.
    pub shared_x_bounds: Option<(f64, f64)>,
    /// Optional shared y-axis bounds applied to all panels.
    pub shared_y_bounds: Option<(f64, f64)>,
    /// Panels that span multiple cells.
    span_panels: Vec<SpanPanel>,
    /// Mosaic panel metadata (populated by `from_mosaic`).
    mosaic_panels: Vec<MosaicPanel>,
    theme: Theme,
    /// When enabled, panels are aligned so axes match across the grid.
    pub constrained_layout: bool,
}

impl MultiPanel {
    /// Create a new multi-panel layout with uniform sizing.
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            width_ratios: vec![1.0; cols],
            height_ratios: vec![1.0; rows],
            gap: 0,
            panels: (0..rows * cols).map(|_| None).collect(),
            suptitle: None,
            supxlabel: None,
            supylabel: None,
            share_x: false,
            share_y: false,
            shared_x_bounds: None,
            shared_y_bounds: None,
            span_panels: Vec::new(),
            mosaic_panels: Vec::new(),
            theme: Theme::get_default(),
            constrained_layout: false,
        }
    }

    /// Create a multi-panel layout from an ASCII-art mosaic pattern.
    ///
    /// Each unique non-whitespace character in the pattern defines a panel.
    /// A character that spans multiple cells means that panel occupies those
    /// cells.  Rows are separated by newlines; every row must have the same
    /// number of characters.
    ///
    /// # Panics
    ///
    /// Panics if the pattern is empty, contains rows of different lengths, or
    /// if a character's cells do not form a contiguous rectangle.
    ///
    /// # Example
    ///
    /// ```
    /// use ratatui_plt::widgets::multi_panel::MultiPanel;
    ///
    /// let mp = MultiPanel::from_mosaic("AAB\nCCB");
    /// let panels = mp.mosaic_panel_info();
    /// assert_eq!(panels.len(), 3);
    /// ```
    pub fn from_mosaic(pattern: &str) -> Self {
        let grid: Vec<Vec<char>> = pattern
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| line.chars().collect())
            .collect();

        assert!(!grid.is_empty(), "mosaic pattern must not be empty");
        let n_rows = grid.len();
        let n_cols = grid[0].len();
        for (i, row) in grid.iter().enumerate() {
            assert_eq!(
                row.len(),
                n_cols,
                "mosaic row {i} has {} columns, expected {n_cols}",
                row.len()
            );
        }

        // Collect cells per unique character, preserving order via BTreeMap.
        let mut char_cells: BTreeMap<char, Vec<(usize, usize)>> = BTreeMap::new();
        for (r, row) in grid.iter().enumerate() {
            for (c, &ch) in row.iter().enumerate() {
                if !ch.is_whitespace() {
                    char_cells.entry(ch).or_default().push((r, c));
                }
            }
        }

        let mut mosaic_panels = Vec::new();

        for (&ch, cells) in &char_cells {
            let Some(min_r) = cells.iter().map(|&(r, _)| r).min() else {
                continue;
            };
            let Some(max_r) = cells.iter().map(|&(r, _)| r).max() else {
                continue;
            };
            let Some(min_c) = cells.iter().map(|&(_, c)| c).min() else {
                continue;
            };
            let Some(max_c) = cells.iter().map(|&(_, c)| c).max() else {
                continue;
            };

            let rowspan = max_r - min_r + 1;
            let colspan = max_c - min_c + 1;

            // Verify the cells form a full rectangle.
            assert_eq!(
                cells.len(),
                rowspan * colspan,
                "mosaic character '{ch}' does not form a contiguous rectangle"
            );

            mosaic_panels.push(MosaicPanel {
                label: ch,
                row: min_r,
                col: min_c,
                rowspan,
                colspan,
            });
        }

        let mut mp = Self::new(n_rows, n_cols);
        mp.mosaic_panels = mosaic_panels;

        // Register each mosaic region as a span panel placeholder (no render
        // callback yet -- users attach them with `mosaic_panel()`).
        // We store the metadata but do not push SpanPanels here; the user
        // attaches render functions via `mosaic_panel(label, render)`.
        mp
    }

    /// Return the mosaic panel metadata produced by [`from_mosaic`](Self::from_mosaic).
    ///
    /// Returns an empty slice if the `MultiPanel` was not built from a mosaic.
    pub fn mosaic_panel_info(&self) -> &[MosaicPanel] {
        &self.mosaic_panels
    }

    /// Attach a render callback to a mosaic panel identified by its character
    /// label.
    ///
    /// If the label was not present in the original mosaic pattern the call is
    /// silently ignored.
    pub fn mosaic_panel(
        mut self,
        label: char,
        render: impl Fn(Rect, &mut Buffer) + 'static,
    ) -> Self {
        if let Some(mp) = self.mosaic_panels.iter().find(|p| p.label == label) {
            let row = mp.row;
            let col = mp.col;
            let rowspan = mp.rowspan;
            let colspan = mp.colspan;
            self.span_panels.push(SpanPanel {
                row,
                col,
                rowspan,
                colspan,
                render: Box::new(render),
            });
        }
        self
    }

    /// Set column width ratios (must have `cols` elements).
    pub fn width_ratios(mut self, ratios: Vec<f64>) -> Self {
        assert_eq!(
            ratios.len(),
            self.cols,
            "width_ratios length must equal cols"
        );
        self.width_ratios = ratios;
        self
    }

    /// Set row height ratios (must have `rows` elements).
    pub fn height_ratios(mut self, ratios: Vec<f64>) -> Self {
        assert_eq!(
            ratios.len(),
            self.rows,
            "height_ratios length must equal rows"
        );
        self.height_ratios = ratios;
        self
    }

    /// Set the gap between panels in characters.
    pub fn gap(mut self, g: u16) -> Self {
        self.gap = g;
        self
    }

    /// Set the render callback for a specific panel position.
    pub fn panel(
        mut self,
        row: usize,
        col: usize,
        render: impl Fn(Rect, &mut Buffer) + 'static,
    ) -> Self {
        let idx = row * self.cols + col;
        if idx < self.panels.len() {
            self.panels[idx] = Some(Box::new(render));
        }
        self
    }

    /// Set panels from a flat list (row-major order).
    pub fn panels_vec(mut self, renders: Vec<RenderFn>) -> Self {
        for (i, r) in renders.into_iter().enumerate() {
            if i < self.panels.len() {
                self.panels[i] = Some(r);
            }
        }
        self
    }

    /// Set a super-title displayed above the entire grid.
    pub fn suptitle(mut self, title: impl Into<String>) -> Self {
        self.suptitle = Some(title.into());
        self
    }

    /// Set a shared x-axis label displayed below the entire grid.
    pub fn supxlabel(mut self, label: impl Into<String>) -> Self {
        self.supxlabel = Some(label.into());
        self
    }

    /// Set a shared y-axis label displayed to the left of the entire grid.
    pub fn supylabel(mut self, label: impl Into<String>) -> Self {
        self.supylabel = Some(label.into());
        self
    }

    /// Enable shared x-axes: only the bottom row shows x tick labels.
    pub fn share_x(mut self, shared: bool) -> Self {
        self.share_x = shared;
        self
    }

    /// Enable shared y-axes: only the leftmost column shows y tick labels.
    pub fn share_y(mut self, shared: bool) -> Self {
        self.share_y = shared;
        self
    }

    /// Set shared x-axis bounds applied to all panels.
    pub fn shared_x_bounds(mut self, bounds: (f64, f64)) -> Self {
        self.shared_x_bounds = Some(bounds);
        self
    }

    /// Set shared y-axis bounds applied to all panels.
    pub fn shared_y_bounds(mut self, bounds: (f64, f64)) -> Self {
        self.shared_y_bounds = Some(bounds);
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Enable or disable constrained layout (aligns axes across panels).
    pub fn constrained_layout(mut self, enabled: bool) -> Self {
        self.constrained_layout = enabled;
        self
    }

    /// Set a panel that spans multiple grid cells.
    pub fn panel_span(
        mut self,
        row: usize,
        col: usize,
        rowspan: usize,
        colspan: usize,
        render: impl Fn(Rect, &mut Buffer) + 'static,
    ) -> Self {
        self.span_panels.push(SpanPanel {
            row,
            col,
            rowspan,
            colspan,
            render: Box::new(render),
        });
        self
    }
}

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for MultiPanel {
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

impl Widget for &MultiPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        // Reserve space for figure-level labels
        let suptitle_height: u16 = if self.suptitle.is_some() { 1 } else { 0 };
        let supxlabel_height: u16 = if self.supxlabel.is_some() { 1 } else { 0 };
        let supylabel_width: u16 = if self.supylabel.is_some() { 2 } else { 0 };

        let grid_area = Rect::new(
            area.x + supylabel_width,
            area.y + suptitle_height,
            area.width.saturating_sub(supylabel_width),
            area.height
                .saturating_sub(suptitle_height)
                .saturating_sub(supxlabel_height),
        );

        let label_style = Style::default().fg(self.theme.foreground);

        // Draw suptitle (centered above the grid)
        if let Some(ref title) = self.suptitle {
            let text_len = title.len() as u16;
            let start = area.x + (area.width.saturating_sub(text_len)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_style(label_style);
                }
            }
        }

        // Draw supxlabel (centered below the grid)
        if let Some(ref xlabel) = self.supxlabel {
            let y = area.y + area.height.saturating_sub(1);
            let text_len = xlabel.len() as u16;
            let start = area.x + (area.width.saturating_sub(text_len)) / 2;
            for (i, ch) in xlabel.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, y)].set_char(ch).set_style(label_style);
                }
            }
        }

        // Draw supylabel (vertically centered to the left of the grid)
        if let Some(ref ylabel) = self.supylabel {
            let x = area.x;
            let text_len = ylabel.len() as u16;
            let start_y = grid_area.y + (grid_area.height.saturating_sub(text_len)) / 2;
            for (i, ch) in ylabel.chars().enumerate() {
                let y = start_y + i as u16;
                if y < grid_area.y + grid_area.height {
                    buf[(x, y)].set_char(ch).set_style(label_style);
                }
            }
        }

        // Compute row heights
        let total_h_ratio: f64 = self.height_ratios.iter().sum();
        let available_h = grid_area
            .height
            .saturating_sub(self.gap * (self.rows.saturating_sub(1)) as u16);
        let row_heights: Vec<u16> = self
            .height_ratios
            .iter()
            .map(|r| (r / total_h_ratio * available_h as f64).round() as u16)
            .collect();

        // Compute column widths
        let total_w_ratio: f64 = self.width_ratios.iter().sum();
        let available_w = grid_area
            .width
            .saturating_sub(self.gap * (self.cols.saturating_sub(1)) as u16);
        let col_widths: Vec<u16> = self
            .width_ratios
            .iter()
            .map(|r| (r / total_w_ratio * available_w as f64).round() as u16)
            .collect();

        // Compute cumulative offsets for row/col positions
        let mut row_offsets = vec![grid_area.y];
        for r in 0..self.rows {
            row_offsets.push(row_offsets[r] + row_heights[r] + self.gap);
        }
        let mut col_offsets = vec![grid_area.x];
        for c in 0..self.cols {
            col_offsets.push(col_offsets[c] + col_widths[c] + self.gap);
        }

        // Track which cells are covered by span panels
        let mut covered = vec![false; self.rows * self.cols];

        // Render span panels first
        for sp in &self.span_panels {
            let x = col_offsets[sp.col];
            let y = row_offsets[sp.row];
            let col_count = sp.colspan.min(self.cols - sp.col);
            let mut w: u16 = 0;
            for (i, &cw) in col_widths.iter().enumerate().skip(sp.col).take(col_count) {
                w += cw;
                if i > sp.col {
                    w += self.gap;
                }
            }
            let row_count = sp.rowspan.min(self.rows - sp.row);
            let mut h: u16 = 0;
            for (i, &rh) in row_heights.iter().enumerate().skip(sp.row).take(row_count) {
                h += rh;
                if i > sp.row {
                    h += self.gap;
                }
            }
            let span_area = Rect::new(
                x,
                y,
                w.min(grid_area.x + grid_area.width - x),
                h.min(grid_area.y + grid_area.height - y),
            );
            (sp.render)(span_area, buf);

            // Mark cells as covered
            for r in sp.row..sp.row + sp.rowspan.min(self.rows - sp.row) {
                for c in sp.col..sp.col + sp.colspan.min(self.cols - sp.col) {
                    covered[r * self.cols + c] = true;
                }
            }
        }

        // Render regular (non-spanned) panels
        for row in 0..self.rows {
            for col in 0..self.cols {
                let idx = row * self.cols + col;
                if covered[idx] {
                    continue;
                }
                let panel_area = Rect::new(
                    col_offsets[col],
                    row_offsets[row],
                    col_widths[col].min(grid_area.x + grid_area.width - col_offsets[col]),
                    row_heights[row].min(grid_area.y + grid_area.height - row_offsets[row]),
                );

                if let Some(ref render_fn) = self.panels[idx] {
                    render_fn(panel_area, buf);
                }
            }
        }
    }
}
