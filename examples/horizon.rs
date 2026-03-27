//! Horizon graph example.
//!
//! Shows three time series as horizon graphs using sine waves with varying
//! frequency and drift, demonstrating the compact multi-series display.

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
        Some("dark") => Theme::dark(),
        None => Theme::auto(),
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

    let theme = Theme::get_default();
    let mut cycle = theme.color_cycle.clone();

    let n = 300;

    let s1 = Series::new("Temperature")
        .data(
            (0..n)
                .map(|i| {
                    let x = i as f64 * 0.05;
                    (x, (x * 0.7).sin() * 3.0 + (x * 2.3).cos() + x * 0.1)
                })
                .collect(),
        )
        .color(cycle.next_color());

    let s2 = Series::new("Pressure")
        .data(
            (0..n)
                .map(|i| {
                    let x = i as f64 * 0.05;
                    (x, (x * 1.2).sin() * 2.5 + (x * 0.4).cos() * 1.5 - x * 0.05)
                })
                .collect(),
        )
        .color(cycle.next_color());

    let s3 = Series::new("Humidity")
        .data(
            (0..n)
                .map(|i| {
                    let x = i as f64 * 0.05;
                    (x, (x * 0.3).sin() * 4.0 + (x * 1.8).cos() * 0.8 + x * 0.15)
                })
                .collect(),
        )
        .color(cycle.next_color());

    let chart = HorizonGraph::new()
        .series(s1)
        .series(s2)
        .series(s3)
        .n_bands(4)
        .title("Sensor Readings - Horizon Graph (q to quit)");

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&chart, square_area(frame.area()));
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
