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

    // Generate 5000 correlated points
    let mut rng = rand::rng();
    let data: Vec<(f64, f64)> = (0..50000)
        .map(|_| {
            let x: f64 = rng.random::<f64>() * 6.0 - 3.0;
            let y: f64 = x * 0.8 + rng.random::<f64>() * 1.2 - 0.6;
            (x, y)
        })
        .collect();

    let plot = Hist2D::new(data)
        .bins_x(40)
        .bins_y(40)
        .colormap(Inferno)
        .show_colorbar(true)
        .title("Correlated 2D Histogram (q to quit)")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y = 0.7x + noise"));

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
