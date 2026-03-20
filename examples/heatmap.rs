//! Heatmap example: 2D Gaussian with Viridis colormap, colorbar, and equal aspect ratio.

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

    // 2D Gaussian: z = exp(-(x^2 + y^2))
    let data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 500, 500, |x, y| {
        (-(x * x + y * y)).exp()
    });

    let heatmap = Heatmap::new(data)
        .colormap(Viridis)
        .title("2D Gaussian  z = exp(-(x^2 + y^2))")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"))
        .show_colorbar(true)
        .aspect_ratio(AspectRatio::Equal);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&heatmap, frame.area());
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
