//! Rug plot example: Earthquake magnitudes.
//!
//! A dataset of ~40 earthquake magnitudes shown as rug ticks along the bottom axis.
//! All magnitudes are deterministic constants (no random generation needed).

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

    // Deterministic earthquake magnitude dataset (~40 values in Richter scale range 2-8)
    // Inspired by realistic magnitude-frequency distribution: many small, few large
    let magnitudes = vec![
        2.1, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 3.0, 3.1, 3.2, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 4.0,
        4.1, 4.2, 4.3, 4.5, 4.7, 5.0, 5.1, 5.3, 5.5, 5.8, 6.0, 6.2, 6.5, 6.7, 7.0, 7.1, 7.3, 7.8,
        8.0, 8.1, 2.2, 3.8,
    ];

    let ds = RugDataset::new("Earthquakes", magnitudes, Color::Red);

    let plot = RugPlot::new()
        .dataset(ds)
        .side(RugSide::Bottom)
        .height(3)
        .title("Rug Plot: Earthquake Magnitudes (q to quit)")
        .x_axis(Axis::new().label("Magnitude (Richter)").grid(true))
        .y_axis(Axis::new());

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
