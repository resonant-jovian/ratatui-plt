//! Shared plot frame rendering framework.
//!
//! `PlotFrame` extracts the duplicated axis/title/grid/tick rendering code
//! from all 2D widgets into a single integration point. This eliminates ~300
//! lines of duplicated code across 22 widgets and provides a single place to
//! add features like reference lines, minor grids, and spine control.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::transform::{apply_aspect_ratio, data_to_screen};

/// Dash style for reference lines.
#[derive(Clone, Debug, Default)]
pub enum RefLineDash {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

/// A reference line or filled span drawn across the plot area.
///
/// # Example
///
/// ```
/// use ratatui_plt::frame::ReferenceLine;
/// use ratatui::style::Color;
///
/// let zero_line = ReferenceLine::hline(0.0, Color::Gray);
/// let threshold = ReferenceLine::hspan(0.5, 1.5, Color::Rgb(50, 50, 80));
/// ```
#[derive(Clone, Debug)]
pub enum ReferenceLine {
    /// Horizontal line at a y value (`axhline`).
    Horizontal {
        y: f64,
        color: Color,
        dash: RefLineDash,
    },
    /// Vertical line at an x value (`axvline`).
    Vertical {
        x: f64,
        color: Color,
        dash: RefLineDash,
    },
    /// Horizontal filled span between two y values (`axhspan`).
    HorizontalSpan { y1: f64, y2: f64, color: Color },
    /// Vertical filled span between two x values (`axvspan`).
    VerticalSpan { x1: f64, x2: f64, color: Color },
}

impl ReferenceLine {
    /// Solid horizontal line at y.
    pub fn hline(y: f64, color: Color) -> Self {
        Self::Horizontal {
            y,
            color,
            dash: RefLineDash::Solid,
        }
    }

    /// Dashed horizontal line at y.
    pub fn hline_dashed(y: f64, color: Color) -> Self {
        Self::Horizontal {
            y,
            color,
            dash: RefLineDash::Dashed,
        }
    }

    /// Solid vertical line at x.
    pub fn vline(x: f64, color: Color) -> Self {
        Self::Vertical {
            x,
            color,
            dash: RefLineDash::Solid,
        }
    }

    /// Dashed vertical line at x.
    pub fn vline_dashed(x: f64, color: Color) -> Self {
        Self::Vertical {
            x,
            color,
            dash: RefLineDash::Dashed,
        }
    }

    /// Horizontal filled span between y1 and y2.
    pub fn hspan(y1: f64, y2: f64, color: Color) -> Self {
        Self::HorizontalSpan { y1, y2, color }
    }

    /// Vertical filled span between x1 and x2.
    pub fn vspan(x1: f64, x2: f64, color: Color) -> Self {
        Self::VerticalSpan { x1, x2, color }
    }
}

/// Computed drawing area returned by [`PlotFrame::render`].
///
/// Contains the screen coordinates and resolved data bounds for widgets
/// to map data points to screen positions.
#[derive(Clone, Debug)]
pub struct PlotArea {
    /// Screen x origin of the drawing area.
    pub x: u16,
    /// Screen y origin of the drawing area.
    pub y: u16,
    /// Width of the drawing area in characters.
    pub width: u16,
    /// Height of the drawing area in characters.
    pub height: u16,
    /// Resolved x data minimum.
    pub x_lo: f64,
    /// Resolved x data maximum.
    pub x_hi: f64,
    /// Resolved y data minimum.
    pub y_lo: f64,
    /// Resolved y data maximum.
    pub y_hi: f64,
    /// The full widget area (for bounds checking).
    pub area: Rect,
}

impl PlotArea {
    /// Map a data x value to a screen x coordinate.
    #[inline]
    pub fn screen_x(&self, data_x: f64) -> f64 {
        data_to_screen(
            data_x,
            self.x_lo,
            self.x_hi,
            self.x as f64,
            (self.x + self.width - 1) as f64,
        )
    }

    /// Map a data y value to a screen y coordinate (y-axis inverted on screen).
    #[inline]
    pub fn screen_y(&self, data_y: f64) -> f64 {
        data_to_screen(
            data_y,
            self.y_lo,
            self.y_hi,
            (self.y + self.height - 1) as f64,
            self.y as f64,
        )
    }

    /// Check if a screen coordinate is within the drawing area.
    #[inline]
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Check if a screen coordinate is within the overall widget area.
    #[inline]
    pub fn in_area(&self, x: u16, y: u16) -> bool {
        x >= self.area.x
            && x < self.area.x + self.area.width
            && y >= self.area.y
            && y < self.area.y + self.area.height
    }

    /// Map a screen x coordinate back to a data x value.
    ///
    /// This is the inverse of [`PlotArea::screen_x`].
    #[inline]
    pub fn data_x_from_screen(&self, screen_x: u16) -> f64 {
        let s_lo = self.x as f64;
        let s_hi = (self.x + self.width - 1) as f64;
        if s_hi == s_lo {
            return self.x_lo;
        }
        self.x_lo + (screen_x as f64 - s_lo) / (s_hi - s_lo) * (self.x_hi - self.x_lo)
    }

    /// Map a screen y coordinate back to a data y value.
    ///
    /// This is the inverse of [`PlotArea::screen_y`]. Screen y-axis is
    /// inverted (top = high y, bottom = low y).
    #[inline]
    pub fn data_y_from_screen(&self, screen_y: u16) -> f64 {
        let s_lo = (self.y + self.height - 1) as f64; // low data = high screen y
        let s_hi = self.y as f64; // high data = low screen y
        if s_lo == s_hi {
            return self.y_lo;
        }
        self.y_lo + (screen_y as f64 - s_lo) / (s_hi - s_lo) * (self.y_hi - self.y_lo)
    }

    /// Find the nearest data point to the given screen coordinates.
    ///
    /// Returns `Some((index, data_x, data_y))` for the closest point,
    /// or `None` if the points slice is empty.
    pub fn nearest_point(
        &self,
        screen_x: u16,
        screen_y: u16,
        points: &[(f64, f64)],
    ) -> Option<(usize, f64, f64)> {
        if points.is_empty() {
            return None;
        }
        let mut best_idx = 0;
        let mut best_dist = f64::INFINITY;
        let sx = screen_x as f64;
        let sy = screen_y as f64;
        for (i, &(dx, dy)) in points.iter().enumerate() {
            let px = self.screen_x(dx);
            let py = self.screen_y(dy);
            let dist = (px - sx) * (px - sx) + (py - sy) * (py - sy);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }
        let (dx, dy) = points[best_idx];
        Some((best_idx, dx, dy))
    }
}

/// Shared rendering framework for 2D plot chrome (axes, title, grid, ticks, labels).
///
/// # Example
///
/// ```ignore
/// // Inside a widget's render method:
/// let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
///     .title(self.title.as_deref())
///     .aspect_ratio(self.aspect_ratio.clone())
///     .spines(self.spines.clone());
///
/// let Some(pa) = frame.render(area, buf, x_lo, x_hi, y_lo, y_hi) else {
///     return;
/// };
/// // Draw widget-specific data using pa.screen_x(), pa.screen_y()...
/// ```
pub struct PlotFrame<'a> {
    title: Option<&'a str>,
    x_axis: &'a Axis,
    y_axis: &'a Axis,
    aspect_ratio: AspectRatio,
    spines: Spines,
    theme: &'a Theme,
    colorbar_width: u16,
    y_label_width: u16,
    reference_lines: &'a [ReferenceLine],
}

impl<'a> PlotFrame<'a> {
    /// Create a new PlotFrame with the given axes and theme.
    pub fn new(x_axis: &'a Axis, y_axis: &'a Axis, theme: &'a Theme) -> Self {
        Self {
            title: None,
            x_axis,
            y_axis,
            aspect_ratio: AspectRatio::Auto,
            spines: Spines::default(),
            theme,
            colorbar_width: 0,
            y_label_width: 8,
            reference_lines: &[],
        }
    }

    /// Set the plot title.
    pub fn title(mut self, title: Option<&'a str>) -> Self {
        self.title = title;
        self
    }

    /// Set the aspect ratio.
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
        self
    }

    /// Set spine visibility.
    pub fn spines(mut self, s: Spines) -> Self {
        self.spines = s;
        self
    }

    /// Reserve width for a colorbar on the right side.
    pub fn colorbar_width(mut self, w: u16) -> Self {
        self.colorbar_width = w;
        self
    }

    /// Set the width reserved for y-axis tick labels (default: 8).
    pub fn y_label_width(mut self, w: u16) -> Self {
        self.y_label_width = w;
        self
    }

    /// Set reference lines to draw.
    pub fn reference_lines(mut self, lines: &'a [ReferenceLine]) -> Self {
        self.reference_lines = lines;
        self
    }

    /// Render all plot chrome and return the inner drawing area.
    ///
    /// `x_lo`..`x_hi` and `y_lo`..`y_hi` are already-resolved data bounds
    /// (after axis bounds resolution). Returns `None` if the area is too small.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        area: Rect,
        buf: &mut Buffer,
        x_lo: f64,
        x_hi: f64,
        y_lo: f64,
        y_hi: f64,
    ) -> Option<PlotArea> {
        if area.width < 4 || area.height < 4 {
            return None;
        }

        // Compute margins
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let x_label_height: u16 = if self.x_axis.label.is_some() { 1 } else { 0 };
        let tick_height: u16 = 1;

        let plot_x = area.x + self.y_label_width;
        let plot_y = area.y + title_height;
        let plot_width = area
            .width
            .saturating_sub(self.y_label_width + self.colorbar_width + 1);
        let plot_height = area
            .height
            .saturating_sub(title_height + tick_height + x_label_height);

        if plot_width < 2 || plot_height < 2 {
            return None;
        }

        // Apply aspect ratio
        let (ax_off, ay_off, aw, ah) = apply_aspect_ratio(
            &self.aspect_ratio,
            (x_hi - x_lo).abs(),
            (y_hi - y_lo).abs(),
            plot_width,
            plot_height,
        );
        let px = plot_x + ax_off;
        let py = plot_y + ay_off;

        if aw < 2 || ah < 2 {
            return None;
        }

        // Draw title
        if let Some(title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        // Draw spines (axis borders)
        if self.spines.bottom {
            for x in px..px + aw {
                if x < area.x + area.width {
                    buf[(x, py + ah)]
                        .set_char('─')
                        .set_fg(self.theme.axis_color);
                }
            }
        }
        if self.spines.left && px > area.x {
            for y in py..py + ah {
                buf[(px.saturating_sub(1), y)]
                    .set_char('│')
                    .set_fg(self.theme.axis_color);
            }
        }
        if self.spines.top && py > 0 {
            for x in px..px + aw {
                if x < area.x + area.width {
                    let ty = py.saturating_sub(1);
                    if ty >= area.y {
                        buf[(x, ty)].set_char('─').set_fg(self.theme.axis_color);
                    }
                }
            }
        }
        if self.spines.right {
            let rx = px + aw;
            if rx < area.x + area.width {
                for y in py..py + ah {
                    buf[(rx, y)].set_char('│').set_fg(self.theme.axis_color);
                }
            }
        }

        // Draw major grid lines
        let x_grid = self.x_axis.grid || self.theme.grid_visible;
        let y_grid = self.y_axis.grid || self.theme.grid_visible;

        if x_grid {
            let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
            for &tv in &x_ticks {
                let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                let xi = sx.round() as u16;
                if xi >= px && xi < px + aw {
                    for y in py..py + ah {
                        buf[(xi, y)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }
        if y_grid {
            let y_ticks = self.y_axis.tick_positions(y_lo, y_hi);
            for &tv in &y_ticks {
                let sy = data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ah {
                    for x in px..px + aw {
                        buf[(x, yi)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }

        // Draw minor grid lines
        if self.x_axis.minor_grid {
            let minor = self.x_axis.minor_tick_positions(x_lo, x_hi);
            for &tv in &minor {
                let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                let xi = sx.round() as u16;
                if xi >= px && xi < px + aw {
                    for y in py..py + ah {
                        buf[(xi, y)].set_char('⋅').set_fg(self.theme.grid_color);
                    }
                }
            }
        }
        if self.y_axis.minor_grid {
            let minor = self.y_axis.minor_tick_positions(y_lo, y_hi);
            for &tv in &minor {
                let sy = data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ah {
                    for x in px..px + aw {
                        buf[(x, yi)].set_char('⋅').set_fg(self.theme.grid_color);
                    }
                }
            }
        }

        // Draw reference lines and spans
        self.draw_reference_lines(buf, px, py, aw, ah, x_lo, x_hi, y_lo, y_hi);

        // Draw x tick labels
        let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let label_start = xi.saturating_sub(label.len() as u16 / 2);
            let y = py + ah;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }

        // Draw y tick labels
        let y_ticks = self.y_axis.tick_positions(y_lo, y_hi);
        for &tv in &y_ticks {
            let sy = data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
            let label = self.y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ah {
                let label_start = px.saturating_sub(label.len() as u16 + 1);
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < px {
                        buf[(lx, yi)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }

        // Draw axis labels
        if let Some(ref label) = self.x_axis.label {
            let y = area.y + area.height - 1;
            let start = px + (aw.saturating_sub(label.len() as u16)) / 2;
            for (i, ch) in label.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width && y < area.y + area.height {
                    buf[(x, y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }
        if let Some(ref label) = self.y_axis.label {
            let x = area.x;
            let start_y = py + (ah.saturating_sub(label.len() as u16)) / 2;
            for (i, ch) in label.chars().enumerate() {
                let y = start_y + i as u16;
                if y < py + ah {
                    buf[(x, y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        Some(PlotArea {
            x: px,
            y: py,
            width: aw,
            height: ah,
            x_lo,
            x_hi,
            y_lo,
            y_hi,
            area,
        })
    }

    /// Draw annotations within the plot area.
    pub fn draw_annotations(pa: &PlotArea, annotations: &[Annotation], buf: &mut Buffer) {
        for ann in annotations {
            let sx = pa.screen_x(ann.text_x);
            let sy = pa.screen_y(ann.text_y);
            let xi = sx.round() as u16;
            let yi = sy.round() as u16;
            if yi >= pa.y && yi < pa.y + pa.height {
                for (j, ch) in ann.text.chars().enumerate() {
                    let x = xi + j as u16;
                    if x >= pa.x && x < pa.x + pa.width {
                        buf[(x, yi)].set_char(ch).set_fg(ann.color);
                    }
                }
            }
            // Draw arrow if target is specified
            if let Some((tx, ty)) = ann.target {
                let target_sx = pa.screen_x(tx).round() as u16;
                let target_sy = pa.screen_y(ty).round() as u16;
                if pa.contains(target_sx, target_sy) {
                    let dx = target_sx as f64 - xi as f64;
                    let dy = target_sy as f64 - yi as f64;
                    let arrow_ch = ann.arrow_char(dx, dy);
                    if arrow_ch != ' ' {
                        buf[(target_sx, target_sy)]
                            .set_char(arrow_ch)
                            .set_fg(ann.color);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_reference_lines(
        &self,
        buf: &mut Buffer,
        px: u16,
        py: u16,
        aw: u16,
        ah: u16,
        x_lo: f64,
        x_hi: f64,
        y_lo: f64,
        y_hi: f64,
    ) {
        for refline in self.reference_lines {
            match refline {
                ReferenceLine::Horizontal { y, color, dash } => {
                    let sy = data_to_screen(*y, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
                    let yi = sy.round() as u16;
                    if yi >= py && yi < py + ah {
                        for x in px..px + aw {
                            let draw = match dash {
                                RefLineDash::Solid => true,
                                RefLineDash::Dashed => ((x - px) / 3).is_multiple_of(2),
                                RefLineDash::Dotted => (x - px).is_multiple_of(2),
                            };
                            if draw {
                                buf[(x, yi)].set_char('─').set_fg(*color);
                            }
                        }
                    }
                }
                ReferenceLine::Vertical { x, color, dash } => {
                    let sx = data_to_screen(*x, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                    let xi = sx.round() as u16;
                    if xi >= px && xi < px + aw {
                        for y in py..py + ah {
                            let draw = match dash {
                                RefLineDash::Solid => true,
                                RefLineDash::Dashed => ((y - py) / 2).is_multiple_of(2),
                                RefLineDash::Dotted => (y - py).is_multiple_of(2),
                            };
                            if draw {
                                buf[(xi, y)].set_char('│').set_fg(*color);
                            }
                        }
                    }
                }
                ReferenceLine::HorizontalSpan { y1, y2, color } => {
                    let sy1 = data_to_screen(*y1, y_lo, y_hi, (py + ah - 1) as f64, py as f64)
                        .round() as u16;
                    let sy2 = data_to_screen(*y2, y_lo, y_hi, (py + ah - 1) as f64, py as f64)
                        .round() as u16;
                    let top = sy1.min(sy2).max(py);
                    let bot = sy1.max(sy2).min(py + ah);
                    for y in top..bot {
                        for x in px..px + aw {
                            buf[(x, y)].set_char('░').set_fg(*color);
                        }
                    }
                }
                ReferenceLine::VerticalSpan { x1, x2, color } => {
                    let sx1 = data_to_screen(*x1, x_lo, x_hi, px as f64, (px + aw - 1) as f64)
                        .round() as u16;
                    let sx2 = data_to_screen(*x2, x_lo, x_hi, px as f64, (px + aw - 1) as f64)
                        .round() as u16;
                    let left = sx1.min(sx2).max(px);
                    let right = sx1.max(sx2).min(px + aw);
                    for x in left..right {
                        for y in py..py + ah {
                            buf[(x, y)].set_char('░').set_fg(*color);
                        }
                    }
                }
            }
        }
    }
}
