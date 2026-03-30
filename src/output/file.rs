//! File export via plotters' native SVG and bitmap backends.
//!
//! This path bypasses the tiny-skia pixmap entirely, using plotters'
//! own `SVGBackend` or `BitMapBackend` for maximum quality output.

use std::path::Path;

use plotters::prelude::*;

use crate::backend::TinySkiaError;

/// Save a chart as an SVG file using plotters' native SVG backend.
///
/// The `draw_fn` receives a plotters `DrawingArea` and should draw
/// the chart using standard plotters API.
///
/// # Errors
///
/// Returns an error if the file cannot be created or written.
pub fn save_svg<P, F>(
    path: P,
    width: u32,
    height: u32,
    draw_fn: F,
) -> Result<(), TinySkiaError>
where
    P: AsRef<Path>,
    F: FnOnce(&DrawingArea<SVGBackend, plotters::coord::Shift>),
{
    let root = SVGBackend::new(path.as_ref(), (width, height))
        .into_drawing_area();
    root.fill(&WHITE)
        .map_err(|e| TinySkiaError::Creation(format!("{e}")))?;

    draw_fn(&root);

    root.present()
        .map_err(|e| TinySkiaError::Creation(format!("{e}")))?;

    Ok(())
}

/// Save a chart as a PNG file using the tiny-skia pixmap pipeline.
///
/// Renders the chart through the same TinySkiaDrawingBackend used for
/// terminal display, then encodes the pixmap as PNG.
///
/// # Errors
///
/// Returns an error if the pixmap cannot be created or PNG encoding fails.
pub fn save_png<P, F>(
    path: P,
    width: u32,
    height: u32,
    bg: (u8, u8, u8),
    draw_fn: F,
) -> Result<(), TinySkiaError>
where
    P: AsRef<Path>,
    F: FnOnce(
        &DrawingArea<
            crate::backend::TinySkiaDrawingBackend,
            plotters::coord::Shift,
        >,
    ),
{
    let (mut backend, pixmap_handle) =
        crate::backend::TinySkiaDrawingBackend::new(width, height)?;
    backend.fill_background(bg.0, bg.1, bg.2);

    {
        let root = backend.into_drawing_area();
        draw_fn(&root);
        let _ = plotters::drawing::DrawingArea::present(&root);
    }

    let pixmap = match std::rc::Rc::try_unwrap(pixmap_handle) {
        Ok(cell) => cell.into_inner(),
        Err(_) => {
            return Err(TinySkiaError::Creation(
                "failed to extract pixmap".to_string(),
            ))
        }
    };

    pixmap
        .save_png(path.as_ref())
        .map_err(|e| TinySkiaError::Creation(format!("{e}")))?;

    Ok(())
}
