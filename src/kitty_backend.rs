//! Kitty graphics protocol rendering backend.
//!
//! Renders plot data to an internal pixel buffer, then composites as a Kitty
//! inline image during [`PlotBackend::composite`]. Provides pixel-level
//! anti-aliased lines and full RGB color, significantly higher fidelity than
//! the default Unicode/Braille backend.
//!
//! Requires the `kitty` feature and a Kitty-compatible terminal (Kitty,
//! WezTerm, Ghostty).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::frame::PlotArea;
use crate::plot_buffer::PlotBackend;

/// Pixels per terminal cell width.
const CELL_PX_W: u32 = 8;
/// Pixels per terminal cell height.
const CELL_PX_H: u32 = 16;

/// Convert a ratatui `Color` to (r, g, b) bytes.
fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 0, 0),
        Color::Green => (0, 205, 0),
        Color::Yellow => (205, 205, 0),
        Color::Blue => (0, 0, 238),
        Color::Magenta => (205, 0, 205),
        Color::Cyan => (0, 205, 205),
        Color::White | Color::Gray => (229, 229, 229),
        Color::DarkGray => (127, 127, 127),
        Color::LightRed => (255, 0, 0),
        Color::LightGreen => (0, 255, 0),
        Color::LightYellow => (255, 255, 0),
        Color::LightBlue => (92, 92, 255),
        Color::LightMagenta => (255, 0, 255),
        Color::LightCyan => (0, 255, 255),
        Color::Reset => (0, 0, 0),
        Color::Indexed(_) => (128, 128, 128), // Approximate for indexed colors
    }
}

/// Pixel-level rendering backend using the Kitty graphics protocol.
///
/// Maintains an internal RGBA pixel buffer at `CELL_PX_W × CELL_PX_H` pixels
/// per terminal cell. During [`composite`](PlotBackend::composite), the pixel
/// buffer is encoded as a PNG and transmitted via the Kitty graphics protocol
/// escape sequence, with surrounding cells marked as skip.
///
/// Text elements (axis labels, legends, tick marks) are rendered to ratatui
/// cells directly — only data elements (lines, fills, markers) use pixel
/// rendering.
pub struct KittyBackend {
    area: Rect,
    /// RGBA pixel buffer: 4 bytes per pixel, row-major.
    pixels: Vec<u8>,
    /// Width of pixel buffer in pixels.
    px_width: u32,
    /// Height of pixel buffer in pixels.
    px_height: u32,
    /// Per-cell Z-tracking for text elements that bypass pixel rendering.
    text_cells: Vec<Option<(char, Color, Color, u8)>>,
}

impl KittyBackend {
    /// Create a new Kitty backend covering the given terminal area.
    pub fn new(area: Rect) -> Self {
        let px_width = area.width as u32 * CELL_PX_W;
        let px_height = area.height as u32 * CELL_PX_H;
        let pixel_count = (px_width * px_height * 4) as usize;
        Self {
            area,
            pixels: vec![0u8; pixel_count],
            px_width,
            px_height,
            text_cells: vec![None; area.width as usize * area.height as usize],
        }
    }

    /// Index into the pixel buffer for pixel (px, py). Returns None if out of bounds.
    fn px_index(&self, px: u32, py: u32) -> Option<usize> {
        if px < self.px_width && py < self.px_height {
            Some(((py * self.px_width + px) * 4) as usize)
        } else {
            None
        }
    }

    /// Set a single pixel to the given color with full opacity.
    fn set_pixel(&mut self, px: u32, py: u32, r: u8, g: u8, b: u8) {
        if let Some(i) = self.px_index(px, py) {
            self.pixels[i] = r;
            self.pixels[i + 1] = g;
            self.pixels[i + 2] = b;
            self.pixels[i + 3] = 255;
        }
    }

    /// Fill a rectangular region of pixels with the given color.
    fn fill_rect(&mut self, px: u32, py: u32, w: u32, h: u32, rgb: (u8, u8, u8)) {
        for dy in 0..h {
            for dx in 0..w {
                self.set_pixel(px + dx, py + dy, rgb.0, rgb.1, rgb.2);
            }
        }
    }

    /// Index into text_cells for terminal cell (x, y).
    fn cell_index(&self, x: u16, y: u16) -> Option<usize> {
        if x >= self.area.x
            && x < self.area.x + self.area.width
            && y >= self.area.y
            && y < self.area.y + self.area.height
        {
            Some(
                (y - self.area.y) as usize * self.area.width as usize
                    + (x - self.area.x) as usize,
            )
        } else {
            None
        }
    }

    /// Convert terminal cell coordinates to pixel coordinates.
    fn cell_to_px(&self, x: u16, y: u16) -> (u32, u32) {
        let px = (x.saturating_sub(self.area.x)) as u32 * CELL_PX_W;
        let py = (y.saturating_sub(self.area.y)) as u32 * CELL_PX_H;
        (px, py)
    }
}

impl PlotBackend for KittyBackend {
    fn set_bg(&mut self, x: u16, y: u16, color: Color, _z: u8) {
        let (px, py) = self.cell_to_px(x, y);
        let (r, g, b) = color_to_rgb(color);
        self.fill_rect(px, py, CELL_PX_W, CELL_PX_H, (r, g, b));
    }

    fn set_char(&mut self, x: u16, y: u16, ch: char, fg: Color, z: u8) {
        // Text elements rendered to ratatui cells, not pixels
        if let Some(i) = self.cell_index(x, y) {
            if self.text_cells[i].is_none_or(|(_, _, _, ez)| z >= ez) {
                self.text_cells[i] = Some((ch, fg, Color::Reset, z));
            }
        }
    }

    fn set_cell(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Color, z: u8) {
        // Half-block cells: render as pixels (top/bottom halves)
        let (px, py) = self.cell_to_px(x, y);
        if ch == '\u{2580}' {
            // ▀ upper half: fg=top, bg=bottom
            let (r, g, b) = color_to_rgb(fg);
            self.fill_rect(px, py, CELL_PX_W, CELL_PX_H / 2, (r, g, b));
            let (r, g, b) = color_to_rgb(bg);
            self.fill_rect(px, py + CELL_PX_H / 2, CELL_PX_W, CELL_PX_H / 2, (r, g, b));
        } else if ch == '\u{2584}' {
            // ▄ lower half
            let (r, g, b) = color_to_rgb(bg);
            self.fill_rect(px, py, CELL_PX_W, CELL_PX_H / 2, (r, g, b));
            let (r, g, b) = color_to_rgb(fg);
            self.fill_rect(px, py + CELL_PX_H / 2, CELL_PX_W, CELL_PX_H / 2, (r, g, b));
        } else if ch == '\u{2588}' {
            // █ full block
            let (r, g, b) = color_to_rgb(fg);
            self.fill_rect(px, py, CELL_PX_W, CELL_PX_H, (r, g, b));
        } else {
            // Other characters: render as text
            self.set_char(x, y, ch, fg, z);
            self.set_bg(x, y, bg, z);
        }
    }

    fn set_braille(&mut self, x: u16, y: u16, bits: u8, fg: Color, _z: u8) {
        // Translate braille dots to pixel positions within the cell
        let (px, py) = self.cell_to_px(x, y);
        let (r, g, b) = color_to_rgb(fg);
        let dot_w = CELL_PX_W / 2;
        let dot_h = CELL_PX_H / 4;

        // Braille dot layout: column 0 bits [0x01, 0x02, 0x04, 0x40], column 1 bits [0x08, 0x10, 0x20, 0x80]
        let col0_bits = [0x01u8, 0x02, 0x04, 0x40];
        let col1_bits = [0x08u8, 0x10, 0x20, 0x80];

        for (row, (&b0, &b1)) in col0_bits.iter().zip(col1_bits.iter()).enumerate() {
            let row_py = py + row as u32 * dot_h;
            if bits & b0 != 0 {
                self.fill_rect(px, row_py, dot_w, dot_h, (r, g, b));
            }
            if bits & b1 != 0 {
                self.fill_rect(px + dot_w, row_py, dot_w, dot_h, (r, g, b));
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, color: Color, pa: &PlotArea, _z: u8) {
        // Anti-aliased pixel-level line drawing (Bresenham at pixel resolution)
        let (r, g, b) = color_to_rgb(color);

        // Convert cell coordinates to pixel coordinates
        let px0 = ((x0 - self.area.x as f64) * CELL_PX_W as f64).round() as i32;
        let py0 = ((y0 - self.area.y as f64) * CELL_PX_H as f64).round() as i32;
        let px1 = ((x1 - self.area.x as f64) * CELL_PX_W as f64).round() as i32;
        let py1 = ((y1 - self.area.y as f64) * CELL_PX_H as f64).round() as i32;

        // Clip bounds in pixel space
        let clip_x0 = ((pa.x - self.area.x) as i32) * CELL_PX_W as i32;
        let clip_y0 = ((pa.y - self.area.y) as i32) * CELL_PX_H as i32;
        let clip_x1 = clip_x0 + pa.width as i32 * CELL_PX_W as i32;
        let clip_y1 = clip_y0 + pa.height as i32 * CELL_PX_H as i32;

        // Bresenham line at pixel resolution
        let mut cx = px0;
        let mut cy = py0;
        let dx = (px1 - px0).abs();
        let dy = -(py1 - py0).abs();
        let sx = if px0 < px1 { 1 } else { -1 };
        let sy = if py0 < py1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if cx >= clip_x0 && cx < clip_x1 && cy >= clip_y0 && cy < clip_y1 {
                self.set_pixel(cx as u32, cy as u32, r, g, b);
            }

            if cx == px1 && cy == py1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                cx += sx;
            }
            if e2 <= dx {
                err += dx;
                cy += sy;
            }
        }
    }

    fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.area.x
            && x < self.area.x + self.area.width
            && y >= self.area.y
            && y < self.area.y + self.area.height
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn composite(&self, buf: &mut Buffer) {
        // Encode pixel buffer as PNG
        let Ok(png_data) = encode_rgba_png(&self.pixels, self.px_width, self.px_height) else {
            // Fall back: just render text cells
            for y in self.area.y..self.area.y + self.area.height {
                for x in self.area.x..self.area.x + self.area.width {
                    if let Some(i) = self.cell_index(x, y) {
                        if let Some((ch, fg, bg, _)) = self.text_cells[i] {
                            buf[(x, y)].set_char(ch).set_fg(fg).set_bg(bg);
                        }
                    }
                }
            }
            return;
        };

        // Build Kitty escape sequence
        let b64 = base64_encode(&png_data);
        let kitty_data = build_kitty_escape(&b64, self.px_width, self.px_height);

        // Write Kitty data to first cell, skip remaining cells
        if self.area.width > 0 && self.area.height > 0 {
            let first_x = self.area.x;
            let first_y = self.area.y;
            // Store the escape sequence in the first cell's symbol
            buf[(first_x, first_y)].set_symbol(&kitty_data);

            // Mark all other cells as empty (Kitty image covers them)
            for y in self.area.y..self.area.y + self.area.height {
                for x in self.area.x..self.area.x + self.area.width {
                    if x == first_x && y == first_y {
                        continue;
                    }
                    buf[(x, y)].set_skip(true);
                }
            }
        }

        // Overlay text cells on top (axis labels, legends, etc.)
        for y in self.area.y..self.area.y + self.area.height {
            for x in self.area.x..self.area.x + self.area.width {
                if let Some(i) = self.cell_index(x, y) {
                    if let Some((ch, fg, _bg, _)) = self.text_cells[i] {
                        if ch != ' ' {
                            buf[(x, y)].set_char(ch).set_fg(fg).set_skip(false);
                        }
                    }
                }
            }
        }
    }
}

/// Encode RGBA pixel data as PNG bytes using the image crate.
fn encode_rgba_png(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, Rgba};

    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, pixels.to_vec())
            .ok_or_else(|| "invalid image dimensions".to_string())?;

    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("PNG encode: {e}"))?;
    Ok(buf.into_inner())
}

/// Base64-encode a byte slice.
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Build a Kitty graphics protocol escape sequence for direct image display.
fn build_kitty_escape(b64: &str, width: u32, height: u32) -> String {
    let mut result = String::new();

    // Transmit in chunks of 4096 bytes
    let chunks: Vec<&str> = b64
        .as_bytes()
        .chunks(4096)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let more = if i < chunks.len() - 1 { 1 } else { 0 };
        if i == 0 {
            // First chunk: include image metadata
            result.push_str(&format!(
                "\x1b_Ga=T,f=100,s={width},v={height},m={more};{chunk}\x1b\\"
            ));
        } else {
            // Continuation chunks
            result.push_str(&format!("\x1b_Gm={more};{chunk}\x1b\\"));
        }
    }

    result
}
