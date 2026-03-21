//! Contour plot example: filled 2D potential field.
//!
//! Demonstrates filled contour rendering with a high-resolution grid
//! and the Viridis colormap.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
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

    // Two-peak Gaussian potential field
    let data = GridData::from_fn((-4.0, 4.0), (-4.0, 4.0), 360, 360, |x, y| {
        let peak1 = (-((x - 1.5).powi(2) + (y - 1.0).powi(2)) / 1.5).exp();
        let peak2 = 0.8 * (-((x + 1.0).powi(2) + (y + 1.5).powi(2)) / 2.0).exp();
        peak1 + peak2
    });

    let plot = ContourPlot::new(data)
        .levels(12)
        .filled(true)
        .colormap(Viridis)
        .title("2D Potential Field (two Gaussian peaks) — q to quit")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .aspect_ratio(AspectRatio::Equal);

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
