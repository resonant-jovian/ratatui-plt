//! TOML theme loading example.
//!
//! Demonstrates loading a theme from a TOML string and applying it to a plot.
//!
//! Run with: `cargo run --example toml_theme --features toml-themes`

use ratatui::prelude::*;
use ratatui_plt::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = r##"
[colors]
background = "#1a1a2e"
foreground = "#e0e0e0"
grid = "#2a2a4a"
minor_grid = "#1e1e3e"
axis = "#8888aa"

[grid]
visible = true
pattern = "dashed"
bold_title = true

[cycle]
colors = ["#e94560", "#0f3460", "#16c79a", "#f5a623", "#b721ff"]
"##;

    let theme = theme_from_toml(toml_str)?;
    println!("Loaded theme: {theme:?}");
    println!();

    // Apply the theme to a plot.
    let _guard = theme.activate();
    let theme = Theme::get_default();

    let series = Series::new("data")
        .data(
            (0..50)
                .map(|i| {
                    let x = i as f64 * 0.1;
                    (x, x.sin() * x.cos())
                })
                .collect(),
        )
        .color(theme.primary);

    let plot = LinePlot::new()
        .series(series)
        .title("TOML Theme Demo")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("f(x)").label_position(LabelPosition::End));

    if headless_export(|area, buf| (&plot).render(area, buf))? {
        return Ok(());
    }

    let buf = render_to_buffer(&plot, 60, 20);
    let text = buffer_to_text(&buf);
    println!("{text}");

    Ok(())
}
