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

    // Generate 5000 points from 2 clusters
    let mut rng = rand::rng();
    let mut data = Vec::with_capacity(5000);
    for _ in 0..3000 {
        let x: f64 = rng.random::<f64>() * 4.0 - 1.0;
        let y: f64 = rng.random::<f64>() * 4.0 - 1.0;
        data.push((x, y));
    }
    for _ in 0..2000 {
        let x: f64 = rng.random::<f64>() * 3.0 + 3.0;
        let y: f64 = rng.random::<f64>() * 3.0 + 3.0;
        data.push((x, y));
    }

    let plot = HexbinPlot::new(data)
        .gridsize(20)
        .colormap(Plasma)
        .title("Two-Cluster Hexbin Density (q to quit)")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"));

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
