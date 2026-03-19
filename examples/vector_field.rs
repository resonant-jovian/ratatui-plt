use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use ratatui_sim::prelude::*;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Circular flow field: (-y, x)
    let field = VectorFieldData::from_fn(
        (-3.0, 3.0),
        (-3.0, 3.0),
        15,
        15,
        |x, y| (-y, x),
    );

    let plot = VectorField::new(field)
        .title("Circular Flow Field (-y, x) - q to quit")
        .color_by_magnitude(true)
        .colormap(Viridis)
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"));

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(&plot, area);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press
                && (key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
            {
                break;
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
