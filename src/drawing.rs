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

/// Horizontal fill levels using left-side eighth blocks.
/// Index 0 = empty, index 8 = full block.
/// Characters: ▏▎▍▌▋▊▉█
#[cfg(feature = "unicode-extended")]
pub const HORIZONTAL_FILL_LEVELS: [char; 9] = [
    ' ',        // 0/8
    '\u{258F}', // ▏ LEFT ONE EIGHTH BLOCK
    '\u{258E}', // ▎ LEFT ONE QUARTER BLOCK
    '\u{258D}', // ▍ LEFT THREE EIGHTHS BLOCK
    '\u{258C}', // ▌ LEFT HALF BLOCK
    '\u{258B}', // ▋ LEFT FIVE EIGHTHS BLOCK
    '\u{258A}', // ▊ LEFT THREE QUARTERS BLOCK
    '\u{2589}', // ▉ LEFT SEVEN EIGHTHS BLOCK
    '\u{2588}', // █ FULL BLOCK
];

/// Convenience: map a 0.0..=1.0 fraction to a horizontal fill character.
#[cfg(feature = "unicode-extended")]
pub fn horizontal_fill_char(fraction: f64) -> char {
    let level = (fraction.clamp(0.0, 1.0) * 8.0).round() as usize;
    HORIZONTAL_FILL_LEVELS[level.min(8)]
}

/// Vertical fill levels using lower eighth blocks.
/// Index 0 = empty, index 8 = full block.
/// Characters: ▁▂▃▄▅▆▇█
#[cfg(feature = "unicode-extended")]
pub const VERTICAL_FILL_LEVELS: [char; 9] = [
    ' ',        // 0/8
    '\u{2581}', // ▁ LOWER ONE EIGHTH BLOCK
    '\u{2582}', // ▂ LOWER ONE QUARTER BLOCK
    '\u{2583}', // ▃ LOWER THREE EIGHTHS BLOCK
    '\u{2584}', // ▄ LOWER HALF BLOCK
    '\u{2585}', // ▅ LOWER FIVE EIGHTHS BLOCK
    '\u{2586}', // ▆ LOWER THREE QUARTERS BLOCK
    '\u{2587}', // ▇ LOWER SEVEN EIGHTHS BLOCK
    '\u{2588}', // █ FULL BLOCK
];

/// Convenience: map a 0.0..=1.0 fraction to a vertical fill character.
#[cfg(feature = "unicode-extended")]
pub fn vertical_fill_char(fraction: f64) -> char {
    let level = (fraction.clamp(0.0, 1.0) * 8.0).round() as usize;
    VERTICAL_FILL_LEVELS[level.min(8)]
}

/// Sextant block lookup: maps a 6-bit pattern (2x3 grid) to a Unicode sextant character.
/// Bit layout: bit 0 = top-left, bit 1 = top-right, bit 2 = mid-left, bit 3 = mid-right,
/// bit 4 = bottom-left, bit 5 = bottom-right.
#[cfg(feature = "unicode-extended")]
pub fn sextant_char(bits: u8) -> char {
    const TABLE: [char; 64] = [
        ' ',         '\u{1FB00}', '\u{1FB01}', '\u{1FB02}',
        '\u{1FB03}', '\u{1FB04}', '\u{1FB05}', '\u{1FB06}',
        '\u{1FB07}', '\u{1FB08}', '\u{1FB09}', '\u{1FB0A}',
        '\u{1FB0B}', '\u{1FB0C}', '\u{1FB0D}', '\u{1FB0E}',
        '\u{1FB0F}', '\u{1FB10}', '\u{1FB11}', '\u{1FB12}',
        '\u{1FB13}', '\u{2580}',  '\u{1FB14}', '\u{1FB15}',
        '\u{1FB16}', '\u{1FB17}', '\u{1FB18}', '\u{1FB19}',
        '\u{1FB1A}', '\u{1FB1B}', '\u{1FB1C}', '\u{1FB1D}',
        '\u{1FB1E}', '\u{1FB1F}', '\u{1FB20}', '\u{1FB21}',
        '\u{1FB22}', '\u{1FB23}', '\u{1FB24}', '\u{1FB25}',
        '\u{1FB26}', '\u{1FB27}', '\u{2584}',  '\u{1FB28}',
        '\u{1FB29}', '\u{1FB2A}', '\u{1FB2B}', '\u{1FB2C}',
        '\u{1FB2D}', '\u{1FB2E}', '\u{1FB2F}', '\u{1FB30}',
        '\u{1FB31}', '\u{1FB32}', '\u{1FB33}', '\u{1FB34}',
        '\u{1FB35}', '\u{1FB36}', '\u{1FB37}', '\u{1FB38}',
        '\u{1FB39}', '\u{1FB3A}', '\u{1FB3B}', '\u{2588}',
    ];
    TABLE[(bits & 0x3F) as usize]
}

/// Quadrant block lookup: maps a 4-bit pattern (2x2 grid) to a quadrant block character.
/// Bit layout: bit 0 = top-left, bit 1 = top-right, bit 2 = bottom-left, bit 3 = bottom-right.
#[cfg(feature = "unicode-extended")]
pub fn quadrant_char(bits: u8) -> char {
    const TABLE: [char; 16] = [
        ' ',        // 0b0000
        '\u{2598}', // ▘ 0b0001 QUADRANT UPPER LEFT
        '\u{259D}', // ▝ 0b0010 QUADRANT UPPER RIGHT
        '\u{2580}', // ▀ 0b0011 UPPER HALF BLOCK
        '\u{2596}', // ▖ 0b0100 QUADRANT LOWER LEFT
        '\u{258C}', // ▌ 0b0101 LEFT HALF BLOCK
        '\u{259E}', // ▞ 0b0110 QUADRANT UPPER RIGHT AND LOWER LEFT
        '\u{259B}', // ▛ 0b0111 QUADRANT UPPER LEFT AND UPPER RIGHT AND LOWER LEFT
        '\u{2597}', // ▗ 0b1000 QUADRANT LOWER RIGHT
        '\u{259A}', // ▚ 0b1001 QUADRANT UPPER LEFT AND LOWER RIGHT
        '\u{2590}', // ▐ 0b1010 RIGHT HALF BLOCK
        '\u{259C}', // ▜ 0b1011 QUADRANT UPPER LEFT AND UPPER RIGHT AND LOWER RIGHT
        '\u{2584}', // ▄ 0b1100 LOWER HALF BLOCK
        '\u{2599}', // ▙ 0b1101 QUADRANT UPPER LEFT AND LOWER LEFT AND LOWER RIGHT
        '\u{259F}', // ▟ 0b1110 QUADRANT UPPER RIGHT AND LOWER LEFT AND LOWER RIGHT
        '\u{2588}', // █ 0b1111 FULL BLOCK
    ];
    TABLE[(bits & 0x0F) as usize]
}

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
pub fn draw_braille_line(
    buf: &mut Buffer,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
    pa: &PlotArea,
) {
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
