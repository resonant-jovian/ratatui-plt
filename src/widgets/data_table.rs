//! Data table widget for formatted tabular display.
//!
//! Renders a formatted data table with optional header coloring and
//! colormap-based cell backgrounds. Uses theme border characters for
//! consistent visual styling.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//!
//! let table = DataTable::new()
//!     .headers(vec!["Name", "Value", "Error"])
//!     .row(vec!["alpha", "1.23", "0.05"])
//!     .row(vec!["beta", "4.56", "0.12"])
//!     .title("Fit Parameters");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

use crate::colormap::Colormap;
use crate::norm::{LinearNorm, Normalize};
use crate::theme::Theme;

/// A formatted data table widget.
///
/// Displays tabular data with configurable headers, column widths, and
/// optional colormap-based cell highlighting. Named `DataTable` to avoid
/// conflicting with ratatui's built-in `Table` widget.
pub struct DataTable {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    title: Option<String>,
    header_color: Option<Color>,
    col_widths: Option<Vec<u16>>,
    cell_colormap: Option<Box<dyn Colormap>>,
    cell_values: Option<Vec<Vec<f64>>>,
    theme: Theme,
}

impl Default for DataTable {
    fn default() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            title: None,
            header_color: None,
            col_widths: None,
            cell_colormap: None,
            cell_values: None,
            theme: Theme::get_default(),
        }
    }
}

impl DataTable {
    /// Create an empty data table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the column headers.
    pub fn headers(mut self, headers: Vec<impl Into<String>>) -> Self {
        self.headers = headers.into_iter().map(Into::into).collect();
        self
    }

    /// Add a single data row.
    pub fn row(mut self, row: Vec<impl Into<String>>) -> Self {
        self.rows.push(row.into_iter().map(Into::into).collect());
        self
    }

    /// Add multiple data rows at once.
    pub fn rows(mut self, rows: Vec<Vec<impl Into<String>>>) -> Self {
        for r in rows {
            self.rows.push(r.into_iter().map(Into::into).collect());
        }
        self
    }

    /// Set the table title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the header text color.
    pub fn header_color(mut self, color: Color) -> Self {
        self.header_color = Some(color);
        self
    }

    /// Set explicit column widths.
    ///
    /// If not set, columns are auto-sized based on content width.
    pub fn col_widths(mut self, widths: Vec<u16>) -> Self {
        self.col_widths = Some(widths);
        self
    }

    /// Set a colormap for cell background coloring.
    ///
    /// Requires [`cell_values`](Self::cell_values) to also be set.
    /// Each cell's background color is determined by mapping its numeric
    /// value through the colormap.
    pub fn cell_colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.cell_colormap = Some(Box::new(cmap));
        self
    }

    /// Set numeric values for colormap-based cell coloring.
    ///
    /// The outer vec corresponds to rows, the inner to columns.
    /// Values are normalized to \[0,1\] using the global min/max across
    /// all provided values.
    pub fn cell_values(mut self, values: Vec<Vec<f64>>) -> Self {
        self.cell_values = Some(values);
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

impl DataTable {
    /// Compute the number of columns from headers and rows.
    fn num_cols(&self) -> usize {
        let mut n = self.headers.len();
        for row in &self.rows {
            n = n.max(row.len());
        }
        n
    }

    /// Compute auto-sized column widths based on content.
    fn compute_col_widths(&self) -> Vec<u16> {
        let ncols = self.num_cols();
        if ncols == 0 {
            return Vec::new();
        }

        // If explicit widths provided, use those (padded or truncated to ncols)
        if let Some(ref widths) = self.col_widths {
            let mut result = widths.clone();
            result.resize(ncols, 6);
            return result;
        }

        // Auto-size: max of header width and all row cell widths, + 2 padding
        let mut widths = vec![0u16; ncols];
        for (i, h) in self.headers.iter().enumerate() {
            if i < ncols {
                widths[i] = widths[i].max(h.len() as u16);
            }
        }
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < ncols {
                    widths[i] = widths[i].max(cell.len() as u16);
                }
            }
        }
        // Add padding (1 space each side)
        for w in &mut widths {
            *w += 2;
            // Minimum width of 3
            *w = (*w).max(3);
        }
        widths
    }

    /// Compute value bounds for colormap normalization.
    fn value_bounds(&self) -> (f64, f64) {
        let mut vmin = f64::INFINITY;
        let mut vmax = f64::NEG_INFINITY;
        if let Some(ref vals) = self.cell_values {
            for row in vals {
                for &v in row {
                    if v.is_finite() {
                        vmin = vmin.min(v);
                        vmax = vmax.max(v);
                    }
                }
            }
        }
        if vmin.is_infinite() {
            (0.0, 1.0)
        } else if (vmax - vmin).abs() < 1e-15 {
            (vmin - 0.5, vmax + 0.5)
        } else {
            (vmin, vmax)
        }
    }
}

/// Write a string into the buffer at the given position, clipped to max_width.
fn write_str_clipped(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    text: &str,
    max_width: u16,
    style: Style,
    area: Rect,
) {
    if y < area.y || y >= area.y + area.height {
        return;
    }
    for (i, ch) in text.chars().enumerate() {
        let cx = x + i as u16;
        if cx >= x + max_width || cx >= area.x + area.width {
            break;
        }
        if cx >= area.x && cx < area.x + area.width {
            let cell = &mut buf[(cx, y)];
            cell.set_char(ch);
            cell.set_style(style);
        }
    }
}

/// Write a single character into the buffer with a style.
fn write_char(buf: &mut Buffer, x: u16, y: u16, ch: char, style: Style, area: Rect) {
    if x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height {
        let cell = &mut buf[(x, y)];
        cell.set_char(ch);
        cell.set_style(style);
    }
}

impl Widget for &DataTable {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 5 || area.height < 2 {
            return;
        }

        let ncols = self.num_cols();
        if ncols == 0 {
            return;
        }

        let col_widths = self.compute_col_widths();
        let border_char_h = self.theme.chars.border.horizontal;
        let border_char_v = self.theme.chars.border.vertical;
        let fg_color = self.theme.foreground;
        let header_fg = self.header_color.unwrap_or(self.theme.accent);
        let chrome_style = Style::default().fg(self.theme.axis_color);
        let fg_style = Style::default().fg(fg_color);

        // Compute total table width (sum of columns + separators + outer borders)
        let total_content_width: u16 =
            col_widths.iter().sum::<u16>() + (ncols as u16).saturating_sub(1); // separators between columns
        let table_width = (total_content_width + 2).min(area.width); // +2 for outer borders

        let mut cur_y = area.y;

        // ── Title ─────────────────────────────────────────────────────────
        if let Some(ref title) = self.title {
            let title_style = if self.theme.bold_title {
                Style::default().fg(fg_color).add_modifier(Modifier::BOLD)
            } else {
                fg_style
            };
            // Center the title
            let title_len = title.len() as u16;
            let title_x = area.x + area.width.saturating_sub(title_len) / 2;
            write_str_clipped(buf, title_x, cur_y, title, area.width, title_style, area);
            cur_y += 1;
        }

        if cur_y >= area.y + area.height {
            return;
        }

        // ── Top border ────────────────────────────────────────────────────
        let table_x = area.x;
        write_char(
            buf,
            table_x,
            cur_y,
            self.theme.chars.border.top_left,
            chrome_style,
            area,
        );
        let mut cx = table_x + 1;
        for (ci, &cw) in col_widths.iter().enumerate() {
            for _ in 0..cw {
                if cx < table_x + table_width - 1 {
                    write_char(buf, cx, cur_y, border_char_h, chrome_style, area);
                    cx += 1;
                }
            }
            if ci + 1 < ncols && cx < table_x + table_width - 1 {
                write_char(
                    buf,
                    cx,
                    cur_y,
                    self.theme.chars.border.tee_down,
                    chrome_style,
                    area,
                );
                cx += 1;
            }
        }
        if table_x + table_width - 1 < area.x + area.width {
            write_char(
                buf,
                table_x + table_width - 1,
                cur_y,
                self.theme.chars.border.top_right,
                chrome_style,
                area,
            );
        }
        cur_y += 1;

        if cur_y >= area.y + area.height {
            return;
        }

        // ── Header row ────────────────────────────────────────────────────
        if !self.headers.is_empty() {
            let header_style = Style::default().fg(header_fg).add_modifier(Modifier::BOLD);

            write_char(buf, table_x, cur_y, border_char_v, chrome_style, area);
            let mut cx = table_x + 1;
            for (ci, &cw) in col_widths.iter().enumerate() {
                let text = self.headers.get(ci).map_or("", |s| s.as_str());
                // Center text in cell, with 1-char padding on each side
                let available = cw.saturating_sub(2) as usize;
                let text_len = text.len().min(available);
                let pad_left = (available.saturating_sub(text_len)) / 2;

                // Write padding
                for _ in 0..pad_left + 1 {
                    if cx < table_x + table_width - 1 {
                        write_char(buf, cx, cur_y, ' ', header_style, area);
                        cx += 1;
                    }
                }
                // Write text
                for ch in text.chars().take(available) {
                    if cx < table_x + table_width - 1 {
                        write_char(buf, cx, cur_y, ch, header_style, area);
                        cx += 1;
                    }
                }
                // Fill remainder
                let used = pad_left + 1 + text_len;
                for _ in used..cw as usize {
                    if cx < table_x + table_width - 1 {
                        write_char(buf, cx, cur_y, ' ', header_style, area);
                        cx += 1;
                    }
                }

                if ci + 1 < ncols && cx < table_x + table_width - 1 {
                    write_char(buf, cx, cur_y, border_char_v, chrome_style, area);
                    cx += 1;
                }
            }
            if table_x + table_width - 1 < area.x + area.width {
                write_char(
                    buf,
                    table_x + table_width - 1,
                    cur_y,
                    border_char_v,
                    chrome_style,
                    area,
                );
            }
            cur_y += 1;

            if cur_y >= area.y + area.height {
                return;
            }

            // ── Header separator ──────────────────────────────────────────
            write_char(
                buf,
                table_x,
                cur_y,
                self.theme.chars.border.tee_right,
                chrome_style,
                area,
            );
            let mut cx = table_x + 1;
            for (ci, &cw) in col_widths.iter().enumerate() {
                for _ in 0..cw {
                    if cx < table_x + table_width - 1 {
                        write_char(buf, cx, cur_y, border_char_h, chrome_style, area);
                        cx += 1;
                    }
                }
                if ci + 1 < ncols && cx < table_x + table_width - 1 {
                    write_char(
                        buf,
                        cx,
                        cur_y,
                        self.theme.chars.border.cross,
                        chrome_style,
                        area,
                    );
                    cx += 1;
                }
            }
            if table_x + table_width - 1 < area.x + area.width {
                write_char(
                    buf,
                    table_x + table_width - 1,
                    cur_y,
                    self.theme.chars.border.tee_left,
                    chrome_style,
                    area,
                );
            }
            cur_y += 1;
        }

        // ── Data rows ─────────────────────────────────────────────────────
        let (vmin, vmax) = self.value_bounds();
        let norm = LinearNorm::new(vmin, vmax);

        for (ri, row) in self.rows.iter().enumerate() {
            if cur_y >= area.y + area.height - 1 {
                break;
            }

            write_char(buf, table_x, cur_y, border_char_v, chrome_style, area);
            let mut cx = table_x + 1;

            for (ci, &cw) in col_widths.iter().enumerate() {
                let text = row.get(ci).map_or("", |s| s.as_str());

                // Determine cell style (with optional colormap background)
                let cell_style = if let (Some(cmap), Some(vals)) =
                    (&self.cell_colormap, &self.cell_values)
                {
                    let val = vals
                        .get(ri)
                        .and_then(|r| r.get(ci))
                        .copied()
                        .unwrap_or(f64::NAN);
                    if val.is_finite() {
                        let t = norm.normalize(val);
                        let bg = cmap.color_at(t);
                        // Choose contrasting foreground
                        let cell_fg = match bg {
                            Color::Rgb(r, g, b) => {
                                let lum = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000;
                                if lum > 128 {
                                    Color::Black
                                } else {
                                    Color::White
                                }
                            }
                            _ => fg_color,
                        };
                        Style::default().fg(cell_fg).bg(bg)
                    } else {
                        fg_style
                    }
                } else {
                    fg_style
                };

                // Right-align numeric-looking values, left-align text
                let available = cw.saturating_sub(2) as usize;
                let text_len = text.len().min(available);
                let is_numeric = text.parse::<f64>().is_ok();

                let pad_left = if is_numeric {
                    available.saturating_sub(text_len)
                } else {
                    0
                };

                // Write left padding + space
                write_char(buf, cx, cur_y, ' ', cell_style, area);
                cx += 1;
                for _ in 0..pad_left {
                    if cx < table_x + table_width - 1 {
                        write_char(buf, cx, cur_y, ' ', cell_style, area);
                        cx += 1;
                    }
                }
                // Write text
                for ch in text.chars().take(available) {
                    if cx < table_x + table_width - 1 {
                        write_char(buf, cx, cur_y, ch, cell_style, area);
                        cx += 1;
                    }
                }
                // Fill remainder
                let used = 1 + pad_left + text_len;
                for _ in used..cw as usize {
                    if cx < table_x + table_width - 1 {
                        write_char(buf, cx, cur_y, ' ', cell_style, area);
                        cx += 1;
                    }
                }

                if ci + 1 < ncols && cx < table_x + table_width - 1 {
                    write_char(buf, cx, cur_y, border_char_v, chrome_style, area);
                    cx += 1;
                }
            }
            if table_x + table_width - 1 < area.x + area.width {
                write_char(
                    buf,
                    table_x + table_width - 1,
                    cur_y,
                    border_char_v,
                    chrome_style,
                    area,
                );
            }
            cur_y += 1;
        }

        if cur_y >= area.y + area.height {
            return;
        }

        // ── Bottom border ─────────────────────────────────────────────────
        write_char(
            buf,
            table_x,
            cur_y,
            self.theme.chars.border.bottom_left,
            chrome_style,
            area,
        );
        let mut cx = table_x + 1;
        for (ci, &cw) in col_widths.iter().enumerate() {
            for _ in 0..cw {
                if cx < table_x + table_width - 1 {
                    write_char(buf, cx, cur_y, border_char_h, chrome_style, area);
                    cx += 1;
                }
            }
            if ci + 1 < ncols && cx < table_x + table_width - 1 {
                write_char(
                    buf,
                    cx,
                    cur_y,
                    self.theme.chars.border.tee_up,
                    chrome_style,
                    area,
                );
                cx += 1;
            }
        }
        if table_x + table_width - 1 < area.x + area.width {
            write_char(
                buf,
                table_x + table_width - 1,
                cur_y,
                self.theme.chars.border.bottom_right,
                chrome_style,
                area,
            );
        }
    }
}
