//! Shared drawing primitives for Braille sub-pixel line rendering.
//!
//! Provides high-resolution line drawing using Unicode Braille characters (U+2800..=U+28FF),
//! which encode 8 dots in a 2x4 grid per terminal cell, giving 2x horizontal and 4x vertical
//! sub-pixel resolution.

use ratatui::buffer::Buffer;
use ratatui::style::Color;

use crate::frame::PlotArea;

/// Braille sub-pixel bit layout for each column/row within a cell.
/// Braille characters (U+2800..=U+28FF) encode 8 dots in a 2x4 grid.
pub const BRAILLE_BITS: [[u8; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40], // column 0: rows 0-3
    [0x08, 0x10, 0x20, 0x80], // column 1: rows 0-3
];
pub const BRAILLE_BASE: u32 = 0x2800;

/// OR a braille dot into the buffer cell, preserving existing dots.
pub fn write_braille(buf: &mut Buffer, x: u16, y: u16, bits: u8, color: Color) {
    let existing = {
        let ch = buf[(x, y)].symbol().chars().next().unwrap_or(' ');
        let code = ch as u32;
        if (BRAILLE_BASE..=0x28FF).contains(&code) {
            (code - BRAILLE_BASE) as u8
        } else {
            0
        }
    };
    let combined = existing | bits;
    if let Some(ch) = char::from_u32(BRAILLE_BASE + combined as u32) {
        buf[(x, y)].set_char(ch).set_fg(color);
    }
}

/// Draw a line between two screen-space points using Bresenham's algorithm
/// at Braille sub-pixel resolution (2x4 per cell).
///
/// Coordinates are in terminal cell space (floating point). The line is clipped
/// to the given plot area bounds.
pub fn draw_braille_line(buf: &mut Buffer, x0: f64, y0: f64, x1: f64, y1: f64, color: Color, pa: &PlotArea) {
    // Scale to braille sub-pixel coordinates (2x horizontal, 4x vertical)
    let mut ix0 = (x0 * 2.0).round() as i32;
    let mut iy0 = (y0 * 4.0).round() as i32;
    let ix1 = (x1 * 2.0).round() as i32;
    let iy1 = (y1 * 4.0).round() as i32;

    let dx = (ix1 - ix0).abs();
    let dy = -(iy1 - iy0).abs();
    let sx = if ix0 < ix1 { 1 } else { -1 };
    let sy = if iy0 < iy1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if ix0 >= 0 && iy0 >= 0 {
            let cell_x = (ix0 / 2) as u16;
            let cell_y = (iy0 / 4) as u16;
            if cell_x >= pa.x
                && cell_x < pa.x + pa.width
                && cell_y >= pa.y
                && cell_y < pa.y + pa.height
            {
                let dot_col = (ix0 % 2) as usize;
                let dot_row = (iy0 % 4) as usize;
                let bit = BRAILLE_BITS[dot_col][dot_row];
                write_braille(buf, cell_x, cell_y, bit, color);
            }
        }

        if ix0 == ix1 && iy0 == iy1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            ix0 += sx;
        }
        if e2 <= dx {
            err += dx;
            iy0 += sy;
        }
    }
}
