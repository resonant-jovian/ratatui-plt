//! Sixel graphics protocol export example.
//!
//! Renders a simple line plot and outputs it as a Sixel image.
//!
//! Run with: `cargo run --example sixel_export --features sixel`
//! (requires a Sixel-compatible terminal)

use ratatui::prelude::*;
use ratatui_plt::export::{ExportOptions, buffer_to_sixel, render_to_buffer};
use ratatui_plt::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let theme = Theme::get_default();
    let series = Series::new("cos(x)")
        .data(
            (0..100)
                .map(|i| {
                    let x = i as f64 * 0.1;
                    (x, x.cos())
                })
                .collect(),
        )
        .color(theme.accent);

    let plot = LinePlot::new()
        .series(series)
        .title("Sixel Export Demo")
        .x_axis(Axis::new().label("x"))
        .y_axis(
            Axis::new()
                .label("cos(x)")
                .label_position(LabelPosition::End),
        );

    if headless_export(|area, buf| (&plot).render(area, buf))? {
        return Ok(());
    }

    let buf = render_to_buffer(&plot, 80, 24);
    let options = ExportOptions::default();
    let sixel_data = buffer_to_sixel(&buf, &options)?;
    print!("{sixel_data}");
    println!();
    Ok(())
}
