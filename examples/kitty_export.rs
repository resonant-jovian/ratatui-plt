//! Kitty graphics protocol export example.
//!
//! Renders a simple line plot and outputs it as a Kitty inline image.
//!
//! Run with: `cargo run --example kitty_export --features kitty`
//! (requires a Kitty-compatible terminal)

use ratatui_plt::export::{ExportOptions, buffer_to_kitty, render_to_buffer};
use ratatui_plt::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let theme = Theme::get_default();

    let series = Series::new("sin(x)")
        .data(
            (0..100)
                .map(|i| {
                    let x = i as f64 * 0.1;
                    (x, x.sin())
                })
                .collect(),
        )
        .color(theme.primary);

    let plot = LinePlot::new()
        .series(series)
        .title("Kitty Export Demo")
        .x_axis(Axis::new().label("x"))
        .y_axis(
            Axis::new()
                .label("sin(x)")
                .label_position(LabelPosition::End),
        );

    let buf = render_to_buffer(&plot, 80, 24);
    let options = ExportOptions::default();
    let kitty_data = buffer_to_kitty(&buf, &options)?;
    print!("{kitty_data}");
    println!();
    Ok(())
}
