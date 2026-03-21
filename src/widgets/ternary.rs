//! Ternary (triangle) plot widget for three-component composition data.
//!
//! Plots data where each point is a triplet (a, b, c) summing to a constant,
//! mapped onto an equilateral triangle. Useful for mixture compositions,
//! mineral classification, and phase diagrams.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::ternary::{TernaryPlot, TernaryData};
//! use ratatui::style::Color;
//! use ratatui_plt::style::MarkerShape;
//!
//! let data = TernaryData::new("samples")
//!     .points(vec![(0.5, 0.3, 0.2), (0.1, 0.8, 0.1), (0.33, 0.33, 0.34)])
//!     .color(Color::Cyan)
//!     .marker(MarkerShape::FilledCircle);
//! let plot = TernaryPlot::new()
//!     .dataset(data)
//!     .corner_labels("A", "B", "C")
//!     .title("Ternary Composition");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::style::MarkerShape;
use crate::theme::Theme;

/// A dataset for the ternary plot.
#[derive(Clone, Debug)]
pub struct TernaryData {
    /// Dataset label.
    pub label: String,
    /// Ternary coordinates: (a, b, c) triplets.
    pub points: Vec<(f64, f64, f64)>,
    /// Point color.
    pub color: Color,
    /// Marker shape.
    pub marker: MarkerShape,
}

impl TernaryData {
    /// Create a new dataset with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            points: Vec::new(),
            color: Color::White,
            marker: MarkerShape::FilledCircle,
        }
    }

    /// Set the data points as (a, b, c) triplets.
    pub fn points(mut self, points: Vec<(f64, f64, f64)>) -> Self {
        self.points = points;
        self
    }

    /// Set the point color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the marker shape.
    pub fn marker(mut self, marker: MarkerShape) -> Self {
        self.marker = marker;
        self
    }
}

/// A ternary (triangle) plot widget.
///
/// Renders scatter data on an equilateral triangle coordinate system.
/// The three corners represent pure components, and interior points
/// represent mixtures.
pub struct TernaryPlot {
    datasets: Vec<TernaryData>,
    corner_labels: (String, String, String),
    title: Option<String>,
    show_grid: bool,
    grid_divisions: usize,
    show_tick_labels: bool,
    theme: Theme,
}

impl Default for TernaryPlot {
    fn default() -> Self {
        Self {
            datasets: Vec::new(),
            corner_labels: ("A".into(), "B".into(), "C".into()),
            title: None,
            show_grid: true,
            grid_divisions: 5,
            show_tick_labels: false,
            theme: Theme::get_default(),
        }
    }
}

impl TernaryPlot {
    /// Create a new empty ternary plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a dataset.
    pub fn dataset(mut self, data: TernaryData) -> Self {
        self.datasets.push(data);
        self
    }

    /// Add multiple datasets.
    pub fn datasets(mut self, data: Vec<TernaryData>) -> Self {
        self.datasets.extend(data);
        self
    }

    /// Set the corner labels (bottom-left, bottom-right, top).
    pub fn corner_labels(
        mut self,
        a: impl Into<String>,
        b: impl Into<String>,
        c: impl Into<String>,
    ) -> Self {
        self.corner_labels = (a.into(), b.into(), c.into());
        self
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Whether to show interior grid lines.
    pub fn show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    /// Number of grid divisions along each edge.
    pub fn grid_divisions(mut self, n: usize) -> Self {
        self.grid_divisions = n.max(2);
        self
    }

    /// Whether to show percentage tick labels along triangle edges.
    pub fn show_tick_labels(mut self, show: bool) -> Self {
        self.show_tick_labels = show;
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

/// Convert ternary coordinates (a, b, c) to Cartesian (x, y) in [0, 1] range.
/// a = bottom-left corner, b = bottom-right corner, c = top corner.
/// x = 0.5 * (2*b + c) / (a + b + c)
/// y = (sqrt(3)/2) * c / (a + b + c)
fn ternary_to_cartesian(a: f64, b: f64, c: f64) -> (f64, f64) {
    let sum = a + b + c;
    if sum <= 0.0 {
        return (0.5, 0.0);
    }
    let x = 0.5 * (2.0 * b + c) / sum;
    let y = (3.0_f64.sqrt() / 2.0) * c / sum;
    (x, y)
}

impl Widget for &TernaryPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 6 {
            return;
        }

        // Reserve space for title and bottom labels
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let bottom_margin: u16 = 2; // corner labels + padding
        let top_margin: u16 = title_height + 1; // title + top corner label
        let side_margin: u16 = 4; // space for corner labels

        let py = area.y + top_margin;
        let ph = area.height.saturating_sub(top_margin + bottom_margin);
        let px = area.x + side_margin;
        let pw = area.width.saturating_sub(side_margin * 2);

        if ph < 3 || pw < 6 {
            return;
        }

        // Draw title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        // Triangle geometry in the ternary Cartesian space:
        // Bottom-left (a=1,b=0,c=0) -> (0, 0)
        // Bottom-right (a=0,b=1,c=0) -> (1, 0)
        // Top (a=0,b=0,c=1) -> (0.5, sqrt(3)/2)
        let tri_height = 3.0_f64.sqrt() / 2.0;

        // Map ternary Cartesian (x in [0,1], y in [0, tri_height]) to screen coords.
        // Account for terminal cell aspect ratio (~2:1 height:width)
        let scale_x = pw as f64;
        let scale_y = ph as f64 / tri_height;

        // Use the smaller scale to fit the triangle
        let cell_aspect = 0.5;
        let eff_scale = scale_y.min(scale_x * cell_aspect / 1.0);
        let screen_w = eff_scale / cell_aspect;
        let screen_h = eff_scale * tri_height;

        // Offset to center the triangle
        let off_x = px as f64 + (pw as f64 - screen_w) / 2.0;
        let off_y = py as f64 + (ph as f64 - screen_h) / 2.0;

        let to_screen = |tx: f64, ty: f64| -> (f64, f64) {
            let sx = off_x + tx * screen_w;
            // Invert y: screen y increases downward
            let sy = off_y + screen_h - ty * (screen_h / tri_height);
            (sx, sy)
        };

        // Triangle corners in screen coords
        let (sx_bl, sy_bl) = to_screen(0.0, 0.0); // bottom-left
        let (sx_br, sy_br) = to_screen(1.0, 0.0); // bottom-right
        let (sx_top, sy_top) = to_screen(0.5, tri_height); // top

        let clip = TernaryClip { area, py, ph };

        // Draw triangle edges using Bresenham-style line drawing
        draw_screen_line(
            buf,
            sx_bl,
            sy_bl,
            sx_br,
            sy_br,
            self.theme.axis_color,
            &clip,
        );
        draw_screen_line(
            buf,
            sx_br,
            sy_br,
            sx_top,
            sy_top,
            self.theme.axis_color,
            &clip,
        );
        draw_screen_line(
            buf,
            sx_top,
            sy_top,
            sx_bl,
            sy_bl,
            self.theme.axis_color,
            &clip,
        );

        // Draw grid lines
        if self.show_grid {
            let n = self.grid_divisions;
            for i in 1..n {
                let frac = i as f64 / n as f64;

                // Lines parallel to bottom edge (constant c)
                let (x0, y0) = ternary_to_cartesian(1.0 - frac, 0.0, frac);
                let (x1, y1) = ternary_to_cartesian(0.0, 1.0 - frac, frac);
                let (sx0, sy0) = to_screen(x0, y0);
                let (sx1, sy1) = to_screen(x1, y1);
                draw_screen_line(buf, sx0, sy0, sx1, sy1, self.theme.grid_color, &clip);

                // Lines parallel to left edge (constant b)
                let (x0, y0) = ternary_to_cartesian(1.0 - frac, frac, 0.0);
                let (x1, y1) = ternary_to_cartesian(0.0, frac, 1.0 - frac);
                let (sx0, sy0) = to_screen(x0, y0);
                let (sx1, sy1) = to_screen(x1, y1);
                draw_screen_line(buf, sx0, sy0, sx1, sy1, self.theme.grid_color, &clip);

                // Lines parallel to right edge (constant a)
                let (x0, y0) = ternary_to_cartesian(frac, 1.0 - frac, 0.0);
                let (x1, y1) = ternary_to_cartesian(frac, 0.0, 1.0 - frac);
                let (sx0, sy0) = to_screen(x0, y0);
                let (sx1, sy1) = to_screen(x1, y1);
                draw_screen_line(buf, sx0, sy0, sx1, sy1, self.theme.grid_color, &clip);
            }
        }

        // Draw tick labels along triangle edges
        if self.show_tick_labels && self.show_grid {
            let n = self.grid_divisions;
            for i in 1..n {
                let frac = i as f64 / n as f64;
                let pct = (frac * 100.0).round() as u32;
                let label = format!("{pct}%");

                // Bottom edge: ticks for component A (bottom-left corner value decreases left to right)
                // Position along the bottom edge at fraction frac from bottom-left
                let (bx, by) = ternary_to_cartesian(1.0 - frac, frac, 0.0);
                let (sbx, sby) = to_screen(bx, by);
                // Place label below the bottom edge
                let lx = (sbx - label.len() as f64 / 2.0).round().max(area.x as f64) as u16;
                let ly = (sby + 1.0).round() as u16;
                if ly < area.y + area.height {
                    for (j, ch) in label.chars().enumerate() {
                        let x = lx + j as u16;
                        if x >= area.x && x < area.x + area.width {
                            buf[(x, ly)].set_char(ch).set_fg(self.theme.grid_color);
                        }
                    }
                }

                // Left edge: ticks for component C (top corner value increases upward)
                let (lx2, ly2) = ternary_to_cartesian(1.0 - frac, 0.0, frac);
                let (slx, sly) = to_screen(lx2, ly2);
                let lx_pos = (slx - label.len() as f64 - 1.0).round().max(area.x as f64) as u16;
                let ly_pos = sly.round() as u16;
                if ly_pos >= area.y && ly_pos < area.y + area.height {
                    for (j, ch) in label.chars().enumerate() {
                        let x = lx_pos + j as u16;
                        if x >= area.x && x < area.x + area.width {
                            buf[(x, ly_pos)].set_char(ch).set_fg(self.theme.grid_color);
                        }
                    }
                }

                // Right edge: ticks for component B (bottom-right corner value increases upward)
                let (rx, ry) = ternary_to_cartesian(0.0, 1.0 - frac, frac);
                let (srx, sry) = to_screen(rx, ry);
                let rx_pos = (srx + 1.0).round() as u16;
                let ry_pos = sry.round() as u16;
                if ry_pos >= area.y && ry_pos < area.y + area.height {
                    for (j, ch) in label.chars().enumerate() {
                        let x = rx_pos + j as u16;
                        if x >= area.x && x < area.x + area.width {
                            buf[(x, ry_pos)].set_char(ch).set_fg(self.theme.grid_color);
                        }
                    }
                }
            }
        }

        // Draw data points
        for dataset in &self.datasets {
            for &(a, b, c) in &dataset.points {
                let (tx, ty) = ternary_to_cartesian(a, b, c);
                let (sx, sy) = to_screen(tx, ty);
                let xi = sx.round() as u16;
                let yi = sy.round() as u16;
                if xi >= area.x && xi < area.x + area.width && yi >= py && yi < py + ph {
                    buf[(xi, yi)]
                        .set_char(dataset.marker.char())
                        .set_fg(dataset.color);
                }
            }
        }

        // Draw corner labels
        // Bottom-left (A)
        {
            let label = &self.corner_labels.0;
            let lx = (sx_bl - label.len() as f64 / 2.0)
                .round()
                .max(area.x as f64) as u16;
            let ly = (sy_bl + 1.0).round() as u16;
            if ly < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let x = lx + j as u16;
                    if x < area.x + area.width {
                        buf[(x, ly)].set_char(ch).set_fg(self.theme.foreground);
                    }
                }
            }
        }

        // Bottom-right (B)
        {
            let label = &self.corner_labels.1;
            let lx = (sx_br - label.len() as f64 / 2.0)
                .round()
                .max(area.x as f64) as u16;
            let ly = (sy_br + 1.0).round() as u16;
            if ly < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let x = lx + j as u16;
                    if x < area.x + area.width {
                        buf[(x, ly)].set_char(ch).set_fg(self.theme.foreground);
                    }
                }
            }
        }

        // Top (C)
        {
            let label = &self.corner_labels.2;
            let lx = (sx_top - label.len() as f64 / 2.0)
                .round()
                .max(area.x as f64) as u16;
            let ly = (sy_top - 1.0).round().max(area.y as f64) as u16;
            if ly >= area.y && ly < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let x = lx + j as u16;
                    if x < area.x + area.width {
                        buf[(x, ly)].set_char(ch).set_fg(self.theme.foreground);
                    }
                }
            }
        }
    }
}

/// Clip region for ternary line drawing.
struct TernaryClip {
    area: Rect,
    py: u16,
    ph: u16,
}

/// Draw a line between two screen coordinates using character-level Bresenham.
fn draw_screen_line(
    buf: &mut Buffer,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
    clip: &TernaryClip,
) {
    let mut ix = x0.round() as i32;
    let mut iy = y0.round() as i32;
    let ix1 = x1.round() as i32;
    let iy1 = y1.round() as i32;

    let dx = (ix1 - ix).abs();
    let dy = -(iy1 - iy).abs();
    let sx = if ix < ix1 { 1 } else { -1 };
    let sy = if iy < iy1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if ix >= clip.area.x as i32
            && ix < (clip.area.x + clip.area.width) as i32
            && iy >= clip.py as i32
            && iy < (clip.py + clip.ph) as i32
        {
            // Choose line character based on direction
            let ch = if dx > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx * 2 {
                '│'
            } else if (ix1 > ix) == (iy1 > iy) {
                '╲'
            } else {
                '╱'
            };
            buf[(ix as u16, iy as u16)].set_char(ch).set_fg(color);
        }

        if ix == ix1 && iy == iy1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            ix += sx;
        }
        if e2 <= dx {
            err += dx;
            iy += sy;
        }
    }
}
