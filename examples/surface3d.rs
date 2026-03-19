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

    // Generate sin(sqrt(x^2 + y^2)) surface data
    let data = GridData::from_fn((-6.0, 6.0), (-6.0, 6.0), 40, 40, |x, y| {
        let r = (x * x + y * y).sqrt();
        r.sin()
    });

    let surface = Surface3D::new(data)
        .colormap(Plasma)
        .title("sin(sqrt(x^2 + y^2)) - Arrow keys: rotate, +/-: zoom, q: quit");

    let mut camera_state = Camera3DState::default();

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_stateful_widget(&surface, area, &mut camera_state);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Left => camera_state.rotate(-5.0, 0.0),
                    KeyCode::Right => camera_state.rotate(5.0, 0.0),
                    KeyCode::Up => camera_state.rotate(0.0, 5.0),
                    KeyCode::Down => camera_state.rotate(0.0, -5.0),
                    KeyCode::Char('+') | KeyCode::Char('=') => camera_state.zoom(1.1),
                    KeyCode::Char('-') => camera_state.zoom(0.9),
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
