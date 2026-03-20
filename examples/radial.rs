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

    // Cardioid: r = 1 + cos(theta)
    let cardioid: Vec<(f64, f64)> = (0..=360)
        .map(|i| {
            let theta = i as f64 * std::f64::consts::PI / 180.0;
            (theta, 1.0 + theta.cos())
        })
        .collect();

    // Rose curve: r = cos(2*theta)
    let rose: Vec<(f64, f64)> = (0..=360)
        .map(|i| {
            let theta = i as f64 * std::f64::consts::PI / 180.0;
            (theta, (2.0 * theta).cos().abs())
        })
        .collect();

    let plot = RadialPlot::new()
        .series(
            Series::new("Cardioid: r=1+cos(\u{03b8})")
                .data(cardioid)
                .color(Color::Cyan),
        )
        .series(
            Series::new("Rose: r=|cos(2\u{03b8})|")
                .data(rose)
                .color(Color::Yellow),
        )
        .title("Polar Coordinate Plot (q to quit)")
        .n_rings(4)
        .n_spokes(8)
        .r_max(2.5);

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(&plot, area);
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
