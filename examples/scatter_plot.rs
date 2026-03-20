//! Scatter plot example: random point cloud with color-mapped third value.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use rand::RngExt;
use ratatui::prelude::*;
use ratatui_plt::prelude::*;

fn parse_theme() -> Theme {
    match std::env::args().nth(1).as_deref() {
        Some("light") => Theme::light(),
        Some("minimal") => Theme::minimal(),
        Some("publication") => Theme::publication(),
        Some("solarized") => Theme::solarized(),
        Some("dark") | None => Theme::dark(),
        Some(other) => {
            eprintln!(
                "Unknown theme '{other}'. Available: dark, light, minimal, publication, solarized"
            );
            std::process::exit(1);
        }
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Generate random point cloud
    let mut rng = rand::rng();
    let n = 300;
    let mut points = Vec::with_capacity(n);
    let mut color_vals = Vec::with_capacity(n);

    for _ in 0..n {
        let x: f64 = rng.random_range(-5.0..5.0);
        let y: f64 = rng.random_range(-5.0..5.0);
        let z = (-(x * x + y * y) / 8.0).exp(); // radial falloff as color value
        points.push((x, y));
        color_vals.push(z);
    }

    let series = Series::new("cloud")
        .data(points)
        .marker(MarkerShape::FilledCircle);

    let plot = ScatterPlot::new()
        .series(series)
        .color_values(color_vals)
        .colormap(Viridis)
        .title("Random Point Cloud (color = radial intensity)")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"))
        .aspect_ratio(AspectRatio::Equal)
        .show_legend(false);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&plot, square_area(frame.area()));
        })?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && (key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
        {
            break;
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
