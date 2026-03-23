//! Image and text export for ratatui-plt widgets.
//!
//! This module provides a "buffer screenshot" approach to exporting plots:
//! render any widget into an off-screen [`Buffer`], then convert that buffer
//! to plain text, ANSI-colored text, or SVG.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::export::{render_to_buffer, buffer_to_text, buffer_to_ansi, buffer_to_svg};
//!
//! let plot = LinePlot::new()
//!     .series(Series::new("demo").data(vec![(0.0, 0.0), (1.0, 1.0)]))
//!     .title("Export demo");
//!
//! let buf = render_to_buffer(&plot, 80, 24);
//! let text = buffer_to_text(&buf);
//! let ansi = buffer_to_ansi(&buf);
//! let svg = buffer_to_svg(&buf, 14.0);
//! ```

use std::fmt::Write as FmtWrite;
use std::path::Path;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// Render a widget into an off-screen buffer of the given dimensions.
///
/// The widget is rendered into a fresh [`Buffer`] at position (0, 0).
/// Because ratatui-plt widgets implement `Widget for &WidgetType`, you
/// can pass a reference to avoid consuming the widget:
///
/// ```
/// # use ratatui_plt::prelude::*;
/// # use ratatui_plt::export::render_to_buffer;
/// let plot = LinePlot::new()
///     .series(Series::new("s").data(vec![(0.0, 0.0), (1.0, 1.0)]));
/// let buf = render_to_buffer(&plot, 80, 24);
/// // `plot` is still usable here
/// ```
pub fn render_to_buffer<W: Widget>(widget: W, width: u16, height: u16) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    widget.render(area, &mut buf);
    buf
}

/// Convert a buffer to a plain-text string.
///
/// Each row becomes one line of output. Trailing spaces on each line are
/// trimmed, and lines are joined with newlines. The result contains only
/// the Unicode characters from the buffer cells with no color information.
pub fn buffer_to_text(buf: &Buffer) -> String {
    let area = buf.area;
    let mut output = String::with_capacity((area.width as usize + 1) * area.height as usize);

    for row in 0..area.height {
        if row > 0 {
            output.push('\n');
        }
        let mut line = String::with_capacity(area.width as usize);
        for col in 0..area.width {
            let idx = (row * area.width + col) as usize;
            line.push_str(buf.content[idx].symbol());
        }
        let trimmed = line.trim_end();
        output.push_str(trimmed);
    }
    output
}

/// Convert a buffer to an ANSI-escaped string that reproduces colors.
///
/// Uses 24-bit true-color sequences (`\x1b[38;2;r;g;bm` for foreground,
/// `\x1b[48;2;r;g;bm` for background). Named ANSI colors are mapped to
/// their standard SGR codes. Each line ends with a reset (`\x1b[0m`).
///
/// The resulting string can be printed to a terminal or saved to a file
/// and viewed with `cat` or `less -R`.
pub fn buffer_to_ansi(buf: &Buffer) -> String {
    let area = buf.area;
    let mut output = String::with_capacity((area.width as usize * 20 + 1) * area.height as usize);

    for row in 0..area.height {
        if row > 0 {
            output.push('\n');
        }

        let mut cur_fg: Option<Color> = None;
        let mut cur_bg: Option<Color> = None;

        for col in 0..area.width {
            let idx = (row * area.width + col) as usize;
            let cell = &buf.content[idx];

            // Emit foreground escape if color changed.
            if cur_fg != Some(cell.fg) {
                write_ansi_fg(&mut output, cell.fg);
                cur_fg = Some(cell.fg);
            }

            // Emit background escape if color changed.
            if cur_bg != Some(cell.bg) {
                write_ansi_bg(&mut output, cell.bg);
                cur_bg = Some(cell.bg);
            }

            output.push_str(cell.symbol());
        }

        // Reset at end of line.
        output.push_str("\x1b[0m");
    }
    output
}

/// Convert a buffer to an SVG document string.
///
/// Each cell is represented by a `<rect>` for the background (if not
/// the default/reset color) and a `<text>` element for the character.
/// The document uses a monospace font at the given `font_size` (in px).
///
/// Character width is assumed to be `0.6 * font_size` (typical for
/// monospace fonts) and line height is `1.2 * font_size`.
pub fn buffer_to_svg(buf: &Buffer, font_size: f64) -> String {
    let area = buf.area;
    let char_width = font_size * 0.6;
    let line_height = font_size * 1.2;
    let svg_width = area.width as f64 * char_width;
    let svg_height = area.height as f64 * line_height;

    let mut svg = String::with_capacity((area.width as usize * area.height as usize) * 200 + 512);

    // SVG header.
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {svg_width} {svg_height}" "#
    );
    let _ = write!(svg, r#"width="{svg_width}" height="{svg_height}" "#);
    let _ = write!(svg, r#"style="background:#1e1e1e">"#);
    svg.push('\n');

    // Style block for monospace text.
    let _ = write!(
        svg,
        r#"<style>text{{font-family:"Cascadia Code","Fira Code","JetBrains Mono","Source Code Pro",monospace;font-size:{font_size}px;dominant-baseline:text-before-edge;white-space:pre}}</style>"#
    );
    svg.push('\n');

    for row in 0..area.height {
        let y = row as f64 * line_height;

        for col in 0..area.width {
            let idx = (row * area.width + col) as usize;
            let cell = &buf.content[idx];
            let x = col as f64 * char_width;

            // Background rect (skip if reset/default).
            if cell.bg != Color::Reset {
                let (r, g, b) = color_to_rgb(cell.bg);
                let _ = write!(
                    svg,
                    r#"<rect x="{x}" y="{y}" width="{char_width}" height="{line_height}" fill="rgb({r},{g},{b})"/>"#
                );
                svg.push('\n');
            }

            // Text element (skip if space with no special color).
            let sym = cell.symbol();
            if sym != " " || cell.fg != Color::Reset {
                let (r, g, b) = color_to_rgb(cell.fg);
                let escaped = xml_escape(sym);
                let _ = write!(
                    svg,
                    r#"<text x="{x}" y="{y}" fill="rgb({r},{g},{b})">{escaped}</text>"#
                );
                svg.push('\n');
            }
        }
    }

    svg.push_str("</svg>\n");
    svg
}

/// Render a widget and save the plain-text output to a file.
///
/// This is a convenience wrapper around [`render_to_buffer`] + [`buffer_to_text`].
pub fn save_text<W: Widget>(
    widget: W,
    width: u16,
    height: u16,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let buf = render_to_buffer(widget, width, height);
    let text = buffer_to_text(&buf);
    std::fs::write(path, text)
}

/// Render a widget and save as an ANSI-escaped text file.
///
/// The output can be viewed in a terminal with `cat` or `less -R`.
pub fn save_ansi<W: Widget>(
    widget: W,
    width: u16,
    height: u16,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let buf = render_to_buffer(widget, width, height);
    let ansi = buffer_to_ansi(&buf);
    std::fs::write(path, ansi)
}

/// Render a widget and save as an SVG file.
///
/// Uses a default font size of 14px. For custom font sizes, use
/// [`render_to_buffer`] + [`buffer_to_svg`] + [`std::fs::write`].
pub fn save_svg<W: Widget>(
    widget: W,
    width: u16,
    height: u16,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    let buf = render_to_buffer(widget, width, height);
    let svg = buffer_to_svg(&buf, 14.0);
    std::fs::write(path, svg)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Map a ratatui [`Color`] to an (r, g, b) triple.
///
/// Named ANSI colors are mapped to widely-used default values.
/// `Color::Reset` maps to light gray (the typical terminal default).
fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::Gray => (170, 170, 170),
        Color::DarkGray => (118, 118, 118),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        Color::White => (229, 229, 229),
        Color::Indexed(idx) => ansi256_to_rgb(idx),
        Color::Reset => (204, 204, 204),
    }
}

/// Convert an 8-bit ANSI-256 color index to RGB.
fn ansi256_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        // Standard colors (0-7).
        0 => (0, 0, 0),
        1 => (205, 49, 49),
        2 => (13, 188, 121),
        3 => (229, 229, 16),
        4 => (36, 114, 200),
        5 => (188, 63, 188),
        6 => (17, 168, 205),
        7 => (170, 170, 170),
        // Bright colors (8-15).
        8 => (118, 118, 118),
        9 => (241, 76, 76),
        10 => (35, 209, 139),
        11 => (245, 245, 67),
        12 => (59, 142, 234),
        13 => (214, 112, 214),
        14 => (41, 184, 219),
        15 => (229, 229, 229),
        // 216-color cube (16-231): 6x6x6.
        16..=231 => {
            let idx = idx - 16;
            let r_idx = idx / 36;
            let g_idx = (idx % 36) / 6;
            let b_idx = idx % 6;
            let to_val = |i: u8| if i == 0 { 0 } else { 55 + 40 * i };
            (to_val(r_idx), to_val(g_idx), to_val(b_idx))
        }
        // Grayscale ramp (232-255): 24 shades.
        232..=255 => {
            let v = 8 + 10 * (idx - 232);
            (v, v, v)
        }
    }
}

/// Write an ANSI foreground color escape sequence.
fn write_ansi_fg(out: &mut String, color: Color) {
    match color {
        Color::Reset => out.push_str("\x1b[39m"),
        Color::Black => out.push_str("\x1b[30m"),
        Color::Red => out.push_str("\x1b[31m"),
        Color::Green => out.push_str("\x1b[32m"),
        Color::Yellow => out.push_str("\x1b[33m"),
        Color::Blue => out.push_str("\x1b[34m"),
        Color::Magenta => out.push_str("\x1b[35m"),
        Color::Cyan => out.push_str("\x1b[36m"),
        Color::Gray => out.push_str("\x1b[37m"),
        Color::DarkGray => out.push_str("\x1b[90m"),
        Color::LightRed => out.push_str("\x1b[91m"),
        Color::LightGreen => out.push_str("\x1b[92m"),
        Color::LightYellow => out.push_str("\x1b[93m"),
        Color::LightBlue => out.push_str("\x1b[94m"),
        Color::LightMagenta => out.push_str("\x1b[95m"),
        Color::LightCyan => out.push_str("\x1b[96m"),
        Color::White => out.push_str("\x1b[97m"),
        Color::Rgb(r, g, b) => {
            let _ = write!(out, "\x1b[38;2;{r};{g};{b}m");
        }
        Color::Indexed(idx) => {
            let _ = write!(out, "\x1b[38;5;{idx}m");
        }
    }
}

/// Write an ANSI background color escape sequence.
fn write_ansi_bg(out: &mut String, color: Color) {
    match color {
        Color::Reset => out.push_str("\x1b[49m"),
        Color::Black => out.push_str("\x1b[40m"),
        Color::Red => out.push_str("\x1b[41m"),
        Color::Green => out.push_str("\x1b[42m"),
        Color::Yellow => out.push_str("\x1b[43m"),
        Color::Blue => out.push_str("\x1b[44m"),
        Color::Magenta => out.push_str("\x1b[45m"),
        Color::Cyan => out.push_str("\x1b[46m"),
        Color::Gray => out.push_str("\x1b[47m"),
        Color::DarkGray => out.push_str("\x1b[100m"),
        Color::LightRed => out.push_str("\x1b[101m"),
        Color::LightGreen => out.push_str("\x1b[102m"),
        Color::LightYellow => out.push_str("\x1b[103m"),
        Color::LightBlue => out.push_str("\x1b[104m"),
        Color::LightMagenta => out.push_str("\x1b[105m"),
        Color::LightCyan => out.push_str("\x1b[106m"),
        Color::White => out.push_str("\x1b[107m"),
        Color::Rgb(r, g, b) => {
            let _ = write!(out, "\x1b[48;2;{r};{g};{b}m");
        }
        Color::Indexed(idx) => {
            let _ = write!(out, "\x1b[48;5;{idx}m");
        }
    }
}

// ---------------------------------------------------------------------------
// Export feature: PNG generation, Kitty and Sixel graphics protocols
// ---------------------------------------------------------------------------

/// Options for image export.
#[cfg(feature = "export")]
#[derive(Clone, Debug)]
pub struct ExportOptions {
    /// Width of each cell in pixels.
    pub cell_width: u32,
    /// Height of each cell in pixels.
    pub cell_height: u32,
}

#[cfg(feature = "export")]
impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            cell_width: 8,
            cell_height: 16,
        }
    }
}

#[cfg(feature = "export")]
impl ExportOptions {
    /// Create new export options with default cell dimensions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the cell width in pixels.
    pub fn cell_width(mut self, w: u32) -> Self {
        self.cell_width = w;
        self
    }

    /// Set the cell height in pixels.
    pub fn cell_height(mut self, h: u32) -> Self {
        self.cell_height = h;
        self
    }
}

/// Errors that can occur during image export.
#[cfg(feature = "export")]
#[derive(Debug)]
pub enum ExportError {
    /// I/O error writing output.
    Io(std::io::Error),
    /// Image encoding error.
    Image(String),
}

#[cfg(feature = "export")]
impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Image(e) => write!(f, "Image error: {e}"),
        }
    }
}

#[cfg(feature = "export")]
impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Image(_) => None,
        }
    }
}

#[cfg(feature = "export")]
impl From<std::io::Error> for ExportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Convert a ratatui buffer to PNG image bytes.
///
/// Each terminal cell is rendered as a `cell_width x cell_height` pixel rectangle
/// using the cell's background color (foreground color if no background set).
#[cfg(feature = "export")]
pub fn buffer_to_png(buf: &Buffer, options: &ExportOptions) -> Result<Vec<u8>, ExportError> {
    let area = buf.area;
    let img_w = area.width as u32 * options.cell_width;
    let img_h = area.height as u32 * options.cell_height;

    let mut img = image::RgbaImage::new(img_w, img_h);

    for row in 0..area.height {
        for col in 0..area.width {
            let idx = (row * area.width + col) as usize;
            let cell = &buf.content[idx];

            // Use background color if set, otherwise use foreground for text cells.
            let (r, g, b) = if cell.bg != Color::Reset {
                color_to_rgb(cell.bg)
            } else if cell.symbol() != " " {
                color_to_rgb(cell.fg)
            } else {
                color_to_rgb(Color::Reset)
            };

            let px_x_start = col as u32 * options.cell_width;
            let px_y_start = row as u32 * options.cell_height;

            for py in 0..options.cell_height {
                for px in 0..options.cell_width {
                    img.put_pixel(
                        px_x_start + px,
                        px_y_start + py,
                        image::Rgba([r, g, b, 255]),
                    );
                }
            }
        }
    }

    let mut png_bytes: Vec<u8> = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| ExportError::Image(e.to_string()))?;
    Ok(png_bytes)
}

// ---------------------------------------------------------------------------
// Kitty graphics protocol
// ---------------------------------------------------------------------------

/// Encode bytes as base64.
#[cfg(feature = "kitty")]
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        result.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Convert a ratatui buffer to a Kitty graphics protocol escape sequence.
///
/// The buffer is first rendered to PNG, then base64-encoded and wrapped in
/// Kitty escape sequences with chunked transfer (4096-byte chunks).
#[cfg(feature = "kitty")]
pub fn buffer_to_kitty(buf: &Buffer, options: &ExportOptions) -> Result<String, ExportError> {
    let png_bytes = buffer_to_png(buf, options)?;
    let b64 = base64_encode(&png_bytes);
    let chunk_size = 4096;
    let mut output = String::new();

    if b64.len() <= chunk_size {
        // Single chunk: m=0 means no more data.
        let _ = write!(output, "\x1b_Gf=100,a=T,t=d,m=0;{b64}\x1b\\");
    } else {
        let chunks: Vec<&str> = {
            let mut v = Vec::new();
            let mut start = 0;
            while start < b64.len() {
                let end = (start + chunk_size).min(b64.len());
                v.push(&b64[start..end]);
                start = end;
            }
            v
        };
        let last_idx = chunks.len() - 1;
        for (i, chunk) in chunks.iter().enumerate() {
            if i == 0 {
                let _ = write!(output, "\x1b_Gf=100,a=T,t=d,m=1;{chunk}\x1b\\");
            } else if i == last_idx {
                let _ = write!(output, "\x1b_Gm=0;{chunk}\x1b\\");
            } else {
                let _ = write!(output, "\x1b_Gm=1;{chunk}\x1b\\");
            }
        }
    }

    Ok(output)
}

/// Render a widget and print it using the Kitty graphics protocol.
///
/// The widget is rendered into an off-screen buffer, converted to PNG,
/// and output as a Kitty inline image to stdout.
#[cfg(feature = "kitty")]
pub fn print_kitty<W: Widget>(
    widget: W,
    width: u16,
    height: u16,
    options: &ExportOptions,
) -> Result<(), ExportError> {
    let buf = render_to_buffer(widget, width, height);
    let kitty = buffer_to_kitty(&buf, options)?;
    print!("{kitty}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Sixel graphics protocol
// ---------------------------------------------------------------------------

/// Convert a ratatui buffer to a Sixel graphics escape sequence.
///
/// The buffer is rendered to PNG, decoded to RGBA pixels, quantized to a
/// 256-color palette, and encoded as Sixel data.
#[cfg(feature = "sixel")]
pub fn buffer_to_sixel(buf: &Buffer, options: &ExportOptions) -> Result<String, ExportError> {
    let png_bytes = buffer_to_png(buf, options)?;
    let img = image::load_from_memory(&png_bytes)
        .map_err(|e| ExportError::Image(e.to_string()))?;
    let rgba = img.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());

    // Build color palette: collect unique RGB triples, limit to 256.
    let mut palette: Vec<(u8, u8, u8)> = Vec::new();
    let mut pixel_indices: Vec<u16> = Vec::with_capacity((width * height) as usize);

    for pixel in rgba.pixels() {
        let rgb = (pixel[0], pixel[1], pixel[2]);
        let idx = if let Some(pos) = palette.iter().position(|c| *c == rgb) {
            pos as u16
        } else if palette.len() < 256 {
            let pos = palette.len() as u16;
            palette.push(rgb);
            pos
        } else {
            // Find nearest color in palette.
            nearest_palette_color(&palette, rgb)
        };
        pixel_indices.push(idx);
    }

    // Build sixel output.
    let mut output = String::new();

    // DCS: enter sixel mode.
    output.push_str("\x1bPq");

    // Raster attributes: pixel aspect 1:1, image dimensions.
    let _ = write!(output, "\"1;1;{width};{height}");

    // Define palette entries.
    for (i, &(r, g, b)) in palette.iter().enumerate() {
        let r_pct = (r as u32 * 100) / 255;
        let g_pct = (g as u32 * 100) / 255;
        let b_pct = (b as u32 * 100) / 255;
        let _ = write!(output, "#{i};2;{r_pct};{g_pct};{b_pct}");
    }

    // Encode sixel bands (6 rows each).
    let mut band_start: u32 = 0;
    while band_start < height {
        let band_end = (band_start + 6).min(height);

        // Find which colors are present in this band.
        let mut colors_in_band: Vec<u16> = Vec::new();
        for row in band_start..band_end {
            for col in 0..width {
                let idx = pixel_indices[(row * width + col) as usize];
                if !colors_in_band.contains(&idx) {
                    colors_in_band.push(idx);
                }
            }
        }

        for (ci, &color_idx) in colors_in_band.iter().enumerate() {
            // Select color.
            let _ = write!(output, "#{color_idx}");

            // For each column, compute 6-bit sixel value.
            for col in 0..width {
                let mut sixel_val: u8 = 0;
                for bit in 0..6u32 {
                    let row = band_start + bit;
                    if row < height {
                        let pi = pixel_indices[(row * width + col) as usize];
                        if pi == color_idx {
                            sixel_val |= 1 << bit;
                        }
                    }
                }
                // Sixel character = value + 63.
                output.push((sixel_val + 63) as char);
            }

            // Carriage return after each color pass (except the last in the band).
            if ci < colors_in_band.len() - 1 {
                output.push('$');
            }
        }

        // Newline / next band.
        band_start += 6;
        if band_start < height {
            output.push('-');
        }
    }

    // String terminator.
    output.push_str("\x1b\\");

    Ok(output)
}

/// Find the index of the nearest color in the palette to the given RGB value.
#[cfg(feature = "sixel")]
fn nearest_palette_color(palette: &[(u8, u8, u8)], rgb: (u8, u8, u8)) -> u16 {
    let mut best_idx: u16 = 0;
    let mut best_dist = u32::MAX;
    for (i, &(pr, pg, pb)) in palette.iter().enumerate() {
        let dr = (rgb.0 as i32 - pr as i32).unsigned_abs();
        let dg = (rgb.1 as i32 - pg as i32).unsigned_abs();
        let db = (rgb.2 as i32 - pb as i32).unsigned_abs();
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best_dist = dist;
            best_idx = i as u16;
        }
    }
    best_idx
}

/// Render a widget and print it using the Sixel graphics protocol.
///
/// The widget is rendered into an off-screen buffer, converted to a Sixel
/// image, and printed to stdout.
#[cfg(feature = "sixel")]
pub fn print_sixel<W: Widget>(
    widget: W,
    width: u16,
    height: u16,
    options: &ExportOptions,
) -> Result<(), ExportError> {
    let buf = render_to_buffer(widget, width, height);
    let sixel = buffer_to_sixel(&buf, options)?;
    print!("{sixel}");
    Ok(())
}

/// Escape a string for embedding in XML/SVG.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_to_buffer_dimensions() {
        let area = Rect::new(0, 0, 40, 10);
        let buf = Buffer::empty(area);
        assert_eq!(buf.area.width, 40);
        assert_eq!(buf.area.height, 10);
        assert_eq!(buf.content.len(), 400);
    }

    #[test]
    fn test_buffer_to_text_empty() {
        let buf = Buffer::empty(Rect::new(0, 0, 5, 2));
        let text = buffer_to_text(&buf);
        // All spaces get trimmed, so we get empty lines.
        assert_eq!(text, "\n");
    }

    #[test]
    fn test_buffer_to_text_with_content() {
        let area = Rect::new(0, 0, 10, 2);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_char('H');
        buf[(1, 0)].set_char('i');
        buf[(0, 1)].set_char('!');
        let text = buffer_to_text(&buf);
        assert_eq!(text, "Hi\n!");
    }

    #[test]
    fn test_buffer_to_ansi_contains_escapes() {
        let area = Rect::new(0, 0, 3, 1);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_char('R');
        buf[(0, 0)].set_fg(Color::Red);
        let ansi = buffer_to_ansi(&buf);
        assert!(ansi.contains("\x1b[31m"));
        assert!(ansi.contains("R"));
        assert!(ansi.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_buffer_to_ansi_rgb_colors() {
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_char('X');
        buf[(0, 0)].set_fg(Color::Rgb(255, 128, 0));
        buf[(0, 0)].set_bg(Color::Rgb(0, 0, 64));
        let ansi = buffer_to_ansi(&buf);
        assert!(ansi.contains("\x1b[38;2;255;128;0m"));
        assert!(ansi.contains("\x1b[48;2;0;0;64m"));
    }

    #[test]
    fn test_buffer_to_svg_structure() {
        let area = Rect::new(0, 0, 3, 2);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_char('A');
        buf[(0, 0)].set_fg(Color::Cyan);
        buf[(1, 1)].set_char('B');
        buf[(1, 1)].set_bg(Color::Red);
        let svg = buffer_to_svg(&buf, 14.0);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<text"));
        assert!(svg.contains("A</text>"));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("monospace"));
    }

    #[test]
    fn test_buffer_to_svg_escapes_special_chars() {
        let area = Rect::new(0, 0, 3, 1);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_char('<');
        buf[(0, 0)].set_fg(Color::White);
        buf[(1, 0)].set_char('&');
        buf[(1, 0)].set_fg(Color::White);
        let svg = buffer_to_svg(&buf, 14.0);
        assert!(svg.contains("&lt;"));
        assert!(svg.contains("&amp;"));
        assert!(!svg.contains(">&<"));
    }

    #[test]
    fn test_color_to_rgb_named() {
        assert_eq!(color_to_rgb(Color::Black), (0, 0, 0));
        assert_eq!(color_to_rgb(Color::White), (229, 229, 229));
        assert_eq!(color_to_rgb(Color::Rgb(10, 20, 30)), (10, 20, 30));
    }

    #[test]
    fn test_ansi256_to_rgb_standard() {
        assert_eq!(ansi256_to_rgb(0), (0, 0, 0));
        assert_eq!(ansi256_to_rgb(15), (229, 229, 229));
    }

    #[test]
    fn test_ansi256_to_rgb_cube() {
        // Index 16 = (0,0,0) in cube = black
        assert_eq!(ansi256_to_rgb(16), (0, 0, 0));
        // Index 196 = (5,0,0) in cube = bright red
        let (r, g, b) = ansi256_to_rgb(196);
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_ansi256_to_rgb_grayscale() {
        // Index 232 = darkest gray
        assert_eq!(ansi256_to_rgb(232), (8, 8, 8));
        // Index 255 = lightest gray
        assert_eq!(ansi256_to_rgb(255), (238, 238, 238));
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("a&b"), "a&amp;b");
        assert_eq!(xml_escape("<>"), "&lt;&gt;");
        assert_eq!(xml_escape("\"'"), "&quot;&apos;");
        assert_eq!(xml_escape("hello"), "hello");
    }

    #[test]
    fn test_save_text_roundtrip() {
        let area = Rect::new(0, 0, 5, 2);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_char('X');
        buf[(4, 1)].set_char('Y');
        let text = buffer_to_text(&buf);
        assert!(text.contains('X'));
        assert!(text.contains('Y'));
    }
}
