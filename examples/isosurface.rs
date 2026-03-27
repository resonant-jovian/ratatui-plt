//! 3D sphere isosurface rendered with the `Isosurface` widget.
//!
//! Builds a scalar field f(x,y,z) = x^2 + y^2 + z^2 on a 20x20x20
//! grid and extracts the iso-level at 1.0, producing a sphere. Arrow
//! keys rotate the camera, +/- zoom, scroll wheel zooms, and q/Esc
//! quits.

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

    // Build a 20x20x20 scalar field: f(x,y,z) = x^2 + y^2 + z^2
    let n = 20;
    let field: Vec<Vec<Vec<f64>>> = (0..n)
        .map(|iz| {
            let z = iz as f64 / (n - 1) as f64 * 2.0 - 1.0;
            (0..n)
                .map(|iy| {
                    let y = iy as f64 / (n - 1) as f64 * 2.0 - 1.0;
                    (0..n)
                        .map(|ix| {
                            let x = ix as f64 / (n - 1) as f64 * 2.0 - 1.0;
                            x * x + y * y + z * z
                        })
                        .collect()
                })
                .collect()
        })
        .collect();

    let iso = Isosurface::new(field, 1.0)
        .x_range(-1.0, 1.0)
        .y_range(-1.0, 1.0)
        .z_range(-1.0, 1.0)
        .colormap(Plasma)
        .title("Sphere Isosurface (r=1) - Arrow keys: rotate, +/-: zoom, q: quit");

    let mut camera_state = Camera3DState::default();

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            frame.render_stateful_widget(&iso, area, &mut camera_state);
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
