//! 3D voxel sphere rendered with the `Voxels` widget.
//!
//! Fills a 10x10x10 grid with voxels whose distance from the center
//! is less than a given radius, producing a discrete sphere. Arrow
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

    // Build a 10x10x10 voxel sphere: occupied if distance from center < radius.
    let size = 10;
    let center = size as f64 / 2.0;
    let radius = 4.0;

    let grid: Vec<Vec<Vec<bool>>> = (0..size)
        .map(|z| {
            (0..size)
                .map(|y| {
                    (0..size)
                        .map(|x| {
                            let dx = x as f64 + 0.5 - center;
                            let dy = y as f64 + 0.5 - center;
                            let dz = z as f64 + 0.5 - center;
                            (dx * dx + dy * dy + dz * dz).sqrt() < radius
                        })
                        .collect()
                })
                .collect()
        })
        .collect();

    let voxel_plot =
        Voxels::new(grid).title("Voxel Sphere - Arrow keys: rotate, +/-: zoom, q: quit");

    if headless_export(|area, buf| {
        StatefulWidget::render(&voxel_plot, area, buf, &mut Camera3DState::default());
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
            frame.render_stateful_widget(&voxel_plot, area, &mut camera_state);
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
