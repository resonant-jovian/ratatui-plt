//! 3D Quiver (vector field) example: Magnetic dipole pattern.
//!
//! Generates a 3D vector field from a magnetic dipole aligned along the z-axis.
//! Interactive: arrow keys rotate, +/- zoom, mouse scroll zoom.

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

    // Generate a magnetic dipole field on a 3D grid.
    // Dipole moment m = (0, 0, 1) at the origin.
    // B(r) = (3(m . r_hat)r_hat - m) / |r|^3  (simplified, ignoring constants)
    let n = 5; // grid points per axis: 5x5x5 = 125 arrows
    let arrows: Vec<Arrow3D> = (-n..=n)
        .flat_map(|ix| {
            (-n..=n).flat_map(move |iy| {
                (-n..=n).filter_map(move |iz| {
                    let x = ix as f64 * 0.5;
                    let y = iy as f64 * 0.5;
                    let z = iz as f64 * 0.5;

                    let r2 = x * x + y * y + z * z;
                    if r2 < 0.5 {
                        return None; // skip near origin (singularity)
                    }
                    let r = r2.sqrt();
                    let r3 = r * r * r;
                    let r5 = r3 * r * r;

                    // Dipole moment along z: m = (0, 0, 1)
                    let m_dot_r = z; // m . r = z
                    let scale = 0.3;

                    let bx = scale * (3.0 * m_dot_r * x / r5);
                    let by = scale * (3.0 * m_dot_r * y / r5);
                    let bz = scale * (3.0 * m_dot_r * z / r5 - 1.0 / r3);

                    // Color by field magnitude
                    let mag = (bx * bx + by * by + bz * bz).sqrt();
                    let intensity = (mag * 200.0).min(255.0) as u8;
                    let color = Color::Rgb(intensity, 80, 255 - intensity);

                    Some(Arrow3D::new(x, y, z, bx, by, bz).color(color))
                })
            })
        })
        .collect();

    let plot = Quiver3D::new(arrows)
        .scale(1.0)
        .camera(Camera3D::new().azimuth(-40.0).elevation(25.0))
        .title("3D Vector Field: Dipole - Arrow keys: rotate, +/-: zoom, q: quit");

    if headless_export(|area, buf| {
        StatefulWidget::render(&plot, area, buf, &mut Camera3DState::default());
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
            frame.render_stateful_widget(&plot, area, &mut camera_state);
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
