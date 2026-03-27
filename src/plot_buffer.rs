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
//! | [`Z_BACKGROUND`] (0) | Plot area background |
//! | [`Z_FILL`] (16) | Fill regions, reference spans |
//! | [`Z_GRID`] (32) | Grid lines |
//! | [`Z_DATA`] (48) | Lines, braille curves, half-blocks (series use Z_DATA, Z_DATA+1, ...) |
//! | [`Z_MARKER`] (128) | Scatter points, line markers |
//! | [`Z_ANNOTATION`] (160) | Text annotations, arrows |
//! | [`Z_CHROME`] (192) | Legend, colorbar, axis labels, spines |

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::drawing::{BRAILLE_BASE, BRAILLE_BITS, colors_match, contrasting_color};
use crate::frame::PlotArea;

/// Trait defining the rendering backend abstraction.
///
/// All plot widgets render through this interface. The default implementation
/// is [`PlotBuffer`] (Braille/half-block Unicode rendering). Alternative
/// backends can implement this trait to provide pixel-level rendering via
/// Kitty graphics protocol, Sixel, or other terminal image protocols.
///
/// # Backend Implementations
///
/// - [`PlotBuffer`] — Default. Uses Braille sub-pixel dots for lines and
///   half-block characters for fills. Works in all terminals.
/// - *(Future)* `KittyBackend` — Renders to a pixel buffer via tiny-skia,
///   outputs via Kitty Unicode placeholders for pixel-perfect plots.
/// - *(Future)* `SixelBackend` — Renders to pixels, encodes as Sixel protocol.
pub trait PlotBackend {
    /// Set the background color at cell (x, y) at the given Z-level.
    fn set_bg(&mut self, x: u16, y: u16, color: Color, z: u8);

    /// Set a foreground character at cell (x, y) at the given Z-level.
    fn set_char(&mut self, x: u16, y: u16, ch: char, fg: Color, z: u8);

    /// Set both foreground character and background color at the given Z-level.
    fn set_cell(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Color, z: u8);

    /// Draw a line from (x0, y0) to (x1, y1), clipped to the plot area.
    ///
    /// Coordinates are in terminal cell space (floating point).
    /// The backend decides the rendering technique:
    /// - Braille sub-pixel dots for Unicode backends
    /// - Anti-aliased pixels for graphics protocol backends
    #[allow(clippy::too_many_arguments)]
    fn draw_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, color: Color, pa: &PlotArea, z: u8);

    /// Set braille dots at cell (x, y) at the given Z-level.
    ///
    /// For Unicode backends, braille bits are OR'd together at the same Z-level.
    /// For pixel backends, this translates to setting small sub-cell regions.
    fn set_braille(&mut self, x: u16, y: u16, bits: u8, fg: Color, z: u8);

    /// Check if cell (x, y) is within this backend's renderable area.
    fn contains(&self, x: u16, y: u16) -> bool;

    /// Get the renderable area covered by this backend.
    fn area(&self) -> Rect;

    /// Composite the backend's internal state into a ratatui Buffer.
    fn composite(&self, buf: &mut Buffer);
}

/// Blanket implementation so `Box<dyn PlotBackend>` can be passed where
/// `&mut dyn PlotBackend` is expected.
impl PlotBackend for Box<dyn PlotBackend> {
    fn set_bg(&mut self, x: u16, y: u16, color: Color, z: u8) {
        (**self).set_bg(x, y, color, z);
    }
    fn set_char(&mut self, x: u16, y: u16, ch: char, fg: Color, z: u8) {
        (**self).set_char(x, y, ch, fg, z);
    }
    fn set_cell(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Color, z: u8) {
        (**self).set_cell(x, y, ch, fg, bg, z);
    }
    fn set_braille(&mut self, x: u16, y: u16, bits: u8, fg: Color, z: u8) {
        (**self).set_braille(x, y, bits, fg, z);
    }
    #[allow(clippy::too_many_arguments)]
    fn draw_line(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        color: Color,
        pa: &PlotArea,
        z: u8,
    ) {
        (**self).draw_line(x0, y0, x1, y1, color, pa, z);
    }
    fn contains(&self, x: u16, y: u16) -> bool {
        (**self).contains(x, y)
    }
    fn area(&self) -> Rect {
        (**self).area()
    }
    fn composite(&self, buf: &mut Buffer) {
        (**self).composite(buf);
    }
}

/// Create a rendering backend for the given area, using the
/// configured backend from
/// [`PlotConfig`](crate::config::PlotConfig).
///
/// With the default [`RenderBackend::Auto`], this auto-detects
/// the best available graphics protocol (Kitty > Sixel >
/// Unicode). Detection results are cached per-process.
pub fn create_backend(area: Rect) -> Box<dyn PlotBackend> {
    use crate::config::{PlotConfig, RenderBackend, detect_backend};

    let cfg = PlotConfig::get_default();

    // Resolve Auto to a concrete backend via cached detection
    let backend = match cfg.render_backend {
        RenderBackend::Auto => detect_backend(),
        other => other,
    };

    match backend {
        RenderBackend::Unicode | RenderBackend::Auto => {
            Box::new(PlotBuffer::new(area))
        }
        #[cfg(feature = "kitty")]
        RenderBackend::Kitty => {
            Box::new(
                crate::kitty_backend::KittyBackend::new(area),
            )
        }
        #[cfg(feature = "sixel")]
        RenderBackend::Sixel => {
            Box::new(
                crate::sixel_backend::SixelBackend::new(area),
            )
        }
        // Feature not enabled — fall back to Unicode
        #[cfg(not(feature = "kitty"))]
        RenderBackend::Kitty => {
            Box::new(PlotBuffer::new(area))
        }
        #[cfg(not(feature = "sixel"))]
        RenderBackend::Sixel => {
            Box::new(PlotBuffer::new(area))
        }
    }
}

/// Plot area background.
pub const Z_BACKGROUND: u8 = 0;
/// Fill regions, reference spans (background-color only).
pub const Z_FILL: u8 = 16;
/// Grid lines (foreground characters).
pub const Z_GRID: u8 = 32;
/// Lines, braille curves, half-block data.
/// Multi-series widgets use Z_DATA, Z_DATA+1, Z_DATA+2, ... so that
/// higher-indexed series replace lower-indexed ones in shared cells.
pub const Z_DATA: u8 = 48;
/// Scatter points, line markers.
pub const Z_MARKER: u8 = 128;
/// Text annotations, arrows.
pub const Z_ANNOTATION: u8 = 160;
/// Legend, colorbar, axis labels, spines.
pub const Z_CHROME: u8 = 192;

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
            Some((y - self.area.y) as usize * self.area.width as usize + (x - self.area.x) as usize)
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

    /// Draw a line from `(x0, y0)` to `(x1, y1)` using Braille sub-pixel dots.
    #[allow(clippy::too_many_arguments)]
    ///
    /// Coordinates are in terminal cell space (floating point). The line is
    /// clipped to the given plot area bounds. Uses Bresenham's algorithm at
    /// 2× horizontal and 4× vertical resolution for sub-cell precision.
    ///
    /// This is the primary line-drawing method for the PlotBackend abstraction.
    /// Alternative backends (e.g., Kitty/Sixel) would implement this differently
    /// to produce pixel-level anti-aliased lines.
    pub fn draw_line(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        color: Color,
        pa: &PlotArea,
        z: u8,
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
                    self.set_braille(cell_x, cell_y, bit, color, z);
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

/// PlotBuffer implements the PlotBackend trait, delegating to its inherent methods.
///
/// This is the default (Braille/Unicode) backend used by all widgets.
impl PlotBackend for PlotBuffer {
    fn set_bg(&mut self, x: u16, y: u16, color: Color, z: u8) {
        PlotBuffer::set_bg(self, x, y, color, z);
    }

    fn set_char(&mut self, x: u16, y: u16, ch: char, fg: Color, z: u8) {
        PlotBuffer::set_char(self, x, y, ch, fg, z);
    }

    fn set_cell(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Color, z: u8) {
        PlotBuffer::set_cell(self, x, y, ch, fg, bg, z);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_line(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        color: Color,
        pa: &PlotArea,
        z: u8,
    ) {
        PlotBuffer::draw_line(self, x0, y0, x1, y1, color, pa, z);
    }

    fn set_braille(&mut self, x: u16, y: u16, bits: u8, fg: Color, z: u8) {
        PlotBuffer::set_braille(self, x, y, bits, fg, z);
    }

    fn contains(&self, x: u16, y: u16) -> bool {
        PlotBuffer::contains(self, x, y)
    }

    fn area(&self) -> Rect {
        self.area
    }

    fn composite(&self, buf: &mut Buffer) {
        PlotBuffer::composite(self, buf);
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
