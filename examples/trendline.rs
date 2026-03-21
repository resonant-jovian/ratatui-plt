//! Trendline example: scatter plot with linear and polynomial trendlines.
//!
//! Requires the `statistics` feature: `cargo run --example trendline --features statistics`

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use rand::RngExt;
use ratatui::prelude::*;
use ratatui_plt::prelude::*;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Generate noisy quadratic data
    let mut rng = rand::rng();
    let n = 200;
    let mut points = Vec::with_capacity(n);
    for _ in 0..n {
        let x: f64 = rng.random_range(-3.0..3.0);
        let noise: f64 = rng.random_range(-2.0..2.0);
        let y = 0.5 * x * x - x + 1.0 + noise;
        points.push((x, y));
    }

    let series = Series::new("data")
        .data(points)
        .color(Color::Cyan)
        .marker(MarkerShape::Dot);

    let plot = ScatterPlot::new()
        .series(series)
        .trendline(TrendlineType::Polynomial(2))
        .trendline_color(Color::Yellow)
        .title("Scatter Plot with Polynomial Trendline")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"))
        .show_legend(false);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&plot, frame.area());
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
