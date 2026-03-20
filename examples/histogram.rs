//! Histogram example: pseudo-normal distribution (sum of uniform randoms) with Cyan bars.

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

    // Generate ~500 samples from a pseudo-normal distribution:
    // sum of 12 uniform [0,1) randoms minus 6 gives approx N(0,1)
    let mut rng = rand::rng();
    let samples: Vec<f64> = (0..500)
        .map(|_| {
            let sum: f64 = (0..12).map(|_| rng.random::<f64>()).sum();
            sum - 6.0
        })
        .collect();

    let hist = Histogram::new(samples)
        .bins(30)
        .color(Color::Cyan)
        .title("Pseudo-Normal Distribution (n=500, sum of 12 uniforms)")
        .x_axis(Axis::new().label("value"))
        .y_axis(Axis::new().label("count"));

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&hist, square_area(frame.area()));
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
