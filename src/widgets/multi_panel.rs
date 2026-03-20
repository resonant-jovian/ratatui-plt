//! Multi-panel subplot grid layout (GridSpec-like).

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

/// A multi-panel layout widget for combining multiple plots.
///
/// Supports non-uniform row/column sizes via width_ratios and height_ratios,
/// cell spanning, and a super-title above the entire grid.
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
    /// Panels that span multiple cells.
    span_panels: Vec<SpanPanel>,
    theme: Theme,
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
            span_panels: Vec::new(),
            theme: Theme::get_default(),
        }
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

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
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

impl Widget for &MultiPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        // Reserve space for suptitle
        let suptitle_height: u16 = if self.suptitle.is_some() { 1 } else { 0 };
        let grid_area = Rect::new(
            area.x,
            area.y + suptitle_height,
            area.width,
            area.height.saturating_sub(suptitle_height),
        );

        // Draw suptitle
        if let Some(ref title) = self.suptitle {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)]
                        .set_char(ch)
                        .set_style(Style::default().fg(self.theme.foreground));
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
        #[allow(clippy::needless_range_loop)]
        for sp in &self.span_panels {
            let x = col_offsets[sp.col];
            let y = row_offsets[sp.row];
            let mut w: u16 = 0;
            for c in sp.col..sp.col + sp.colspan.min(self.cols - sp.col) {
                w += col_widths[c];
                if c > sp.col {
                    w += self.gap;
                }
            }
            let mut h: u16 = 0;
            for r in sp.row..sp.row + sp.rowspan.min(self.rows - sp.row) {
                h += row_heights[r];
                if r > sp.row {
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
        #[allow(clippy::needless_range_loop)]
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
