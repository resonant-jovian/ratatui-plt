//! Kitty graphics protocol output adapter.
//!
//! Encodes a tiny-skia pixmap as PNG, base64-encodes it, and writes
//! it to a ratatui Buffer as a Kitty APC escape sequence.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tiny_skia::Pixmap;

/// Write a pixmap to a ratatui Buffer via Kitty graphics protocol.
pub fn pixmap_to_kitty(area: Rect, buf: &mut Buffer, pixmap: &Pixmap) {
    let px_w = pixmap.width();
    let px_h = pixmap.height();

    let Ok(png_bytes) = pixmap.encode_png() else {
        return;
    };
    let b64 = base64_encode(&png_bytes);
    let escape = build_kitty_escape(&b64, px_w, px_h);
    write_kitty_to_buf(area, buf, &escape);
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
fn write_kitty_to_buf(area: Rect, buf: &mut Buffer, escape: &str) {
    if let Some(cell) = buf.cell_mut((area.x, area.y)) {
        cell.set_symbol(escape);
    }

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
