//! Line plot example: sin(x) and cos(x) with error bands, legend, and grid.

use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use ratatui_sim::prelude::*;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Generate sin(x) and cos(x) data with error bands
    let n = 200;
    let sin_data: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let x = i as f64 * 0.05;
            (x, x.sin())
        })
        .collect();
    let sin_err: Vec<f64> = (0..n).map(|i| 0.1 + 0.05 * (i as f64 * 0.05).abs().sin()).collect();

    let cos_data: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let x = i as f64 * 0.05;
            (x, x.cos())
        })
        .collect();
    let cos_err: Vec<f64> = (0..n).map(|i| 0.08 + 0.04 * (i as f64 * 0.03).cos().abs()).collect();

    let sin_series = Series::new("sin(x)")
        .data(sin_data)
        .color(Color::Cyan)
        .y_err(sin_err);

    let cos_series = Series::new("cos(x)")
        .data(cos_data)
        .color(Color::Yellow)
        .y_err(cos_err);

    let plot = LinePlot::new()
        .series(sin_series)
        .series(cos_series)
        .title("Trigonometric Functions with Error Bands")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&plot, frame.area());
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press
                && (key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
            {
                break;
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
