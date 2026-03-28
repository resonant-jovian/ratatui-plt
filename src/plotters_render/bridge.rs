//! Bridge between plotters rendering and ratatui Buffer output.
//!
//! Handles pixmap creation, Kitty protocol encoding, and writing
//! the result into a ratatui Buffer for display.

use std::io::IsTerminal;

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

    // Choose output mode: Kitty APC for real terminals, cell-color
    // sampling for headless export (SVG/PNG capture).
    if std::io::stdout().is_terminal()
        && std::env::var("RATATUI_PLT_EXPORT").is_err()
    {
        // Live terminal: encode as Kitty APC escape sequence.
        let Ok(png_bytes) = pixmap.encode_png() else {
            return;
        };
        let b64 = base64_encode(&png_bytes);
        let kitty_escape = build_kitty_escape(&b64, px_w, px_h);
        write_kitty_to_buf(area, buf, &kitty_escape);
    } else {
        // Headless: sample pixel colors into buffer cells so
        // buffer_to_png / buffer_to_svg can capture them.
        write_pixmap_to_cells(area, buf, &pixmap, CELL_PX_W, CELL_PX_H);
    }
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

/// Sample pixmap pixel colors into ratatui Buffer cells.
///
/// Each cell covers `cell_w x cell_h` pixels. We use the top-left
/// pixel of each cell for the cell's background color, and the
/// half-block character `▀` to encode two vertical rows per cell
/// (top half = fg, bottom half = bg) for 2x vertical resolution.
fn write_pixmap_to_cells(
    area: Rect,
    buf: &mut Buffer,
    pixmap: &tiny_skia::Pixmap,
    cell_w: u32,
    cell_h: u32,
) {
    use ratatui::style::Color;

    let img_w = pixmap.width();
    let img_h = pixmap.height();
    let pixels = pixmap.data();

    // Average pixels in a region to get the cell color.
    #[allow(clippy::manual_checked_ops)]
    let avg_region =
        |x0: u32, y0: u32, w: u32, h: u32| -> (u8, u8, u8) {
            let mut r_sum: u64 = 0;
            let mut g_sum: u64 = 0;
            let mut b_sum: u64 = 0;
            let mut count: u64 = 0;
            for py in y0..y0 + h {
                for px in x0..x0 + w {
                    if px < img_w && py < img_h {
                        let idx =
                            (py * img_w + px) as usize * 4;
                        if idx + 3 < pixels.len() {
                            let a = pixels[idx + 3] as u64;
                            if a > 0 {
                                r_sum += pixels[idx] as u64
                                    * 255
                                    / a;
                                g_sum += pixels[idx + 1] as u64
                                    * 255
                                    / a;
                                b_sum += pixels[idx + 2] as u64
                                    * 255
                                    / a;
                            }
                            count += 1;
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
        };

    let half_h = cell_h / 2;

    for row in 0..area.height {
        for col in 0..area.width {
            let px_x = col as u32 * cell_w;
            let px_y = row as u32 * cell_h;

            // Average the top half and bottom half of the cell
            // separately for half-block rendering.
            let (tr, tg, tb) =
                avg_region(px_x, px_y, cell_w, half_h);
            let (br, bg_r, bb) =
                avg_region(px_x, px_y + half_h, cell_w, half_h);

            if let Some(cell) =
                buf.cell_mut((area.x + col, area.y + row))
            {
                cell.set_symbol("▀");
                cell.set_fg(Color::Rgb(tr, tg, tb));
                cell.set_bg(Color::Rgb(br, bg_r, bb));
            }
        }
    }
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
