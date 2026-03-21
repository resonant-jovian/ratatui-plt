//! Export example: save a line plot as PNG and PDF.
//!
//! Demonstrates the raster export feature by rendering a sine wave
//! to both PNG and PDF files.
//!
//! Run with:
//! ```sh
//! cargo run --example export_png_pdf --features export
//! ```

use ratatui_plt::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build a simple sine-wave series.
    let data: Vec<(f64, f64)> = (0..200)
        .map(|i| {
            let x = i as f64 * 0.05;
            (x, x.sin())
        })
        .collect();

    let series = Series::new("sin(x)")
        .data(data)
        .color(Color::Cyan);

    let plot = LinePlot::new()
        .series(series)
        .title("Sine Wave Export")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"));

    let options = ExportOptions::new()
        .font_size(14.0)
        .dpi(150.0)
        .background(30, 30, 30);

    let width: u16 = 120;
    let height: u16 = 40;

    // Save as PNG.
    save_png(&plot, width, height, "export_demo.png", &options)?;
    let png_meta = std::fs::metadata("export_demo.png")?;
    println!("Saved export_demo.png ({} bytes)", png_meta.len());

    // Save as PDF.
    save_pdf(&plot, width, height, "export_demo.pdf", &options)?;
    let pdf_meta = std::fs::metadata("export_demo.pdf")?;
    println!("Saved export_demo.pdf ({} bytes)", pdf_meta.len());

    Ok(())
}
