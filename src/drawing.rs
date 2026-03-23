//! Shared drawing primitives for Braille sub-pixel line rendering.
//!
//! Provides high-resolution line drawing using Unicode Braille characters (U+2800..=U+28FF),
//! which encode 8 dots in a 2x4 grid per terminal cell, giving 2x horizontal and 4x vertical
//! sub-pixel resolution.

use ratatui::buffer::Buffer;
use ratatui::style::Color;

use crate::frame::PlotArea;
use crate::plot_buffer::PlotBuffer;

/// Braille sub-pixel bit layout for each column/row within a cell.
/// Braille characters (U+2800..=U+28FF) encode 8 dots in a 2x4 grid.
pub const BRAILLE_BITS: [[u8; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40], // column 0: rows 0-3
    [0x08, 0x10, 0x20, 0x80], // column 1: rows 0-3
];
pub const BRAILLE_BASE: u32 = 0x2800;

/// Horizontal fill levels using left-side eighth blocks.
#[cfg(feature = "unicode-extended")]
pub const HORIZONTAL_FILL_LEVELS: [char; 9] = [
    ' ', '\u{258F}', '\u{258E}', '\u{258D}', '\u{258C}',
    '\u{258B}', '\u{258A}', '\u{2589}', '\u{2588}',
];

/// Convenience: map a 0.0..=1.0 fraction to a horizontal fill character.
#[cfg(feature = "unicode-extended")]
pub fn horizontal_fill_char(fraction: f64) -> char {
    let level = (fraction.clamp(0.0, 1.0) * 8.0).round() as usize;
    HORIZONTAL_FILL_LEVELS[level.min(8)]
}

/// Vertical fill levels using lower eighth blocks.
#[cfg(feature = "unicode-extended")]
pub const VERTICAL_FILL_LEVELS: [char; 9] = [
    ' ', '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}',
    '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}',
];

/// Convenience: map a 0.0..=1.0 fraction to a vertical fill character.
#[cfg(feature = "unicode-extended")]
pub fn vertical_fill_char(fraction: f64) -> char {
    let level = (fraction.clamp(0.0, 1.0) * 8.0).round() as usize;
    VERTICAL_FILL_LEVELS[level.min(8)]
}

/// Sextant block lookup: maps a 6-bit pattern (2x3 grid) to a Unicode sextant character.
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
#[cfg(feature = "unicode-extended")]
pub fn quadrant_char(bits: u8) -> char {
    const TABLE: [char; 16] = [
        ' ',        '\u{2598}', '\u{259D}', '\u{2580}',
        '\u{2596}', '\u{258C}', '\u{259E}', '\u{259B}',
        '\u{2597}', '\u{259A}', '\u{2590}', '\u{259C}',
        '\u{2584}', '\u{2599}', '\u{259F}', '\u{2588}',
    ];
    TABLE[(bits & 0x0F) as usize]
}

/// Check if two colors are effectively the same (for contrast detection).
pub fn colors_match(a: Color, b: Color) -> bool {
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            r1 == r2 && g1 == g2 && b1 == b2
        }
        (Color::Reset, Color::Reset) => true,
        _ => a == b,
    }
}

/// Return a contrasting color (white for dark colors, black for light).
pub fn contrasting_color(c: Color) -> Color {
    match c {
        Color::Rgb(r, g, b) => {
            // Perceived brightness: 0.299R + 0.587G + 0.114B
            let brightness = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
            if brightness > 128.0 {
                Color::Black
            } else {
                Color::White
            }
        }
        Color::Black => Color::White,
        Color::White => Color::Black,
        _ => Color::White,
    }
}

/// OR a braille dot into the buffer cell, preserving existing dots and background color.
///
/// Half-block characters (`'▀'` U+2580, `'▄'` U+2584) used by filled surfaces and
/// contours are never overwritten — filled faces have higher visual priority than
/// wireframe edges.
pub fn write_braille(buf: &mut Buffer, x: u16, y: u16, bits: u8, color: Color) {
    let (existing_bits, existing_bg) = {
        let cell = &buf[(x, y)];
        let ch = cell.symbol().chars().next().unwrap_or(' ');
        let bg = cell.bg;
        let code = ch as u32;
        // Skip cells with half-block characters (filled faces in 3D / contour rendering)
        if code == 0x2580 || code == 0x2584 {
            return;
        }
        let bits = if (BRAILLE_BASE..=0x28FF).contains(&code) {
            (code - BRAILLE_BASE) as u8
        } else {
            0
        };
        (bits, bg)
    };
    let combined = existing_bits | bits;
    if let Some(ch) = char::from_u32(BRAILLE_BASE + combined as u32) {
        // Ensure braille fg contrasts with existing bg (fill color).
        // If they match, use white or black depending on brightness.
        let fg = if colors_match(color, existing_bg) {
            contrasting_color(color)
        } else {
            color
        };
        buf[(x, y)].set_char(ch).set_fg(fg).set_bg(existing_bg);
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

/// OR a braille dot into the [`PlotBuffer`] at the given cell, using `Z_DATA` priority.
///
/// This is the `PlotBuffer` counterpart of [`write_braille`].
pub fn write_braille_pb(pb: &mut PlotBuffer, x: u16, y: u16, bits: u8, color: Color) {
    pb.set_braille(x, y, bits, color, crate::plot_buffer::Z_DATA);
}

/// Draw a line between two screen-space points using Bresenham's algorithm
/// at Braille sub-pixel resolution (2x4 per cell), writing into a [`PlotBuffer`].
///
/// This is the `PlotBuffer` counterpart of [`draw_braille_line`].
/// Coordinates are in terminal cell space (floating point). The line is clipped
/// to the given plot area bounds.
pub fn draw_braille_line_pb(
    pb: &mut PlotBuffer,
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
                write_braille_pb(pb, cell_x, cell_y, bit, color);
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
