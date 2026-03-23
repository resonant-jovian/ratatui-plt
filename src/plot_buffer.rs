//! Z-buffered compositing buffer for plot rendering.
//!
//! `PlotBuffer` separates rendering into Z-ordered layers, ensuring correct
//! visual compositing: fills appear behind grid lines, braille data overlays
//! fills without occluding, and markers render on top of everything.
//!
//! # Z-Level Constants
//!
//! | Level | Purpose |
//! |-------|---------|
//! | [`Z_BACKGROUND`] | Plot area background |
//! | [`Z_FILL`] | Fill regions, reference spans |
//! | [`Z_GRID`] | Grid lines |
//! | [`Z_DATA`] | Lines, braille curves, half-blocks |
//! | [`Z_MARKER`] | Scatter points, line markers |
//! | [`Z_ANNOTATION`] | Text annotations, arrows |
//! | [`Z_CHROME`] | Legend, colorbar, axis labels, spines |

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::drawing::{colors_match, contrasting_color, BRAILLE_BASE};

/// Plot area background.
pub const Z_BACKGROUND: u8 = 0;
/// Fill regions, reference spans (background-color only).
pub const Z_FILL: u8 = 1;
/// Grid lines (foreground characters).
pub const Z_GRID: u8 = 2;
/// Lines, braille curves, half-block data.
pub const Z_DATA: u8 = 3;
/// Scatter points, line markers.
pub const Z_MARKER: u8 = 4;
/// Text annotations, arrows.
pub const Z_ANNOTATION: u8 = 5;
/// Legend, colorbar, axis labels, spines.
pub const Z_CHROME: u8 = 6;

/// Per-cell compositing state tracking background, foreground, and braille
/// contributions at different Z-levels.
#[derive(Clone, Debug, Default)]
struct CellState {
    /// Background color and the Z-level it was set at.
    bg: Option<(Color, u8)>,
    /// Foreground character, its color, and the Z-level.
    fg_char: Option<(char, Color, u8)>,
    /// Accumulated braille dot bits (OR'd together at the same Z-level).
    braille_bits: u8,
    /// Braille foreground color and Z-level.
    braille_fg: Option<(Color, u8)>,
}

/// A Z-buffered compositing buffer for plot rendering.
///
/// Collects rendering operations at different Z-levels, then composites them
/// into a ratatui [`Buffer`] with correct layering.
///
/// # Usage
///
/// ```rust,ignore
/// let mut pb = PlotBuffer::new(area);
/// pb.set_bg(x, y, fill_color, Z_FILL);         // fill background
/// pb.set_char(x, y, '─', grid_color, Z_GRID);  // grid line on top
/// pb.set_braille(x, y, bits, line_color, Z_DATA); // braille data
/// pb.set_char(x, y, '●', marker_color, Z_MARKER); // marker on top
/// pb.composite(buf); // write final result
/// ```
pub struct PlotBuffer {
    area: Rect,
    cells: Vec<CellState>,
}

impl PlotBuffer {
    /// Create a new PlotBuffer covering the given area.
    pub fn new(area: Rect) -> Self {
        let size = area.width as usize * area.height as usize;
        Self {
            area,
            cells: vec![CellState::default(); size],
        }
    }

    /// Flat index for (x, y) within the area. Returns `None` if out of bounds.
    fn index(&self, x: u16, y: u16) -> Option<usize> {
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

    /// Check if (x, y) is within this buffer's area.
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.area.x
            && x < self.area.x + self.area.width
            && y >= self.area.y
            && y < self.area.y + self.area.height
    }

    /// Set background color at the given Z-level.
    ///
    /// Used for fills and reference spans. Does not affect the foreground
    /// character — grid lines and data drawn at higher Z-levels show through.
    pub fn set_bg(&mut self, x: u16, y: u16, color: Color, z: u8) {
        if let Some(i) = self.index(x, y) {
            let cell = &mut self.cells[i];
            if cell.bg.is_none_or(|(_, ez)| z >= ez) {
                cell.bg = Some((color, z));
            }
        }
    }

    /// Set a foreground character at the given Z-level.
    ///
    /// Used for grid lines, markers, text, error bars, etc. Only the
    /// highest-Z character wins. Background is unaffected.
    pub fn set_char(&mut self, x: u16, y: u16, ch: char, fg: Color, z: u8) {
        if let Some(i) = self.index(x, y) {
            let cell = &mut self.cells[i];
            if cell.fg_char.is_none_or(|(_, _, ez)| z >= ez) {
                cell.fg_char = Some((ch, fg, z));
            }
        }
    }

    /// Set braille dots at the given Z-level.
    ///
    /// At the same Z-level, braille bits are OR'd together (multiple line
    /// segments merge). At a higher Z-level, previous bits are replaced.
    pub fn set_braille(&mut self, x: u16, y: u16, bits: u8, fg: Color, z: u8) {
        if let Some(i) = self.index(x, y) {
            let cell = &mut self.cells[i];
            match cell.braille_fg {
                Some((_, ez)) if z > ez => {
                    // Higher Z: replace
                    cell.braille_bits = bits;
                    cell.braille_fg = Some((fg, z));
                }
                Some((_, ez)) if z == ez => {
                    // Same Z: merge via OR
                    cell.braille_bits |= bits;
                    // Keep existing fg color (first writer wins for color)
                }
                None => {
                    cell.braille_bits = bits;
                    cell.braille_fg = Some((fg, z));
                }
                _ => {} // Lower Z: ignore
            }
        }
    }

    /// Set both foreground character and background color at the given Z-level.
    ///
    /// Used for opaque elements like half-block heatmap cells (`'▀'` with
    /// fg=top color, bg=bottom color) and legend entries.
    pub fn set_cell(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Color, z: u8) {
        self.set_bg(x, y, bg, z);
        self.set_char(x, y, ch, fg, z);
    }

    /// Composite all Z-layers into the target ratatui buffer.
    ///
    /// For each cell:
    /// 1. Background: highest-Z `set_bg`/`set_cell` call wins.
    /// 2. Character: if braille bits exist at Z >= fg_char Z, use braille;
    ///    otherwise use fg_char.
    /// 3. Contrast: if fg matches bg, apply contrasting color.
    pub fn composite(&self, buf: &mut Buffer) {
        for y in self.area.y..self.area.y + self.area.height {
            for x in self.area.x..self.area.x + self.area.width {
                let Some(i) = self.index(x, y) else {
                    continue;
                };
                let cell = &self.cells[i];

                // Determine background
                let result_bg = cell.bg.map_or(Color::Reset, |(c, _)| c);

                // Determine character + foreground
                let (result_ch, result_fg) = self.resolve_char(cell, result_bg);

                buf[(x, y)]
                    .set_char(result_ch)
                    .set_fg(result_fg)
                    .set_bg(result_bg);
            }
        }
    }

    /// Resolve which character and foreground color win for a cell.
    fn resolve_char(&self, cell: &CellState, bg: Color) -> (char, Color) {
        let braille_z = cell.braille_fg.map(|(_, z)| z);
        let char_z = cell.fg_char.map(|(_, _, z)| z);

        let (ch, fg) = match (braille_z, char_z) {
            // Both braille and char present
            (Some(bz), Some(cz)) => {
                if cell.braille_bits > 0 && bz >= cz {
                    // Braille wins (higher or equal Z)
                    let braille_ch =
                        char::from_u32(BRAILLE_BASE + cell.braille_bits as u32).unwrap_or(' ');
                    let bfg = cell.braille_fg.map_or(Color::Reset, |(c, _)| c);
                    (braille_ch, bfg)
                } else {
                    // Char wins
                    let (c, f) = cell.fg_char.map_or((' ', Color::Reset), |(c, f, _)| (c, f));
                    (c, f)
                }
            }
            // Only braille
            (Some(_), None) if cell.braille_bits > 0 => {
                let braille_ch =
                    char::from_u32(BRAILLE_BASE + cell.braille_bits as u32).unwrap_or(' ');
                let bfg = cell.braille_fg.map_or(Color::Reset, |(c, _)| c);
                (braille_ch, bfg)
            }
            // Only char
            (_, Some(_)) => {
                let (c, f) = cell.fg_char.map_or((' ', Color::Reset), |(c, f, _)| (c, f));
                (c, f)
            }
            // Nothing
            _ => (' ', Color::Reset),
        };

        // Contrast check — skip for block characters where fg==bg is intentional
        // (full block '█', half blocks '▀'/'▄' encode visual data via fg/bg)
        let is_block = ch == '\u{2588}' || ch == '\u{2580}' || ch == '\u{2584}';
        let fg = if !is_block && colors_match(fg, bg) && ch != ' ' {
            contrasting_color(fg)
        } else {
            fg
        };

        (ch, fg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buf(w: u16, h: u16) -> Buffer {
        Buffer::empty(Rect::new(0, 0, w, h))
    }

    #[test]
    fn test_bg_z_ordering() {
        let area = Rect::new(0, 0, 10, 5);
        let mut pb = PlotBuffer::new(area);
        pb.set_bg(1, 1, Color::Red, Z_FILL);
        pb.set_bg(1, 1, Color::Blue, Z_DATA); // higher Z wins
        let mut buf = make_buf(10, 5);
        pb.composite(&mut buf);
        assert_eq!(buf[(1, 1)].bg, Color::Blue);
    }

    #[test]
    fn test_char_z_ordering() {
        let area = Rect::new(0, 0, 10, 5);
        let mut pb = PlotBuffer::new(area);
        pb.set_char(1, 1, '─', Color::Gray, Z_GRID);
        pb.set_char(1, 1, '●', Color::Cyan, Z_MARKER); // higher Z wins
        let mut buf = make_buf(10, 5);
        pb.composite(&mut buf);
        assert_eq!(buf[(1, 1)].symbol(), "●");
        assert_eq!(buf[(1, 1)].fg, Color::Cyan);
    }

    #[test]
    fn test_braille_or_same_z() {
        let area = Rect::new(0, 0, 10, 5);
        let mut pb = PlotBuffer::new(area);
        pb.set_braille(1, 1, 0x01, Color::Cyan, Z_DATA); // dot 0
        pb.set_braille(1, 1, 0x08, Color::Cyan, Z_DATA); // dot 3 (same Z → OR)
        let mut buf = make_buf(10, 5);
        pb.composite(&mut buf);
        let expected = char::from_u32(BRAILLE_BASE + 0x09).unwrap();
        assert_eq!(buf[(1, 1)].symbol(), expected.to_string());
    }

    #[test]
    fn test_braille_higher_z_replaces() {
        let area = Rect::new(0, 0, 10, 5);
        let mut pb = PlotBuffer::new(area);
        pb.set_braille(1, 1, 0x01, Color::Cyan, Z_DATA);
        pb.set_braille(1, 1, 0x08, Color::Red, Z_MARKER); // higher Z: replaces
        let mut buf = make_buf(10, 5);
        pb.composite(&mut buf);
        let expected = char::from_u32(BRAILLE_BASE + 0x08).unwrap();
        assert_eq!(buf[(1, 1)].symbol(), expected.to_string());
        assert_eq!(buf[(1, 1)].fg, Color::Red);
    }

    #[test]
    fn test_braille_vs_char_priority() {
        let area = Rect::new(0, 0, 10, 5);
        let mut pb = PlotBuffer::new(area);
        pb.set_char(1, 1, '─', Color::Gray, Z_GRID);
        pb.set_braille(1, 1, 0x01, Color::Cyan, Z_DATA); // higher Z → braille wins
        let mut buf = make_buf(10, 5);
        pb.composite(&mut buf);
        let expected = char::from_u32(BRAILLE_BASE + 0x01).unwrap();
        assert_eq!(buf[(1, 1)].symbol(), expected.to_string());
    }

    #[test]
    fn test_fill_does_not_overwrite_grid() {
        // Core bug fix: grid at Z_GRID shows through fill at Z_FILL
        let area = Rect::new(0, 0, 10, 5);
        let mut pb = PlotBuffer::new(area);
        pb.set_bg(1, 1, Color::Rgb(40, 40, 80), Z_FILL);
        pb.set_char(1, 1, '─', Color::Gray, Z_GRID);
        let mut buf = make_buf(10, 5);
        pb.composite(&mut buf);
        assert_eq!(buf[(1, 1)].symbol(), "─");
        assert_eq!(buf[(1, 1)].fg, Color::Gray);
        assert_eq!(buf[(1, 1)].bg, Color::Rgb(40, 40, 80));
    }

    #[test]
    fn test_marker_preserves_fill_bg() {
        let area = Rect::new(0, 0, 10, 5);
        let mut pb = PlotBuffer::new(area);
        pb.set_bg(2, 2, Color::Rgb(40, 40, 80), Z_FILL);
        pb.set_char(2, 2, '●', Color::Cyan, Z_MARKER);
        let mut buf = make_buf(10, 5);
        pb.composite(&mut buf);
        assert_eq!(buf[(2, 2)].symbol(), "●");
        assert_eq!(buf[(2, 2)].fg, Color::Cyan);
        assert_eq!(buf[(2, 2)].bg, Color::Rgb(40, 40, 80));
    }

    #[test]
    fn test_composite_empty() {
        let area = Rect::new(0, 0, 5, 3);
        let pb = PlotBuffer::new(area);
        let mut buf = make_buf(5, 3);
        pb.composite(&mut buf);
        assert_eq!(buf[(0, 0)].symbol(), " ");
        assert_eq!(buf[(0, 0)].bg, Color::Reset);
    }

    #[test]
    fn test_set_cell_opaque() {
        let area = Rect::new(0, 0, 10, 5);
        let mut pb = PlotBuffer::new(area);
        pb.set_cell(3, 3, '▀', Color::Red, Color::Blue, Z_DATA);
        let mut buf = make_buf(10, 5);
        pb.composite(&mut buf);
        assert_eq!(buf[(3, 3)].symbol(), "▀");
        assert_eq!(buf[(3, 3)].fg, Color::Red);
        assert_eq!(buf[(3, 3)].bg, Color::Blue);
    }

    #[test]
    fn test_contrast_adjustment() {
        let area = Rect::new(0, 0, 10, 5);
        let mut pb = PlotBuffer::new(area);
        // White braille on white bg → should get contrasting color
        pb.set_bg(1, 1, Color::White, Z_FILL);
        pb.set_braille(1, 1, 0x01, Color::White, Z_DATA);
        let mut buf = make_buf(10, 5);
        pb.composite(&mut buf);
        assert_eq!(buf[(1, 1)].fg, Color::Black); // contrasted
    }

    #[test]
    fn test_opaque_fill_no_contrast() {
        // Full block with matching fg/bg should NOT be contrasted
        let area = Rect::new(0, 0, 10, 5);
        let mut pb = PlotBuffer::new(area);
        pb.set_cell(1, 1, '\u{2588}', Color::Red, Color::Red, Z_DATA);
        let mut buf = make_buf(10, 5);
        pb.composite(&mut buf);
        assert_eq!(buf[(1, 1)].fg, Color::Red); // NOT contrasted
        assert_eq!(buf[(1, 1)].bg, Color::Red);
    }

    #[test]
    fn test_half_block_no_contrast() {
        // Half block with matching fg/bg should NOT be contrasted
        let area = Rect::new(0, 0, 10, 5);
        let mut pb = PlotBuffer::new(area);
        pb.set_cell(1, 1, '\u{2580}', Color::Blue, Color::Blue, Z_DATA);
        let mut buf = make_buf(10, 5);
        pb.composite(&mut buf);
        assert_eq!(buf[(1, 1)].fg, Color::Blue); // NOT contrasted
    }
}
