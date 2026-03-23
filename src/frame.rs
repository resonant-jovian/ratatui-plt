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
use crate::plot_buffer::{PlotBuffer, Z_ANNOTATION, Z_CHROME, Z_FILL, Z_GRID};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::transform::{apply_aspect_ratio, data_to_screen};

/// Resolved data bounds for a 2D plot (x and y range).
///
/// Used by [`PlotFrame::render`] to pass the four axis limits as a single argument.
#[derive(Clone, Debug, Copy)]
pub struct DataBounds {
    /// Minimum x data value.
    pub x_lo: f64,
    /// Maximum x data value.
    pub x_hi: f64,
    /// Minimum y data value.
    pub y_lo: f64,
    /// Maximum y data value.
    pub y_hi: f64,
}

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
    /// Optional y bounds limit the vertical extent of the span.
    VerticalSpan {
        x1: f64,
        x2: f64,
        color: Color,
        y_lo: Option<f64>,
        y_hi: Option<f64>,
    },
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

    /// Vertical filled span between x1 and x2, covering the full Y range.
    pub fn vspan(x1: f64, x2: f64, color: Color) -> Self {
        Self::VerticalSpan {
            x1,
            x2,
            color,
            y_lo: None,
            y_hi: None,
        }
    }

    /// Vertical filled span between x1 and x2, limited to the given Y range.
    pub fn vspan_bounded(x1: f64, x2: f64, y_lo: f64, y_hi: f64, color: Color) -> Self {
        Self::VerticalSpan {
            x1,
            x2,
            color,
            y_lo: Some(y_lo),
            y_hi: Some(y_hi),
        }
    }
}

/// Border style for plot frames.
#[derive(Clone, Debug, Default)]
pub enum BorderStyle {
    /// Standard single-line box drawing.
    #[default]
    Single,
    /// Rounded corners.
    Rounded,
    /// Double-line box drawing.
    Double,
    /// No border.
    None,
}

impl BorderStyle {
    /// Top-left corner character.
    pub fn top_left(&self) -> char {
        match self { Self::Single => '┌', Self::Rounded => '╭', Self::Double => '╔', Self::None => ' ' }
    }
    /// Top-right corner character.
    pub fn top_right(&self) -> char {
        match self { Self::Single => '┐', Self::Rounded => '╮', Self::Double => '╗', Self::None => ' ' }
    }
    /// Bottom-left corner character.
    pub fn bottom_left(&self) -> char {
        match self { Self::Single => '└', Self::Rounded => '╰', Self::Double => '╚', Self::None => ' ' }
    }
    /// Bottom-right corner character.
    pub fn bottom_right(&self) -> char {
        match self { Self::Single => '┘', Self::Rounded => '╯', Self::Double => '╝', Self::None => ' ' }
    }
    /// Horizontal line character.
    pub fn horizontal(&self) -> char {
        match self { Self::Single | Self::Rounded => '─', Self::Double => '═', Self::None => ' ' }
    }
    /// Vertical line character.
    pub fn vertical(&self) -> char {
        match self { Self::Single | Self::Rounded => '│', Self::Double => '║', Self::None => ' ' }
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
/// let bounds = DataBounds { x_lo, x_hi, y_lo, y_hi };
/// let Some(pa) = frame.render(area, buf, bounds) else {
///     return;
/// };
/// // Draw widget-specific data using pa.screen_x(), pa.screen_y()...
/// ```
pub struct PlotFrame<'a> {
    title: Option<&'a str>,
    x_axis: &'a Axis,
    y_axis: &'a Axis,
    aspect_ratio: AspectRatio,
    border_style: BorderStyle,
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
            border_style: BorderStyle::default(),
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

    /// Set the border style (Single, Rounded, Double, None).
    pub fn border_style(mut self, bs: BorderStyle) -> Self {
        self.border_style = bs;
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
    /// `bounds` contains the already-resolved data bounds
    /// (after axis bounds resolution). Returns `None` if the area is too small.
    pub fn render(&self, area: Rect, buf: &mut Buffer, bounds: DataBounds) -> Option<PlotArea> {
        let DataBounds {
            mut x_lo,
            mut x_hi,
            mut y_lo,
            mut y_hi,
        } = bounds;
        if area.width < 4 || area.height < 4 {
            return None;
        }

        // Compute margins
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let x_label_height: u16 = if self.x_axis.label.is_some() {
            match self.x_axis.label_position {
                crate::axis::LabelPosition::End => 1, // 1 row for bottom border of box
                crate::axis::LabelPosition::Center => {
                    if self.x_axis.label_boxed { 3 } else { 1 }
                }
            }
        } else {
            0
        };
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
        let (ax_off, ay_off, mut aw, mut ah) = apply_aspect_ratio(
            &self.aspect_ratio,
            (x_hi - x_lo).abs(),
            (y_hi - y_lo).abs(),
            plot_width,
            plot_height,
        );
        let mut px = plot_x + ax_off;
        let mut py = plot_y + ay_off;

        if aw < 2 || ah < 2 {
            return None;
        }

        // Snap Auto bounds to tick positions and align pixel grid for uniform cells.
        {
            let x_ticks_snap = self.x_axis.tick_positions(x_lo, x_hi);
            if matches!(self.x_axis.bounds, crate::axis::Bounds::Auto)
                && matches!(self.x_axis.scale, crate::axis::Scale::Linear)
                && x_ticks_snap.len() >= 2
            {
                x_lo = x_ticks_snap[0];
                x_hi = x_ticks_snap[x_ticks_snap.len() - 1];
                // Align pixel width to be divisible by number of intervals
                let n_intervals = (x_ticks_snap.len() - 1) as u16;
                if let Some(cell_w) = aw.checked_div(n_intervals) {
                    let aligned_w = cell_w * n_intervals;
                    let pad = aw - aligned_w;
                    px += pad / 2;
                    aw = aligned_w;
                }
            }
            let y_ticks_snap = self.y_axis.tick_positions(y_lo, y_hi);
            if matches!(self.y_axis.bounds, crate::axis::Bounds::Auto)
                && matches!(self.y_axis.scale, crate::axis::Scale::Linear)
                && y_ticks_snap.len() >= 2
            {
                y_lo = y_ticks_snap[0];
                y_hi = y_ticks_snap[y_ticks_snap.len() - 1];
                let n_intervals = (y_ticks_snap.len() - 1) as u16;
                if let Some(cell_h) = ah.checked_div(n_intervals) {
                    let aligned_h = cell_h * n_intervals;
                    let pad = ah - aligned_h;
                    py += pad / 2;
                    ah = aligned_h;
                }
            }
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

        // Draw spines (axis borders) using the configured border style
        let h_char = self.border_style.horizontal();
        let v_char = self.border_style.vertical();

        if self.spines.bottom {
            for x in px..px + aw {
                if x < area.x + area.width {
                    buf[(x, py + ah)]
                        .set_char(h_char)
                        .set_fg(self.theme.axis_color);
                }
            }
        }
        if self.spines.left && px > area.x {
            for y in py..py + ah {
                buf[(px.saturating_sub(1), y)]
                    .set_char(v_char)
                    .set_fg(self.theme.axis_color);
            }
        }
        if self.spines.top && py > 0 {
            for x in px..px + aw {
                if x < area.x + area.width {
                    let ty = py.saturating_sub(1);
                    if ty >= area.y {
                        buf[(x, ty)].set_char(h_char).set_fg(self.theme.axis_color);
                    }
                }
            }
        }
        if self.spines.right {
            let rx = px + aw;
            if rx < area.x + area.width {
                for y in py..py + ah {
                    buf[(rx, y)].set_char(v_char).set_fg(self.theme.axis_color);
                }
            }
        }

        // Draw major grid lines using thin box-drawing characters.
        // Horizontal lines use '─', vertical use '│', intersections use '┼'.
        let x_grid = self.x_axis.grid || self.theme.grid_visible;
        let y_grid = self.y_axis.grid || self.theme.grid_visible;

        if y_grid {
            let y_ticks = self.y_axis.tick_positions(y_lo, y_hi);
            for &tv in &y_ticks {
                let sy = data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ah {
                    for x in px..px + aw {
                        buf[(x, yi)].set_char('─').set_fg(self.theme.grid_color);
                    }
                }
            }
        }
        if x_grid {
            let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
            for &tv in &x_ticks {
                let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                let xi = sx.round() as u16;
                if xi >= px && xi < px + aw {
                    for y in py..py + ah {
                        let ch = if buf[(xi, y)].symbol() == "─" { '┼' } else { '│' };
                        buf[(xi, y)].set_char(ch).set_fg(self.theme.grid_color);
                    }
                }
            }
        }

        // Draw minor grid lines using light dashed box-drawing characters.
        if self.y_axis.minor_grid {
            let minor = self.y_axis.minor_tick_positions(y_lo, y_hi);
            for &tv in &minor {
                let sy = data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ah {
                    for x in px..px + aw {
                        buf[(x, yi)]
                            .set_char('┄')
                            .set_fg(self.theme.minor_grid_color);
                    }
                }
            }
        }
        if self.x_axis.minor_grid {
            let minor = self.x_axis.minor_tick_positions(x_lo, x_hi);
            for &tv in &minor {
                let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                let xi = sx.round() as u16;
                if xi >= px && xi < px + aw {
                    for y in py..py + ah {
                        buf[(xi, y)]
                            .set_char('┆')
                            .set_fg(self.theme.minor_grid_color);
                    }
                }
            }
        }

        // Draw reference lines and spans
        self.draw_reference_lines(
            buf,
            &PlotArea {
                x: px,
                y: py,
                width: aw,
                height: ah,
                x_lo,
                x_hi,
                y_lo,
                y_hi,
                area,
            },
        );

        // Draw x tick labels with overlap detection
        let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
        let mut last_label_end: u16 = 0;
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let label_len = label.len() as u16;
            let label_start = xi.saturating_sub(label_len / 2);
            let label_end = label_start + label_len;

            // Skip this label if it would overlap with the previous one
            if label_start < last_label_end + 1 && last_label_end > 0 {
                continue;
            }

            let y = py + ah;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
                last_label_end = label_end;
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
        self.draw_x_label(buf, area, px, py, aw, ah);
        self.draw_y_label(buf, area, py, ah);

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

    /// Draw the x-axis label, optionally in a box.
    fn draw_x_label(&self, buf: &mut Buffer, area: Rect, px: u16, py: u16, aw: u16, ah: u16) {
        let Some(ref label) = self.x_axis.label else {
            return;
        };
        let fg = self.theme.foreground;
        let bc = self.theme.axis_color;
        let label_len = label.chars().count() as u16;

        match self.x_axis.label_position {
            crate::axis::LabelPosition::Center => {
                let y = area.y + area.height - 1;
                if self.x_axis.label_boxed {
                    let box_w = label_len + 4;
                    let box_x = px + (aw.saturating_sub(box_w)) / 2;
                    Self::draw_boxed_label_h(buf, box_x, y, label, fg, bc, area);
                } else {
                    let start = px + (aw.saturating_sub(label_len)) / 2;
                    for (i, ch) in label.chars().enumerate() {
                        let x = start + i as u16;
                        if x < area.x + area.width && y < area.y + area.height {
                            buf[(x, y)].set_char(ch).set_fg(fg);
                        }
                    }
                }
            }
            crate::axis::LabelPosition::End => {
                // Place on the tick label row, right after the plot area
                let y = py + ah; // same row as tick values
                let box_x = (px + aw).saturating_sub(2); // slightly overlapping end
                if self.x_axis.label_boxed {
                    Self::draw_boxed_label_h(buf, box_x, y, label, fg, bc, area);
                } else {
                    for (i, ch) in label.chars().enumerate() {
                        let x = box_x + i as u16;
                        if x < area.x + area.width && y < area.y + area.height {
                            buf[(x, y)].set_char(ch).set_fg(fg);
                        }
                    }
                }
            }
        }
    }

    /// Draw the y-axis label, optionally in a box.
    fn draw_y_label(&self, buf: &mut Buffer, area: Rect, py: u16, ah: u16) {
        let Some(ref label) = self.y_axis.label else {
            return;
        };
        let fg = self.theme.foreground;
        let bc = self.theme.axis_color;
        let label_len = label.chars().count() as u16;

        match self.y_axis.label_position {
            crate::axis::LabelPosition::End => {
                // Horizontal text at the top of the y-axis, in a box
                let y = py.max(area.y);
                if self.y_axis.label_boxed {
                    let box_x = area.x;
                    Self::draw_boxed_label_h(buf, box_x, y, label, fg, bc, area);
                } else {
                    for (i, ch) in label.chars().enumerate() {
                        let x = area.x + i as u16;
                        if x < area.x + area.width && y < area.y + area.height {
                            buf[(x, y)].set_char(ch).set_fg(fg);
                        }
                    }
                }
            }
            crate::axis::LabelPosition::Center => {
                // Bottom-to-top vertical stacking, centered along y-axis
                let label_x = area.x;
                let center_y = py + ah / 2;
                let label_start_y = center_y.saturating_sub(label_len / 2);
                if self.y_axis.label_boxed {
                    // Box around vertical text: 3 chars wide, label_len+2 tall
                    let box_top = label_start_y.saturating_sub(1);
                    let box_bot = label_start_y + label_len;
                    // Top border
                    if box_top >= area.y && box_top < area.y + area.height {
                        if label_x < area.x + area.width {
                            buf[(label_x, box_top)].set_char('┌').set_fg(fg);
                        }
                        if label_x + 1 < area.x + area.width {
                            buf[(label_x + 1, box_top)].set_char('─').set_fg(fg);
                        }
                        if label_x + 2 < area.x + area.width {
                            buf[(label_x + 2, box_top)].set_char('┐').set_fg(fg);
                        }
                    }
                    // Characters bottom-to-top
                    for (i, ch) in label.chars().enumerate() {
                        let y = label_start_y + i as u16;
                        if y >= area.y && y < area.y + area.height {
                            if label_x < area.x + area.width {
                                buf[(label_x, y)].set_char('│').set_fg(fg);
                            }
                            if label_x + 1 < area.x + area.width {
                                buf[(label_x + 1, y)].set_char(ch).set_fg(fg);
                            }
                            if label_x + 2 < area.x + area.width {
                                buf[(label_x + 2, y)].set_char('│').set_fg(fg);
                            }
                        }
                    }
                    // Bottom border
                    if box_bot >= area.y && box_bot < area.y + area.height {
                        if label_x < area.x + area.width {
                            buf[(label_x, box_bot)].set_char('└').set_fg(fg);
                        }
                        if label_x + 1 < area.x + area.width {
                            buf[(label_x + 1, box_bot)].set_char('─').set_fg(fg);
                        }
                        if label_x + 2 < area.x + area.width {
                            buf[(label_x + 2, box_bot)].set_char('┘').set_fg(fg);
                        }
                    }
                } else {
                    // Bottom-to-top without box
                    for (i, ch) in label.chars().enumerate() {
                        let y = label_start_y + i as u16;
                        if label_x < area.x + area.width && y >= area.y && y < area.y + area.height
                        {
                            buf[(label_x, y)].set_char(ch).set_fg(fg);
                        }
                    }
                }
            }
        }
    }

    /// Draw a horizontal label in a bordered box (legend-style, 3 rows tall).
    ///
    /// ```text
    /// ┌──────────┐
    /// │  label   │
    /// └──────────┘
    /// ```
    fn draw_boxed_label_h(
        buf: &mut Buffer,
        x: u16,
        y: u16,
        label: &str,
        fg: Color,
        border_color: Color,
        area: Rect,
    ) {
        let label_len = label.chars().count() as u16;
        let box_w = label_len + 4; // 1 border + 1 pad + label + 1 pad + 1 border
        let top_y = y.saturating_sub(1);
        let bot_y = y + 1;
        let max_x = area.x + area.width;
        let max_y = area.y + area.height;

        // Top border row: ┌──┐
        if top_y >= area.y && top_y < max_y {
            if x < max_x {
                buf[(x, top_y)].set_char('┌').set_fg(border_color);
            }
            for i in 1..box_w.saturating_sub(1) {
                if x + i < max_x {
                    buf[(x + i, top_y)].set_char('─').set_fg(border_color);
                }
            }
            if x + box_w - 1 < max_x {
                buf[(x + box_w - 1, top_y)].set_char('┐').set_fg(border_color);
            }
        }

        // Middle row: │ label │ (bold text)
        if y >= area.y && y < max_y {
            if x < max_x {
                buf[(x, y)].set_char('│').set_fg(border_color);
            }
            if x + 1 < max_x {
                buf[(x + 1, y)].set_char(' ').set_fg(fg);
            }
            for (i, ch) in label.chars().enumerate() {
                let cx = x + 2 + i as u16;
                if cx < max_x {
                    buf[(cx, y)]
                        .set_char(ch)
                        .set_fg(fg)
                        .set_style(ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD));
                }
            }
            if x + label_len + 2 < max_x {
                buf[(x + label_len + 2, y)].set_char(' ').set_fg(fg);
            }
            if x + box_w - 1 < max_x {
                buf[(x + box_w - 1, y)].set_char('│').set_fg(border_color);
            }
        }

        // Bottom border row: └──┘
        if bot_y >= area.y && bot_y < max_y {
            if x < max_x {
                buf[(x, bot_y)].set_char('└').set_fg(border_color);
            }
            for i in 1..box_w.saturating_sub(1) {
                if x + i < max_x {
                    buf[(x + i, bot_y)].set_char('─').set_fg(border_color);
                }
            }
            if x + box_w - 1 < max_x {
                buf[(x + box_w - 1, bot_y)].set_char('┘').set_fg(border_color);
            }
        }
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

    fn draw_reference_lines(&self, buf: &mut Buffer, pa: &PlotArea) {
        let (px, py, aw, ah) = (pa.x, pa.y, pa.width, pa.height);
        let (x_lo, x_hi, y_lo, y_hi) = (pa.x_lo, pa.x_hi, pa.y_lo, pa.y_hi);
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
                            buf[(x, y)]
                                .set_char('░')
                                .set_fg(*color)
                                .set_bg(*color);
                        }
                    }
                }
                ReferenceLine::VerticalSpan {
                    x1,
                    x2,
                    color,
                    y_lo: span_y_lo,
                    y_hi: span_y_hi,
                } => {
                    let sx1 = data_to_screen(*x1, x_lo, x_hi, px as f64, (px + aw - 1) as f64)
                        .round() as u16;
                    let sx2 = data_to_screen(*x2, x_lo, x_hi, px as f64, (px + aw - 1) as f64)
                        .round() as u16;
                    let left = sx1.min(sx2).max(px);
                    let right = sx1.max(sx2).min(px + aw);
                    // Determine vertical extent: use bounded Y range if provided
                    let (y_top, y_bottom) = if let (Some(yl), Some(yh)) = (span_y_lo, span_y_hi) {
                        let sy_lo = data_to_screen(*yl, y_lo, y_hi, (py + ah - 1) as f64, py as f64)
                            .round() as u16;
                        let sy_hi = data_to_screen(*yh, y_lo, y_hi, (py + ah - 1) as f64, py as f64)
                            .round() as u16;
                        (sy_hi.min(sy_lo).max(py), sy_hi.max(sy_lo).min(py + ah))
                    } else {
                        (py, py + ah)
                    };
                    for x in left..right {
                        for y in y_top..y_bottom {
                            buf[(x, y)]
                                .set_char('░')
                                .set_fg(*color)
                                .set_bg(*color);
                        }
                    }
                }
            }
        }
    }

    /// Render all plot chrome into a [`PlotBuffer`] and return the inner drawing area.
    ///
    /// This is the Z-buffered counterpart of [`PlotFrame::render`]. Every visual
    /// element is written with an explicit Z-level so that compositing produces
    /// correct layering (fills behind grids behind data, etc.).
    ///
    /// `bounds` contains the already-resolved data bounds (after axis bounds
    /// resolution). Returns `None` if the area is too small.
    pub fn render_to_pb(
        &self,
        pb: &mut PlotBuffer,
        area: Rect,
        bounds: DataBounds,
    ) -> Option<PlotArea> {
        let DataBounds {
            mut x_lo,
            mut x_hi,
            mut y_lo,
            mut y_hi,
        } = bounds;
        if area.width < 4 || area.height < 4 {
            return None;
        }

        // Compute margins
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let x_label_height: u16 = if self.x_axis.label.is_some() {
            match self.x_axis.label_position {
                crate::axis::LabelPosition::End => 1, // 1 row for bottom border of box
                crate::axis::LabelPosition::Center => {
                    if self.x_axis.label_boxed { 3 } else { 1 }
                }
            }
        } else {
            0
        };
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
        let (ax_off, ay_off, mut aw, mut ah) = apply_aspect_ratio(
            &self.aspect_ratio,
            (x_hi - x_lo).abs(),
            (y_hi - y_lo).abs(),
            plot_width,
            plot_height,
        );
        let mut px = plot_x + ax_off;
        let mut py = plot_y + ay_off;

        if aw < 2 || ah < 2 {
            return None;
        }

        // Snap Auto bounds to tick positions and align pixel grid for uniform cells.
        {
            let x_ticks_snap = self.x_axis.tick_positions(x_lo, x_hi);
            if matches!(self.x_axis.bounds, crate::axis::Bounds::Auto)
                && matches!(self.x_axis.scale, crate::axis::Scale::Linear)
                && x_ticks_snap.len() >= 2
            {
                x_lo = x_ticks_snap[0];
                x_hi = x_ticks_snap[x_ticks_snap.len() - 1];
                let n_intervals = (x_ticks_snap.len() - 1) as u16;
                if let Some(cell_w) = aw.checked_div(n_intervals) {
                    let aligned_w = cell_w * n_intervals;
                    let pad = aw - aligned_w;
                    px += pad / 2;
                    aw = aligned_w;
                }
            }
            let y_ticks_snap = self.y_axis.tick_positions(y_lo, y_hi);
            if matches!(self.y_axis.bounds, crate::axis::Bounds::Auto)
                && matches!(self.y_axis.scale, crate::axis::Scale::Linear)
                && y_ticks_snap.len() >= 2
            {
                y_lo = y_ticks_snap[0];
                y_hi = y_ticks_snap[y_ticks_snap.len() - 1];
                let n_intervals = (y_ticks_snap.len() - 1) as u16;
                if let Some(cell_h) = ah.checked_div(n_intervals) {
                    let aligned_h = cell_h * n_intervals;
                    let pad = ah - aligned_h;
                    py += pad / 2;
                    ah = aligned_h;
                }
            }
        }

        // Draw title
        if let Some(title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    pb.set_char(x, area.y, ch, self.theme.foreground, Z_CHROME);
                }
            }
        }

        // Draw spines (axis borders) using the configured border style
        let h_char = self.border_style.horizontal();
        let v_char = self.border_style.vertical();

        if self.spines.bottom {
            for x in px..px + aw {
                if x < area.x + area.width {
                    pb.set_char(x, py + ah, h_char, self.theme.axis_color, Z_CHROME);
                }
            }
        }
        if self.spines.left && px > area.x {
            for y in py..py + ah {
                pb.set_char(
                    px.saturating_sub(1),
                    y,
                    v_char,
                    self.theme.axis_color,
                    Z_CHROME,
                );
            }
        }
        if self.spines.top && py > 0 {
            for x in px..px + aw {
                if x < area.x + area.width {
                    let ty = py.saturating_sub(1);
                    if ty >= area.y {
                        pb.set_char(x, ty, h_char, self.theme.axis_color, Z_CHROME);
                    }
                }
            }
        }
        if self.spines.right {
            let rx = px + aw;
            if rx < area.x + area.width {
                for y in py..py + ah {
                    pb.set_char(rx, y, v_char, self.theme.axis_color, Z_CHROME);
                }
            }
        }

        // Draw major grid lines
        let x_grid = self.x_axis.grid || self.theme.grid_visible;
        let y_grid = self.y_axis.grid || self.theme.grid_visible;

        // Track horizontal grid row positions for intersection detection
        let mut h_grid_rows: Vec<u16> = Vec::new();

        if y_grid {
            let y_ticks = self.y_axis.tick_positions(y_lo, y_hi);
            for &tv in &y_ticks {
                let sy = data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ah {
                    h_grid_rows.push(yi);
                    for x in px..px + aw {
                        pb.set_char(x, yi, '─', self.theme.grid_color, Z_GRID);
                    }
                }
            }
        }
        if x_grid {
            let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
            for &tv in &x_ticks {
                let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                let xi = sx.round() as u16;
                if xi >= px && xi < px + aw {
                    for y in py..py + ah {
                        let ch = if h_grid_rows.contains(&y) { '┼' } else { '│' };
                        pb.set_char(xi, y, ch, self.theme.grid_color, Z_GRID);
                    }
                }
            }
        }

        // Draw minor grid lines
        if self.y_axis.minor_grid {
            let minor = self.y_axis.minor_tick_positions(y_lo, y_hi);
            for &tv in &minor {
                let sy = data_to_screen(tv, y_lo, y_hi, (py + ah - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ah {
                    for x in px..px + aw {
                        pb.set_char(x, yi, '┄', self.theme.minor_grid_color, Z_GRID);
                    }
                }
            }
        }
        if self.x_axis.minor_grid {
            let minor = self.x_axis.minor_tick_positions(x_lo, x_hi);
            for &tv in &minor {
                let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
                let xi = sx.round() as u16;
                if xi >= px && xi < px + aw {
                    for y in py..py + ah {
                        pb.set_char(xi, y, '┆', self.theme.minor_grid_color, Z_GRID);
                    }
                }
            }
        }

        // Draw reference lines and spans
        self.draw_reference_lines_pb(
            pb,
            &PlotArea {
                x: px,
                y: py,
                width: aw,
                height: ah,
                x_lo,
                x_hi,
                y_lo,
                y_hi,
                area,
            },
        );

        // Draw x tick labels with overlap detection
        let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
        let mut last_label_end: u16 = 0;
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + aw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let label_len = label.len() as u16;
            let label_start = xi.saturating_sub(label_len / 2);
            let label_end = label_start + label_len;

            // Skip this label if it would overlap with the previous one
            if label_start < last_label_end + 1 && last_label_end > 0 {
                continue;
            }

            let y = py + ah;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        pb.set_char(lx, y, ch, self.theme.axis_color, Z_CHROME);
                    }
                }
                last_label_end = label_end;
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
                        pb.set_char(lx, yi, ch, self.theme.axis_color, Z_CHROME);
                    }
                }
            }
        }

        // Draw axis labels
        self.draw_x_label_pb(pb, area, px, py, aw, ah);
        self.draw_y_label_pb(pb, area, py, ah);

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

    /// Draw annotations into a [`PlotBuffer`] at [`Z_ANNOTATION`].
    ///
    /// This is the Z-buffered counterpart of [`PlotFrame::draw_annotations`].
    /// Draw the x-axis label to PlotBuffer.
    fn draw_x_label_pb(&self, pb: &mut PlotBuffer, area: Rect, px: u16, _py: u16, aw: u16, _ah: u16) {
        let Some(ref label) = self.x_axis.label else {
            return;
        };
        let fg = self.theme.foreground;
        let bc = self.theme.axis_color;
        let label_len = label.chars().count() as u16;

        match self.x_axis.label_position {
            crate::axis::LabelPosition::Center => {
                let y = area.y + area.height - 1;
                if self.x_axis.label_boxed {
                    let box_w = label_len + 4;
                    let box_x = px + (aw.saturating_sub(box_w)) / 2;
                    Self::draw_boxed_label_h_pb(pb, box_x, y, label, fg, bc, area);
                } else {
                    let start = px + (aw.saturating_sub(label_len)) / 2;
                    for (i, ch) in label.chars().enumerate() {
                        let x = start + i as u16;
                        if x < area.x + area.width && y < area.y + area.height {
                            pb.set_char(x, y, ch, fg, Z_CHROME);
                        }
                    }
                }
            }
            crate::axis::LabelPosition::End => {
                // End labels are rendered after composite via draw_end_labels()
                // to avoid PlotBuffer bounds clipping.
            }
        }
    }

    /// Draw End-positioned axis labels directly to the buffer.
    ///
    /// Call this AFTER `pb.composite(buf)` so the labels aren't clipped by
    /// the PlotBuffer area bounds.
    pub fn draw_end_labels(&self, buf: &mut Buffer, _area: Rect, pa: &PlotArea) {
        // Use the full buffer area for bounds (not the widget area) so labels
        // can extend beyond the plot's square_area.
        let buf_area = buf.area;
        if let Some(ref label) = self.x_axis.label
            && matches!(self.x_axis.label_position, crate::axis::LabelPosition::End)
        {
            let fg = self.theme.foreground;
            let bc = self.theme.axis_color;
            let y = pa.y + pa.height;
            let box_x = (pa.x + pa.width).saturating_sub(2);
            if self.x_axis.label_boxed {
                Self::draw_boxed_label_h(buf, box_x, y, label, fg, bc, buf_area);
            } else {
                for (i, ch) in label.chars().enumerate() {
                    let x = box_x + i as u16;
                    if x < buf_area.x + buf_area.width && y < buf_area.y + buf_area.height {
                        buf[(x, y)].set_char(ch).set_fg(fg);
                    }
                }
            }
        }
        // Y-axis End label (horizontal at top)
        if let Some(ref label) = self.y_axis.label
            && matches!(self.y_axis.label_position, crate::axis::LabelPosition::End)
        {
                let fg = self.theme.foreground;
                let bc = self.theme.axis_color;
                let y = pa.y.max(buf_area.y);
                let box_x = buf_area.x;
                if self.y_axis.label_boxed {
                    Self::draw_boxed_label_h(buf, box_x, y, label, fg, bc, buf_area);
                } else {
                    for (i, ch) in label.chars().enumerate() {
                        let x = box_x + i as u16;
                        if x < buf_area.x + buf_area.width && y < buf_area.y + buf_area.height {
                            buf[(x, y)].set_char(ch).set_fg(fg);
                        }
                    }
                }
        }
    }

    /// Draw the y-axis label to PlotBuffer.
    fn draw_y_label_pb(&self, pb: &mut PlotBuffer, area: Rect, py: u16, ah: u16) {
        let Some(ref label) = self.y_axis.label else {
            return;
        };
        let fg = self.theme.foreground;
        let bc = self.theme.axis_color;
        let label_len = label.chars().count() as u16;

        match self.y_axis.label_position {
            crate::axis::LabelPosition::End => {
                // End labels rendered after composite via draw_end_labels()
                // for bold support and to avoid PB bounds clipping.
            }
            crate::axis::LabelPosition::Center => {
                let label_x = area.x;
                let center_y = py + ah / 2;
                let label_start_y = center_y.saturating_sub(label_len / 2);
                if self.y_axis.label_boxed {
                    let box_top = label_start_y.saturating_sub(1);
                    let box_bot = label_start_y + label_len;
                    if box_top >= area.y && box_top < area.y + area.height {
                        if label_x < area.x + area.width {
                            pb.set_char(label_x, box_top, '┌', bc, Z_CHROME);
                        }
                        if label_x + 1 < area.x + area.width {
                            pb.set_char(label_x + 1, box_top, '─', bc, Z_CHROME);
                        }
                        if label_x + 2 < area.x + area.width {
                            pb.set_char(label_x + 2, box_top, '┐', bc, Z_CHROME);
                        }
                    }
                    for (i, ch) in label.chars().enumerate() {
                        let y = label_start_y + i as u16;
                        if y >= area.y && y < area.y + area.height {
                            if label_x < area.x + area.width {
                                pb.set_char(label_x, y, '│', bc, Z_CHROME);
                            }
                            if label_x + 1 < area.x + area.width {
                                pb.set_char(label_x + 1, y, ch, fg, Z_CHROME);
                            }
                            if label_x + 2 < area.x + area.width {
                                pb.set_char(label_x + 2, y, '│', bc, Z_CHROME);
                            }
                        }
                    }
                    if box_bot >= area.y && box_bot < area.y + area.height {
                        if label_x < area.x + area.width {
                            pb.set_char(label_x, box_bot, '└', fg, Z_CHROME);
                        }
                        if label_x + 1 < area.x + area.width {
                            pb.set_char(label_x + 1, box_bot, '─', fg, Z_CHROME);
                        }
                        if label_x + 2 < area.x + area.width {
                            pb.set_char(label_x + 2, box_bot, '┘', fg, Z_CHROME);
                        }
                    }
                } else {
                    for (i, ch) in label.chars().enumerate() {
                        let y = label_start_y + i as u16;
                        if label_x < area.x + area.width && y >= area.y && y < area.y + area.height
                        {
                            pb.set_char(label_x, y, ch, fg, Z_CHROME);
                        }
                    }
                }
            }
        }
    }

    /// Draw a horizontal boxed label to PlotBuffer (legend-style, 3 rows).
    /// Uses set_cell with Color::Reset bg to make the box opaque (clears grid lines behind it).
    fn draw_boxed_label_h_pb(
        pb: &mut PlotBuffer,
        x: u16,
        y: u16,
        label: &str,
        fg: Color,
        border_color: Color,
        area: Rect,
    ) {
        let label_len = label.chars().count() as u16;
        let box_w = label_len + 4;
        let top_y = y.saturating_sub(1);
        let bot_y = y + 1;
        let max_x = area.x + area.width;
        let max_y = area.y + area.height;
        let bg = Color::Reset;

        // Top border
        if top_y >= area.y && top_y < max_y {
            if x < max_x {
                pb.set_cell(x, top_y, '┌', border_color, bg, Z_CHROME);
            }
            for i in 1..box_w.saturating_sub(1) {
                if x + i < max_x {
                    pb.set_cell(x + i, top_y, '─', border_color, bg, Z_CHROME);
                }
            }
            if x + box_w - 1 < max_x {
                pb.set_cell(x + box_w - 1, top_y, '┐', border_color, bg, Z_CHROME);
            }
        }

        // Middle row: │ label │
        if y >= area.y && y < max_y {
            if x < max_x {
                pb.set_cell(x, y, '│', border_color, bg, Z_CHROME);
            }
            // Space padding
            if x + 1 < max_x {
                pb.set_cell(x + 1, y, ' ', fg, bg, Z_CHROME);
            }
            for (i, ch) in label.chars().enumerate() {
                let cx = x + 2 + i as u16;
                if cx < max_x {
                    pb.set_cell(cx, y, ch, fg, bg, Z_CHROME);
                }
            }
            if x + label_len + 2 < max_x {
                pb.set_cell(x + label_len + 2, y, ' ', fg, bg, Z_CHROME);
            }
            if x + box_w - 1 < max_x {
                pb.set_cell(x + box_w - 1, y, '│', border_color, bg, Z_CHROME);
            }
        }

        // Bottom border
        if bot_y >= area.y && bot_y < max_y {
            if x < max_x {
                pb.set_cell(x, bot_y, '└', border_color, bg, Z_CHROME);
            }
            for i in 1..box_w.saturating_sub(1) {
                if x + i < max_x {
                    pb.set_cell(x + i, bot_y, '─', border_color, bg, Z_CHROME);
                }
            }
            if x + box_w - 1 < max_x {
                pb.set_cell(x + box_w - 1, bot_y, '┘', border_color, bg, Z_CHROME);
            }
        }
    }

    pub fn draw_annotations_pb(pa: &PlotArea, annotations: &[Annotation], pb: &mut PlotBuffer) {
        for ann in annotations {
            let sx = pa.screen_x(ann.text_x);
            let sy = pa.screen_y(ann.text_y);
            let xi = sx.round() as u16;
            let yi = sy.round() as u16;
            if yi >= pa.y && yi < pa.y + pa.height {
                for (j, ch) in ann.text.chars().enumerate() {
                    let x = xi + j as u16;
                    if x >= pa.x && x < pa.x + pa.width {
                        pb.set_char(x, yi, ch, ann.color, Z_ANNOTATION);
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
                        pb.set_char(target_sx, target_sy, arrow_ch, ann.color, Z_ANNOTATION);
                    }
                }
            }
        }
    }

    /// Draw reference lines and spans into a [`PlotBuffer`].
    ///
    /// Reference lines use [`Z_GRID`], reference spans/fills use [`Z_FILL`].
    fn draw_reference_lines_pb(&self, pb: &mut PlotBuffer, pa: &PlotArea) {
        let (px, py, aw, ah) = (pa.x, pa.y, pa.width, pa.height);
        let (x_lo, x_hi, y_lo, y_hi) = (pa.x_lo, pa.x_hi, pa.y_lo, pa.y_hi);
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
                                pb.set_char(x, yi, '─', *color, Z_GRID);
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
                                pb.set_char(xi, y, '│', *color, Z_GRID);
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
                            pb.set_bg(x, y, *color, Z_FILL);
                        }
                    }
                }
                ReferenceLine::VerticalSpan {
                    x1,
                    x2,
                    color,
                    y_lo: span_y_lo,
                    y_hi: span_y_hi,
                } => {
                    let sx1 = data_to_screen(*x1, x_lo, x_hi, px as f64, (px + aw - 1) as f64)
                        .round() as u16;
                    let sx2 = data_to_screen(*x2, x_lo, x_hi, px as f64, (px + aw - 1) as f64)
                        .round() as u16;
                    let left = sx1.min(sx2).max(px);
                    let right = sx1.max(sx2).min(px + aw);
                    // Determine vertical extent: use bounded Y range if provided
                    let (y_top, y_bottom) = if let (Some(yl), Some(yh)) = (span_y_lo, span_y_hi) {
                        let sy_lo =
                            data_to_screen(*yl, y_lo, y_hi, (py + ah - 1) as f64, py as f64)
                                .round() as u16;
                        let sy_hi =
                            data_to_screen(*yh, y_lo, y_hi, (py + ah - 1) as f64, py as f64)
                                .round() as u16;
                        (sy_hi.min(sy_lo).max(py), sy_hi.max(sy_lo).min(py + ah))
                    } else {
                        (py, py + ah)
                    };
                    for x in left..right {
                        for y in y_top..y_bottom {
                            pb.set_bg(x, y, *color, Z_FILL);
                        }
                    }
                }
            }
        }
    }
}
