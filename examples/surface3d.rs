use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind, MouseEventKind},
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
    io::stdout().execute(crossterm::event::EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Generate sin(sqrt(x^2 + y^2)) surface data
    let data = GridData::from_fn((-6.0, 6.0), (-6.0, 6.0), 120, 120, |x, y| {
        let r = (x * x + y * y).sqrt();
        r.sin()
    });

    let surface = Surface3D::new(data)
        .colormap(Plasma)
        .show_wireframe(false)
        .title("sin(sqrt(x^2 + y^2)) - Arrow keys: rotate, +/-: zoom, q: quit");

    let mut camera_state = Camera3DState::default();

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            frame.render_stateful_widget(&surface, area, &mut camera_state);
        })?;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Left => camera_state.rotate(-5.0, 0.0),
                KeyCode::Right => camera_state.rotate(5.0, 0.0),
                KeyCode::Up => camera_state.rotate(0.0, 5.0),
                KeyCode::Down => camera_state.rotate(0.0, -5.0),
                KeyCode::Char('+') | KeyCode::Char('=') => camera_state.zoom(1.2),
                KeyCode::Char('-') => camera_state.zoom(0.8),
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => camera_state.zoom(1.2),
                MouseEventKind::ScrollDown => camera_state.zoom(0.8),
                _ => {}
            },
            _ => {}
        }
    }

    io::stdout().execute(crossterm::event::DisableMouseCapture)?;
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
