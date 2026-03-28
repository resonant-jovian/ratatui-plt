//! Bridge between plotters rendering and ratatui Buffer output.
//!
//! Handles pixmap creation, Kitty protocol encoding, and writing
//! the result into a ratatui Buffer for display.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::backend::TinySkiaDrawingBackend;

/// Default cell width in pixels (matches existing Kitty/Sixel backends).
pub const CELL_PX_W: u32 = 8;
/// Default cell height in pixels.
pub const CELL_PX_H: u32 = 16;

/// Render a chart via plotters into a ratatui Buffer using Kitty
/// graphics protocol.
///
/// The `draw_fn` closure receives a plotters `DrawingArea` backed by
/// tiny-skia. After rendering, the pixmap is PNG-encoded and written
/// to `buf` as a Kitty APC escape sequence.
///
/// # Arguments
///
/// * `area` — Terminal cell rectangle to render into
/// * `buf` — ratatui Buffer to write the result into
/// * `bg` — Background color (r, g, b)
/// * `draw_fn` — Closure that draws the chart using plotters API
pub fn render_plotters_to_buf<F>(
    area: Rect,
    buf: &mut Buffer,
    bg: (u8, u8, u8),
    draw_fn: F,
) where
    F: FnOnce(
        &plotters::prelude::DrawingArea<
            TinySkiaDrawingBackend,
            plotters::coord::Shift,
        >,
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

    // DrawingArea consumes the backend. We keep a shared handle to
    // the pixmap so we can extract it after rendering.
    {
        let root =
            plotters::prelude::IntoDrawingArea::into_drawing_area(
                backend,
            );

        draw_fn(&root);

        let _ = plotters::drawing::DrawingArea::present(&root);
    }

    // Extract pixmap from the shared handle (backend is now dropped).
    let pixmap = match std::rc::Rc::try_unwrap(pixmap_handle) {
        Ok(cell) => cell.into_inner(),
        Err(_) => return,
    };

    // Encode pixmap as PNG.
    let Ok(png_bytes) = pixmap.encode_png() else {
        return;
    };

    // Base64-encode and build Kitty APC escape sequence.
    let b64 = base64_encode(&png_bytes);
    let kitty_escape = build_kitty_escape(&b64, px_w, px_h);

    // Write escape to first cell, mark rest as skip.
    write_kitty_to_buf(area, buf, &kitty_escape);
}

/// Base64-encode a byte slice.
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\
          0123456789+/";
    let mut result =
        String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result
            .push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        result
            .push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(
                TABLE[((triple >> 6) & 0x3F) as usize] as char,
            );
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result
                .push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Build a Kitty graphics APC escape sequence from base64 PNG data.
fn build_kitty_escape(b64: &str, width: u32, height: u32) -> String {
    use std::fmt::Write;
    let chunk_size = 4096;
    let mut output = String::new();

    if b64.len() <= chunk_size {
        let _ = write!(
            output,
            "\x1b_Gf=100,a=T,t=d,s={width},v={height},m=0;{b64}\x1b\\"
        );
    } else {
        let chunks: Vec<&str> = {
            let mut v = Vec::new();
            let mut start = 0;
            while start < b64.len() {
                let end = (start + chunk_size).min(b64.len());
                v.push(&b64[start..end]);
                start = end;
            }
            v
        };
        let last_idx = chunks.len() - 1;
        for (i, chunk) in chunks.iter().enumerate() {
            if i == 0 {
                let _ = write!(
                    output,
                    "\x1b_Gf=100,a=T,t=d,s={width},v={height},m=1;\
                     {chunk}\x1b\\"
                );
            } else if i == last_idx {
                let _ = write!(
                    output,
                    "\x1b_Gm=0;{chunk}\x1b\\"
                );
            } else {
                let _ = write!(
                    output,
                    "\x1b_Gm=1;{chunk}\x1b\\"
                );
            }
        }
    }
    output
}

/// Write a Kitty escape sequence into a ratatui Buffer.
///
/// The escape is stored in the first cell's symbol. All other cells
/// in the area are marked as `skip` so the terminal renders only
/// the image.
fn write_kitty_to_buf(area: Rect, buf: &mut Buffer, escape: &str) {
    // Store full escape in the first cell.
    if let Some(cell) = buf.cell_mut((area.x, area.y)) {
        cell.set_symbol(escape);
    }

    // Mark all other cells as skip.
    for row in area.y..area.y + area.height {
        for col in area.x..area.x + area.width {
            if row == area.y && col == area.x {
                continue;
            }
            if let Some(cell) = buf.cell_mut((col, row)) {
                cell.set_skip(true);
            }
        }
    }
}
