//! Stem plot example: discrete event sequence with 15 events at varying positions and heights.

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

    // 15 discrete events at pseudo-random positions with varying heights.
    // Uses a simple deterministic pattern so no extra RNG crate is needed.
    let events: Vec<(f64, f64)> = (0..150)
        .map(|i| {
            let x = i as f64 * 1.3 + 0.5;
            // Heights from a mix of sin + sawtooth for variety
            let y = 2.0 * (i as f64 * 0.7).sin() + 1.5 * ((i as f64 * 0.4).cos()).abs() + 0.5;
            (x, y)
        })
        .collect();

    let theme = Theme::get_default();
    let plot = StemPlot::new(events)
        .baseline(0.0)
        .color(theme.primary)
        .marker(MarkerShape::FilledCircle)
        .title("Discrete Event Sequence (150 events)")
        .x_axis(Axis::new().label("time"))
        .y_axis(
            Axis::new()
                .label("amplitude")
                .label_position(LabelPosition::End),
        );

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
