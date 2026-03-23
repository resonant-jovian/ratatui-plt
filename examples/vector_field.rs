//! Vector field (quiver) plot example: circular flow.
//!
//! Displays a 2D vector field with arrows colored by magnitude.
//! Uses a 10x10 grid with increased arrow scale for visibility.

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

    // Circular flow field: (-y, x)
    // Use a 10x10 grid so arrows are well-spaced in typical terminal sizes.
    let field = VectorFieldData::from_fn((-3.0, 3.0), (-3.0, 3.0), 10, 10, |x, y| (-y, x));

    let plot = VectorField::new(field)
        .title("Circular Flow Field (-y, x) - q to quit")
        .color_by_magnitude(true)
        .colormap(Viridis)
        .arrow_scale(3.0)
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .aspect_ratio(AspectRatio::Equal);

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
