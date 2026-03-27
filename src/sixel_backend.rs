//! Sixel graphics protocol rendering backend.
//!
//! Renders plot data to an internal pixel buffer, then composites as a Sixel
//! image during [`PlotBackend::composite`]. Provides pixel-level rendering
//! with palette-based color (up to 256 colors).
//!
//! Requires the `sixel` feature and a Sixel-compatible terminal (foot,
//! WezTerm, mlterm, xterm with `-ti vt340`, contour).

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

/// Pixel-level rendering backend using the Sixel graphics protocol.
///
/// Maintains an internal RGB pixel buffer. During [`composite`](PlotBackend::composite),
/// the buffer is quantized to a 256-color palette and encoded as Sixel escape sequences.
///
/// Text elements (axis labels, legends) are rendered to ratatui cells. Data elements
/// (lines, fills, markers) use pixel rendering.
pub struct SixelBackend {
    area: Rect,
    /// RGB pixel buffer: 3 bytes per pixel, row-major.
    pixels: Vec<u8>,
    /// Width in pixels.
    px_width: u32,
    /// Height in pixels.
    px_height: u32,
    /// Track which pixels have been written (for transparency).
    written: Vec<bool>,
    /// Per-cell Z-tracking for text elements.
    text_cells: Vec<Option<(char, Color, Color, u8)>>,
}

impl SixelBackend {
    /// Create a new Sixel backend covering the given terminal area.
    pub fn new(area: Rect) -> Self {
        let px_width = area.width as u32 * CELL_PX_W;
        let px_height = area.height as u32 * CELL_PX_H;
        let pixel_count = (px_width * px_height) as usize;
        Self {
            area,
            pixels: vec![0u8; pixel_count * 3],
            px_width,
            px_height,
            written: vec![false; pixel_count],
            text_cells: vec![None; area.width as usize * area.height as usize],
        }
    }

    /// Set a single pixel to the given color.
    fn set_pixel(&mut self, px: u32, py: u32, r: u8, g: u8, b: u8) {
        if px < self.px_width && py < self.px_height {
            let i = (py * self.px_width + px) as usize;
            let bi = i * 3;
            self.pixels[bi] = r;
            self.pixels[bi + 1] = g;
            self.pixels[bi + 2] = b;
            self.written[i] = true;
        }
    }

    /// Fill a rectangular region of pixels.
    fn fill_rect(&mut self, px: u32, py: u32, w: u32, h: u32, rgb: (u8, u8, u8)) {
        for dy in 0..h {
            for dx in 0..w {
                self.set_pixel(px + dx, py + dy, rgb.0, rgb.1, rgb.2);
            }
        }
    }

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

    fn cell_to_px(&self, x: u16, y: u16) -> (u32, u32) {
        let px = (x.saturating_sub(self.area.x)) as u32 * CELL_PX_W;
        let py = (y.saturating_sub(self.area.y)) as u32 * CELL_PX_H;
        (px, py)
    }
}

impl PlotBackend for SixelBackend {
    fn set_bg(&mut self, x: u16, y: u16, color: Color, _z: u8) {
        let (px, py) = self.cell_to_px(x, y);
        let (r, g, b) = color_to_rgb(color);
        self.fill_rect(px, py, CELL_PX_W, CELL_PX_H, (r, g, b));
    }

    fn set_char(&mut self, x: u16, y: u16, ch: char, fg: Color, z: u8) {
        if let Some(i) = self.cell_index(x, y) {
            if self.text_cells[i].is_none_or(|(_, _, _, ez)| z >= ez) {
                self.text_cells[i] = Some((ch, fg, Color::Reset, z));
            }
        }
    }

    fn set_cell(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Color, z: u8) {
        let (px, py) = self.cell_to_px(x, y);
        if ch == '\u{2580}' {
            let (r, g, b) = color_to_rgb(fg);
            self.fill_rect(px, py, CELL_PX_W, CELL_PX_H / 2, (r, g, b));
            let (r, g, b) = color_to_rgb(bg);
            self.fill_rect(px, py + CELL_PX_H / 2, CELL_PX_W, CELL_PX_H / 2, (r, g, b));
        } else if ch == '\u{2584}' {
            let (r, g, b) = color_to_rgb(bg);
            self.fill_rect(px, py, CELL_PX_W, CELL_PX_H / 2, (r, g, b));
            let (r, g, b) = color_to_rgb(fg);
            self.fill_rect(px, py + CELL_PX_H / 2, CELL_PX_W, CELL_PX_H / 2, (r, g, b));
        } else if ch == '\u{2588}' {
            let (r, g, b) = color_to_rgb(fg);
            self.fill_rect(px, py, CELL_PX_W, CELL_PX_H, (r, g, b));
        } else {
            self.set_char(x, y, ch, fg, z);
            self.set_bg(x, y, bg, z);
        }
    }

    fn set_braille(&mut self, x: u16, y: u16, bits: u8, fg: Color, _z: u8) {
        let (px, py) = self.cell_to_px(x, y);
        let (r, g, b) = color_to_rgb(fg);
        let dot_w = CELL_PX_W / 2;
        let dot_h = CELL_PX_H / 4;

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
        let (r, g, b) = color_to_rgb(color);

        let px0 = ((x0 - self.area.x as f64) * CELL_PX_W as f64).round() as i32;
        let py0 = ((y0 - self.area.y as f64) * CELL_PX_H as f64).round() as i32;
        let px1 = ((x1 - self.area.x as f64) * CELL_PX_W as f64).round() as i32;
        let py1 = ((y1 - self.area.y as f64) * CELL_PX_H as f64).round() as i32;

        let clip_x0 = ((pa.x - self.area.x) as i32) * CELL_PX_W as i32;
        let clip_y0 = ((pa.y - self.area.y) as i32) * CELL_PX_H as i32;
        let clip_x1 = clip_x0 + pa.width as i32 * CELL_PX_W as i32;
        let clip_y1 = clip_y0 + pa.height as i32 * CELL_PX_H as i32;

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
        // Build Sixel escape sequence from pixel data
        let sixel_data = encode_sixel(&self.pixels, self.px_width, self.px_height);

        // Write to first cell, skip rest
        if self.area.width > 0 && self.area.height > 0 {
            buf[(self.area.x, self.area.y)].set_symbol(&sixel_data);

            for y in self.area.y..self.area.y + self.area.height {
                for x in self.area.x..self.area.x + self.area.width {
                    if x == self.area.x && y == self.area.y {
                        continue;
                    }
                    buf[(x, y)].set_skip(true);
                }
            }
        }

        // Overlay text cells
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

/// Encode RGB pixel data as a Sixel escape sequence.
///
/// Performs simple palette quantization (up to 256 unique colors) and
/// generates the Sixel protocol output.
fn encode_sixel(pixels: &[u8], width: u32, height: u32) -> String {
    use std::collections::HashMap;

    // Quantize colors: collect unique RGB values (limit 256)
    let mut palette: HashMap<(u8, u8, u8), u16> = HashMap::new();
    let mut palette_list: Vec<(u8, u8, u8)> = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize * 3;
            let rgb = (pixels[i], pixels[i + 1], pixels[i + 2]);

            // Reduce to 6-bit per channel for palette compression
            let quantized = (rgb.0 & 0xFC, rgb.1 & 0xFC, rgb.2 & 0xFC);

            if !palette.contains_key(&quantized) && palette_list.len() < 256 {
                palette.insert(quantized, palette_list.len() as u16);
                palette_list.push(quantized);
            }
        }
    }

    let mut out = String::new();

    // Sixel header: P7;1q (raster attributes)
    out.push_str(&format!("\x1bP7;1q\"1;1;{width};{height}"));

    // Define palette
    for (idx, &(r, g, b)) in palette_list.iter().enumerate() {
        let pr = (r as u32 * 100) / 255;
        let pg = (g as u32 * 100) / 255;
        let pb = (b as u32 * 100) / 255;
        out.push_str(&format!("#{idx};2;{pr};{pg};{pb}"));
    }

    // Encode pixel data in 6-row bands
    let mut y: u32 = 0;
    while y < height {
        let band_height = (height - y).min(6);

        // For each color in the palette, emit the sixel row
        for (color_idx, &color_rgb) in palette_list.iter().enumerate() {
            let mut has_pixels = false;
            let mut row_data = Vec::with_capacity(width as usize);

            for x in 0..width {
                let mut sixel_bits: u8 = 0;
                for dy in 0..band_height {
                    let py = y + dy;
                    let i = (py * width + x) as usize * 3;
                    let prgb = (
                        pixels[i] & 0xFC,
                        pixels[i + 1] & 0xFC,
                        pixels[i + 2] & 0xFC,
                    );
                    if prgb == color_rgb {
                        sixel_bits |= 1 << dy;
                        has_pixels = true;
                    }
                }
                row_data.push(sixel_bits + 63); // Sixel encoding: value + 63
            }

            if has_pixels {
                out.push_str(&format!("#{color_idx}"));
                for &b in &row_data {
                    out.push(b as char);
                }
                out.push('$'); // Carriage return within band
            }
        }
        out.push('-'); // New band (line feed)
        y += 6;
    }

    // Sixel terminator
    out.push_str("\x1b\\");

    out
}
