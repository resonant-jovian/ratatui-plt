//! Multi-panel subplot grid layout (GridSpec-like).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

/// A callback that renders a widget into a given area.
pub type RenderFn = Box<dyn Fn(Rect, &mut Buffer)>;

/// A multi-panel layout widget for combining multiple plots.
///
/// Supports non-uniform row/column sizes via width_ratios and height_ratios.
///
/// # Example
///
/// ```
/// use ratatui_sim::widgets::multi_panel::MultiPanel;
///
/// let panel = MultiPanel::new(2, 2)
///     .width_ratios(vec![2.0, 1.0])  // Left column is twice as wide
///     .height_ratios(vec![1.0, 1.0]);
/// ```
pub struct MultiPanel {
    rows: usize,
    cols: usize,
    width_ratios: Vec<f64>,
    height_ratios: Vec<f64>,
    gap: u16,
    /// Render callbacks indexed by (row * cols + col).
    panels: Vec<Option<RenderFn>>,
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
        }
    }

    /// Set column width ratios (must have `cols` elements).
    pub fn width_ratios(mut self, ratios: Vec<f64>) -> Self {
        assert_eq!(ratios.len(), self.cols, "width_ratios length must equal cols");
        self.width_ratios = ratios;
        self
    }

    /// Set row height ratios (must have `rows` elements).
    pub fn height_ratios(mut self, ratios: Vec<f64>) -> Self {
        assert_eq!(ratios.len(), self.rows, "height_ratios length must equal rows");
        self.height_ratios = ratios;
        self
    }

    /// Set the gap between panels in characters.
    pub fn gap(mut self, g: u16) -> Self {
        self.gap = g;
        self
    }

    /// Set the render callback for a specific panel position.
    pub fn panel(mut self, row: usize, col: usize, render: impl Fn(Rect, &mut Buffer) + 'static) -> Self {
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
}

impl Widget for &MultiPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        // Compute row heights
        let total_h_ratio: f64 = self.height_ratios.iter().sum();
        let available_h = area.height.saturating_sub(self.gap * (self.rows.saturating_sub(1)) as u16);
        let row_heights: Vec<u16> = self
            .height_ratios
            .iter()
            .map(|r| (r / total_h_ratio * available_h as f64).round() as u16)
            .collect();

        // Compute column widths
        let total_w_ratio: f64 = self.width_ratios.iter().sum();
        let available_w = area.width.saturating_sub(self.gap * (self.cols.saturating_sub(1)) as u16);
        let col_widths: Vec<u16> = self
            .width_ratios
            .iter()
            .map(|r| (r / total_w_ratio * available_w as f64).round() as u16)
            .collect();

        let mut y_offset = area.y;
        #[allow(clippy::needless_range_loop)]
        for row in 0..self.rows {
            let mut x_offset = area.x;
            for col in 0..self.cols {
                let idx = row * self.cols + col;
                let panel_area = Rect::new(
                    x_offset,
                    y_offset,
                    col_widths[col].min(area.x + area.width - x_offset),
                    row_heights[row].min(area.y + area.height - y_offset),
                );

                if let Some(ref render_fn) = self.panels[idx] {
                    render_fn(panel_area, buf);
                }

                x_offset += col_widths[col] + self.gap;
            }
            y_offset += row_heights[row] + self.gap;
        }
    }
}
