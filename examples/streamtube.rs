//! 3D streamlines rendered with the `Streamtube` widget.
//!
//! Generates several streamline paths by integrating a simple rotating
//! vector field v = (-y, x, 0.3) from different seed points, producing
//! helical trajectories. Arrow keys rotate the camera, +/- zoom, scroll
//! wheel zooms, and q/Esc quits.

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

/// Integrate a streamline through the vector field v = (-y, x, 0.3)
/// starting from `seed` using simple Euler steps.
fn integrate_streamline(seed: (f64, f64, f64), steps: usize, dt: f64) -> Vec<(f64, f64, f64)> {
    let mut path = Vec::with_capacity(steps);
    let (mut x, mut y, mut z) = seed;
    for _ in 0..steps {
        path.push((x, y, z));
        let vx = -y;
        let vy = x;
        let vz = 0.3;
        x += vx * dt;
        y += vy * dt;
        z += vz * dt;
    }
    path
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());

    // Generate streamlines from several seed points at different radii.
    let seeds = [
        (1.0, 0.0, -1.0),
        (0.0, 1.0, -1.0),
        (-1.0, 0.0, -1.0),
        (0.0, -1.0, -1.0),
        (0.5, 0.5, -1.0),
        (-0.5, -0.5, -1.0),
    ];

    let paths: Vec<Vec<(f64, f64, f64)>> = seeds
        .iter()
        .map(|&seed| integrate_streamline(seed, 200, 0.05))
        .collect();

    let stream = Streamtube::new(paths)
        .tube_radius(0.08)
        .title("3D Streamlines - Arrow keys: rotate, +/-: zoom, q: quit");

    if headless_export(|area, buf| {
        StatefulWidget::render(&stream, area, buf, &mut Camera3DState::default());
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
            frame.render_stateful_widget(&stream, area, &mut camera_state);
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
