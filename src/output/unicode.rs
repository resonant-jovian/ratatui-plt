//! Downsample a tiny-skia pixmap to Unicode terminal characters.
//!
//! Two modes: half-block (2x vertical resolution, full color) and
//! braille (2x4 binary dots per cell, luminance thresholding).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use tiny_skia::Pixmap;

use super::{CELL_PX_H, CELL_PX_W, UnicodeMode};

/// Braille sub-pixel bit layout for each column/row within a cell.
/// Braille characters (U+2800..=U+28FF) encode 8 dots in a 2x4 grid.
const BRAILLE_BITS: [[u8; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40], // column 0: rows 0-3
    [0x08, 0x10, 0x20, 0x80], // column 1: rows 0-3
];
const BRAILLE_BASE: u32 = 0x2800;

/// Write a pixmap to a ratatui Buffer using Unicode characters.
pub fn pixmap_to_buf(
    area: Rect,
    buf: &mut Buffer,
    pixmap: &Pixmap,
    mode: UnicodeMode,
) {
    match mode {
        UnicodeMode::HalfBlock => halfblock_to_buf(area, buf, pixmap),
        UnicodeMode::Braille => braille_to_buf(area, buf, pixmap),
    }
}

/// Half-block mode: each cell encodes two vertical pixels using `▀`.
///
/// The top half of the cell's pixel region → foreground color.
/// The bottom half → background color. This gives 2x vertical
/// resolution with full RGB color in both halves.
fn halfblock_to_buf(area: Rect, buf: &mut Buffer, pixmap: &Pixmap) {
    let img_w = pixmap.width();
    let img_h = pixmap.height();
    let pixels = pixmap.data();
    let half_h = CELL_PX_H / 2;

    for row in 0..area.height {
        for col in 0..area.width {
            let px_x = col as u32 * CELL_PX_W;
            let px_y = row as u32 * CELL_PX_H;

            let (tr, tg, tb) =
                avg_region(pixels, img_w, img_h, px_x, px_y, CELL_PX_W, half_h);
            let (br, bg_r, bb) =
                avg_region(pixels, img_w, img_h, px_x, px_y + half_h, CELL_PX_W, half_h);

            if let Some(cell) =
                buf.cell_mut((area.x + col, area.y + row))
            {
                cell.set_symbol("\u{2580}");
                cell.set_fg(Color::Rgb(tr, tg, tb));
                cell.set_bg(Color::Rgb(br, bg_r, bb));
            }
        }
    }
}

#[allow(clippy::manual_checked_ops)]
/// Braille mode: each cell encodes 2x4 binary dots.
///
/// The cell's pixel region is divided into a 2x4 grid of sub-cells
/// (each covering `CELL_PX_W/2` x `CELL_PX_H/4` pixels). Each
/// sub-cell's average luminance is compared against a threshold to
/// decide whether the dot is "on" or "off".
fn braille_to_buf(area: Rect, buf: &mut Buffer, pixmap: &Pixmap) {
    let img_w = pixmap.width();
    let img_h = pixmap.height();
    let pixels = pixmap.data();
    let sub_w = CELL_PX_W / 2;
    let sub_h = CELL_PX_H / 4;

    for row in 0..area.height {
        for col in 0..area.width {
            let px_x = col as u32 * CELL_PX_W;
            let px_y = row as u32 * CELL_PX_H;

            // Compute luminance for each of the 8 sub-cells and
            // the overall cell background luminance.
            let bg_lum = avg_luminance(
                pixels, img_w, img_h, px_x, px_y, CELL_PX_W, CELL_PX_H,
            );

            let mut bits: u8 = 0;
            let mut fg_r: u32 = 0;
            let mut fg_g: u32 = 0;
            let mut fg_b: u32 = 0;
            let mut fg_count: u32 = 0;

            for bcol in 0..2u32 {
                for brow in 0..4u32 {
                    let sx = px_x + bcol * sub_w;
                    let sy = px_y + brow * sub_h;
                    let lum = avg_luminance(
                        pixels, img_w, img_h, sx, sy, sub_w, sub_h,
                    );
                    // Dot is "on" if its luminance differs from the
                    // background by more than a threshold.
                    if (lum - bg_lum).abs() > 0.08 {
                        bits |= BRAILLE_BITS[bcol as usize][brow as usize];
                        let (r, g, b) = avg_region(
                            pixels, img_w, img_h, sx, sy, sub_w, sub_h,
                        );
                        fg_r += r as u32;
                        fg_g += g as u32;
                        fg_b += b as u32;
                        fg_count += 1;
                    }
                }
            }

            if let Some(cell) =
                buf.cell_mut((area.x + col, area.y + row))
            {
                let ch = char::from_u32(BRAILLE_BASE | bits as u32)
                    .unwrap_or(' ');
                cell.set_symbol(&ch.to_string());

                if fg_count > 0 {
                    cell.set_fg(Color::Rgb(
                        (fg_r / fg_count).min(255) as u8,
                        (fg_g / fg_count).min(255) as u8,
                        (fg_b / fg_count).min(255) as u8,
                    ));
                }

                let (br, bg_val, bb) = avg_region(
                    pixels, img_w, img_h, px_x, px_y, CELL_PX_W, CELL_PX_H,
                );
                cell.set_bg(Color::Rgb(br, bg_val, bb));
            }
        }
    }
}

/// Average the RGB values of pixels in a rectangular region.
///
/// Returns (r, g, b) as u8 values. Handles pre-multiplied alpha
/// by un-premultiplying before averaging.
#[allow(clippy::manual_checked_ops)]
fn avg_region(
    pixels: &[u8],
    img_w: u32,
    img_h: u32,
    x0: u32,
    y0: u32,
    w: u32,
    h: u32,
) -> (u8, u8, u8) {
    let mut r_sum: u64 = 0;
    let mut g_sum: u64 = 0;
    let mut b_sum: u64 = 0;
    let mut count: u64 = 0;
    for py in y0..y0 + h {
        for px in x0..x0 + w {
            if px < img_w && py < img_h {
                let idx = (py * img_w + px) as usize * 4;
                if idx + 3 < pixels.len() {
                    let a = pixels[idx + 3] as u64;
                    if a > 0 {
                        r_sum += pixels[idx] as u64 * 255 / a;
                        g_sum += pixels[idx + 1] as u64 * 255 / a;
                        b_sum += pixels[idx + 2] as u64 * 255 / a;
                        count += 1;
                    }
                }
            }
        }
    }
    if count == 0 {
        (30, 30, 30)
    } else {
        (
            (r_sum / count).min(255) as u8,
            (g_sum / count).min(255) as u8,
            (b_sum / count).min(255) as u8,
        )
    }
}

/// Compute the average luminance (0.0..=1.0) of a rectangular region.
fn avg_luminance(
    pixels: &[u8],
    img_w: u32,
    img_h: u32,
    x0: u32,
    y0: u32,
    w: u32,
    h: u32,
) -> f64 {
    let (r, g, b) = avg_region(pixels, img_w, img_h, x0, y0, w, h);
    // sRGB relative luminance
    0.2126 * (r as f64 / 255.0)
        + 0.7152 * (g as f64 / 255.0)
        + 0.0722 * (b as f64 / 255.0)
}
