//! 3D icosahedron rendered with the `Mesh3D` widget.
//!
//! Constructs the 12 vertices and 20 triangular faces of a regular
//! icosahedron and displays them with wireframe overlay. Arrow keys
//! rotate the camera, +/- zoom, scroll wheel zooms, and q/Esc quits.

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

    // Build an icosahedron.
    // Golden ratio for vertex placement.
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;

    let vertices = vec![
        (-1.0, phi, 0.0),
        (1.0, phi, 0.0),
        (-1.0, -phi, 0.0),
        (1.0, -phi, 0.0),
        (0.0, -1.0, phi),
        (0.0, 1.0, phi),
        (0.0, -1.0, -phi),
        (0.0, 1.0, -phi),
        (phi, 0.0, -1.0),
        (phi, 0.0, 1.0),
        (-phi, 0.0, -1.0),
        (-phi, 0.0, 1.0),
    ];

    let faces = vec![
        (0, 11, 5),
        (0, 5, 1),
        (0, 1, 7),
        (0, 7, 10),
        (0, 10, 11),
        (1, 5, 9),
        (5, 11, 4),
        (11, 10, 2),
        (10, 7, 6),
        (7, 1, 8),
        (3, 9, 4),
        (3, 4, 2),
        (3, 2, 6),
        (3, 6, 8),
        (3, 8, 9),
        (4, 9, 5),
        (2, 4, 11),
        (6, 2, 10),
        (8, 6, 7),
        (9, 8, 1),
    ];

    let mesh = Mesh3D::new(vertices, faces)
        .colormap(Plasma)
        .show_wireframe(true)
        .title("Icosahedron - Arrow keys: rotate, +/-: zoom, q: quit");

    if headless_export(|area, buf| {
        StatefulWidget::render(&mesh, area, buf, &mut Camera3DState::default());
    })? {
        return Ok(());
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    io::stdout().execute(crossterm::event::EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut camera_state = Camera3DState::default();

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            frame.render_stateful_widget(&mesh, area, &mut camera_state);
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
