//! Output adapters for the plotters rendering pipeline.
//!
//! All widgets render through plotters into a tiny-skia pixmap. This
//! module adapts that pixmap for display on different terminals or
//! export to files.

pub mod file;
#[cfg(feature = "kitty")]
pub mod kitty;
#[cfg(feature = "sixel")]
pub mod sixel;
pub mod unicode;

use plotters::coord::Shift;
use plotters::prelude::{DrawingArea, IntoDrawingArea};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::backend::TinySkiaDrawingBackend;

/// Pixels per terminal cell width.
pub const CELL_PX_W: u32 = 8;
/// Pixels per terminal cell height.
pub const CELL_PX_H: u32 = 16;

/// Rendering output mode for terminal display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputMode {
    /// Downsample pixmap to braille/half-block Unicode characters.
    /// Works in all terminals.
    #[default]
    Unicode,
    /// Transmit pixmap as Kitty APC inline image.
    /// Requires `kitty` feature and a Kitty-capable terminal.
    Kitty,
    /// Quantize and encode pixmap as Sixel escape sequence.
    /// Requires `sixel` feature and a Sixel-capable terminal.
    Sixel,
}

/// Unicode rendering sub-mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnicodeMode {
    /// Half-block characters: 2x vertical resolution, full color.
    /// Best for color-dense plots (heatmaps, filled areas).
    #[default]
    HalfBlock,
    /// Braille characters: 2x4 binary dots per cell.
    /// Best for line plots and scatter plots.
    Braille,
}

/// Render a plotters chart into a ratatui Buffer.
///
/// Creates a tiny-skia pixmap, calls `draw_fn` with a plotters
/// `DrawingArea`, then dispatches the result to the appropriate
/// output adapter based on `mode`.
pub fn render_chart<F>(
    area: Rect,
    buf: &mut Buffer,
    bg: (u8, u8, u8),
    mode: OutputMode,
    unicode_mode: UnicodeMode,
    draw_fn: F,
) where
    F: FnOnce(
        &DrawingArea<TinySkiaDrawingBackend, Shift>,
    ),
{
    if area.width == 0 || area.height == 0 {
        return;
    }

    let px_w = area.width as u32 * CELL_PX_W;
    let px_h = area.height as u32 * CELL_PX_H;

    let Ok((mut backend, pixmap_handle)) =
        TinySkiaDrawingBackend::new(px_w, px_h)
    else {
        return;
    };
    backend.fill_background(bg.0, bg.1, bg.2);

    {
        let root = backend.into_drawing_area();
        draw_fn(&root);
        let _ = plotters::drawing::DrawingArea::present(&root);
    }

    let pixmap = match std::rc::Rc::try_unwrap(pixmap_handle) {
        Ok(cell) => cell.into_inner(),
        Err(_) => return,
    };

    match mode {
        OutputMode::Unicode => {
            unicode::pixmap_to_buf(area, buf, &pixmap, unicode_mode);
        }
        #[cfg(feature = "kitty")]
        OutputMode::Kitty => {
            kitty::pixmap_to_kitty(area, buf, &pixmap);
        }
        #[cfg(not(feature = "kitty"))]
        OutputMode::Kitty => {
            unicode::pixmap_to_buf(area, buf, &pixmap, unicode_mode);
        }
        #[cfg(feature = "sixel")]
        OutputMode::Sixel => {
            sixel::pixmap_to_sixel(area, buf, &pixmap);
        }
        #[cfg(not(feature = "sixel"))]
        OutputMode::Sixel => {
            unicode::pixmap_to_buf(area, buf, &pixmap, unicode_mode);
        }
    }
}
