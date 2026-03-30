//! Sixel graphics protocol output adapter.
//!
//! Quantizes a tiny-skia pixmap to a 256-color palette and encodes
//! it as Sixel escape sequences for display in Sixel-capable terminals.

use std::collections::HashMap;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tiny_skia::Pixmap;

/// Write a pixmap to a ratatui Buffer via Sixel graphics protocol.
pub fn pixmap_to_sixel(area: Rect, buf: &mut Buffer, pixmap: &Pixmap) {
    let width = pixmap.width();
    let height = pixmap.height();

    // Convert pre-multiplied RGBA to plain RGB.
    let rgb_pixels = premultiplied_to_rgb(pixmap.data(), width, height);
    let sixel_data = encode_sixel(&rgb_pixels, width, height);

    // Write to first cell, skip rest.
    if area.width > 0 && area.height > 0 {
        if let Some(cell) = buf.cell_mut((area.x, area.y)) {
            cell.set_symbol(&sixel_data);
        }

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if x == area.x && y == area.y {
                    continue;
                }
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_skip(true);
                }
            }
        }
    }
}

/// Convert pre-multiplied RGBA pixel data to plain RGB.
#[allow(clippy::manual_checked_ops)]
fn premultiplied_to_rgb(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut rgb = vec![0u8; pixel_count * 3];
    for i in 0..pixel_count {
        let si = i * 4;
        let di = i * 3;
        if let (Some(&r), Some(&g), Some(&b), Some(&a)) = (
            data.get(si),
            data.get(si + 1),
            data.get(si + 2),
            data.get(si + 3),
        ) {
            let a16 = a as u16;
            if a16 > 0 {
                rgb[di] = ((r as u16 * 255) / a16).min(255) as u8;
                rgb[di + 1] = ((g as u16 * 255) / a16).min(255) as u8;
                rgb[di + 2] = ((b as u16 * 255) / a16).min(255) as u8;
            }
        }
    }
    rgb
}

/// Encode RGB pixel data as a Sixel escape sequence.
///
/// Performs simple palette quantization (up to 256 unique colors via
/// 6-bit-per-channel reduction) and generates Sixel protocol output.
fn encode_sixel(pixels: &[u8], width: u32, height: u32) -> String {
    let mut palette: HashMap<(u8, u8, u8), u16> = HashMap::new();
    let mut palette_list: Vec<(u8, u8, u8)> = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) as usize * 3;
            let quantized = (
                pixels[i] & 0xFC,
                pixels[i + 1] & 0xFC,
                pixels[i + 2] & 0xFC,
            );

            if !palette.contains_key(&quantized) && palette_list.len() < 256 {
                palette.insert(quantized, palette_list.len() as u16);
                palette_list.push(quantized);
            }
        }
    }

    let mut out = String::new();

    // Sixel header
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
                row_data.push(sixel_bits + 63);
            }

            if has_pixels {
                out.push_str(&format!("#{color_idx}"));
                for &b in &row_data {
                    out.push(b as char);
                }
                out.push('$');
            }
        }
        out.push('-');
        y += 6;
    }

    // Sixel terminator
    out.push_str("\x1b\\");

    out
}
