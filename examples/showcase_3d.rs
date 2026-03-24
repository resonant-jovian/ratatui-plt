//! Showcase 3D: 5 three-dimensional plot types with interactive camera.
//!
//! Replicates matplotlib reference plots:
//!   Top row:    Surface3D  |  Wireframe3D  |  Scatter3D
//!   Bottom row: Bar3D      |  Quiver3D
//!
//! Uses manual Layout splitting because 3D widgets require StatefulWidget
//! with mutable Camera3DState, which is incompatible with MultiPanel closures.
//!
//! Controls: Arrow keys rotate, +/- zoom, q/Esc quit.
//!
//! Run: cargo run --example showcase_3d

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind, MouseEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::bar3d::Bar3DData;

/// Deterministic LCG pseudo-random number generator.
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*seed >> 33) as f64 / (1u64 << 31) as f64
}

/// Box-Muller normal sample from LCG.
fn lcg_normal(seed: &mut u64) -> f64 {
    let u1 = lcg(seed).max(1e-10);
    let u2 = lcg(seed);
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(Theme::light());
    let theme = Theme::get_default();
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    io::stdout().execute(crossterm::event::EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // ── 1. Surface3D: Mexican hat / sinc surface ────────────────────────
    let surface_data = GridData::from_fn((-6.0, 6.0), (-6.0, 6.0), 60, 60, |x, y| {
        let r = (x * x + y * y).sqrt().max(0.001);
        r.sin() / r
    });
    let surface = Surface3D::new(surface_data)
        .colormap(Viridis)
        .show_wireframe(false)
        .camera(Camera3D::new().azimuth(-60.0).elevation(30.0))
        .title("Surface3D: sinc(r)");

    // ── 2. Wireframe3D: same surface as wireframe ───────────────────────
    let wire_data = GridData::from_fn((-6.0, 6.0), (-6.0, 6.0), 30, 30, |x, y| {
        let r = (x * x + y * y).sqrt().max(0.001);
        r.sin() / r
    });
    let wireframe = Wireframe3D::new(wire_data)
        .color(theme.primary)
        .camera(Camera3D::new().azimuth(-60.0).elevation(30.0))
        .title("Wireframe3D: sinc(r)");

    // ── 3. Scatter3D: random 3D point cloud ─────────────────────────────
    let mut seed = 77_u64;
    let n_pts = 150;
    let scatter_data: Vec<(f64, f64, f64)> = (0..n_pts)
        .map(|_| {
            let x = lcg_normal(&mut seed);
            let y = lcg_normal(&mut seed);
            let z = lcg_normal(&mut seed);
            (x, y, z)
        })
        .collect();
    let scatter_values: Vec<f64> = scatter_data
        .iter()
        .map(|&(x, y, z)| (x * x + y * y + z * z).sqrt())
        .collect();
    let scatter_series = Series3D::new("Points")
        .data(scatter_data)
        .values(scatter_values);
    let scatter = Scatter3D::new()
        .series(scatter_series)
        .color_by_value(true)
        .colormap(Viridis)
        .marker(MarkerShape::FilledCircle)
        .camera(Camera3D::new().azimuth(-50.0).elevation(25.0))
        .title("Scatter3D: Gaussian cloud");

    // ── 4. Bar3D: small grid of bars ────────────────────────────────────
    let bar_colors = [
        theme.color_cycle.at(0),
        theme.color_cycle.at(1),
        theme.color_cycle.at(2),
        theme.color_cycle.at(3),
    ];
    let bar_seed = 999_u64;
    let bar_rows = 4_usize;
    let bar_cols = 4_usize;
    let bars: Vec<Bar3DData> = (0..bar_rows)
        .flat_map(|r| {
            (0..bar_cols).map(move |c| {
                let mut s = bar_seed.wrapping_add((r * 31 + c * 17) as u64);
                let h = lcg(&mut s) * 8.0 + 2.0;
                // Update outer seed to stay deterministic across iterations
                Bar3DData::new(r as f64 * 1.5, c as f64 * 1.5, h)
                    .color(bar_colors[r % bar_colors.len()])
                    .width(0.9)
            })
        })
        .collect();
    let bar3d = Bar3D::new(bars)
        .camera(Camera3D::new().azimuth(-50.0).elevation(25.0))
        .title("Bar3D: grid");

    // ── 5. Quiver3D: helical / rotational vector field ──────────────────
    let n_q = 4_i32;
    let arrows: Vec<Arrow3D> = (-n_q..=n_q)
        .flat_map(|ix| {
            (-n_q..=n_q).flat_map(move |iy| {
                (-n_q..=n_q).filter_map(move |iz| {
                    let x = ix as f64 * 0.6;
                    let y = iy as f64 * 0.6;
                    let z = iz as f64 * 0.6;
                    let r2 = x * x + y * y + z * z;
                    if r2 < 0.3 {
                        return None;
                    }
                    // Rotational + upward field
                    let scale = 0.25;
                    let dx = -y * scale;
                    let dy = x * scale;
                    let dz = 0.1 * scale;
                    let mag = (dx * dx + dy * dy + dz * dz).sqrt();
                    let intensity = (mag * 600.0).min(255.0) as u8;
                    Some(Arrow3D::new(x, y, z, dx, dy, dz).color(Color::Rgb(
                        31,
                        119,
                        intensity.max(80),
                    )))
                })
            })
        })
        .collect();
    let quiver3d = Quiver3D::new(arrows)
        .scale(1.0)
        .camera(Camera3D::new().azimuth(-40.0).elevation(25.0))
        .title("Quiver3D: rotation");

    // ── Camera states (one per 3D widget) ───────────────────────────────
    let mut cam_surface = Camera3DState::default();
    let mut cam_wire = Camera3DState::default();
    let mut cam_scatter = Camera3DState::default();
    let mut cam_bar = Camera3DState::default();
    let mut cam_quiver = Camera3DState::default();

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());

            // Title row (1 line)
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(area);

            let title = "ratatui-plt 3D Showcase  (arrows: rotate, +/-: zoom, q: quit)";
            let start = outer[0].x + outer[0].width.saturating_sub(title.len() as u16) / 2;
            for (i, ch) in title.chars().enumerate() {
                let tx = start + i as u16;
                if tx < outer[0].x + outer[0].width {
                    frame
                        .buffer_mut()
                        .cell_mut((tx, outer[0].y))
                        .map(|cell| cell.set_char(ch));
                }
            }

            // Split remaining area into 2 rows
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(outer[1]);

            // Top row: 3 columns
            let top_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(rows[0]);

            // Bottom row: 2 columns (+ empty space)
            let bot_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(rows[1]);

            frame.render_stateful_widget(&surface, top_cols[0], &mut cam_surface);
            frame.render_stateful_widget(&wireframe, top_cols[1], &mut cam_wire);
            frame.render_stateful_widget(&scatter, top_cols[2], &mut cam_scatter);
            frame.render_stateful_widget(&bar3d, bot_cols[0], &mut cam_bar);
            frame.render_stateful_widget(&quiver3d, bot_cols[1], &mut cam_quiver);
            // bot_cols[2] intentionally left empty
        })?;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Left => {
                    cam_surface.rotate(-5.0, 0.0);
                    cam_wire.rotate(-5.0, 0.0);
                    cam_scatter.rotate(-5.0, 0.0);
                    cam_bar.rotate(-5.0, 0.0);
                    cam_quiver.rotate(-5.0, 0.0);
                }
                KeyCode::Right => {
                    cam_surface.rotate(5.0, 0.0);
                    cam_wire.rotate(5.0, 0.0);
                    cam_scatter.rotate(5.0, 0.0);
                    cam_bar.rotate(5.0, 0.0);
                    cam_quiver.rotate(5.0, 0.0);
                }
                KeyCode::Up => {
                    cam_surface.rotate(0.0, 5.0);
                    cam_wire.rotate(0.0, 5.0);
                    cam_scatter.rotate(0.0, 5.0);
                    cam_bar.rotate(0.0, 5.0);
                    cam_quiver.rotate(0.0, 5.0);
                }
                KeyCode::Down => {
                    cam_surface.rotate(0.0, -5.0);
                    cam_wire.rotate(0.0, -5.0);
                    cam_scatter.rotate(0.0, -5.0);
                    cam_bar.rotate(0.0, -5.0);
                    cam_quiver.rotate(0.0, -5.0);
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    cam_surface.zoom(1.2);
                    cam_wire.zoom(1.2);
                    cam_scatter.zoom(1.2);
                    cam_bar.zoom(1.2);
                    cam_quiver.zoom(1.2);
                }
                KeyCode::Char('-') => {
                    cam_surface.zoom(0.8);
                    cam_wire.zoom(0.8);
                    cam_scatter.zoom(0.8);
                    cam_bar.zoom(0.8);
                    cam_quiver.zoom(0.8);
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => {
                    cam_surface.zoom(1.2);
                    cam_wire.zoom(1.2);
                    cam_scatter.zoom(1.2);
                    cam_bar.zoom(1.2);
                    cam_quiver.zoom(1.2);
                }
                MouseEventKind::ScrollDown => {
                    cam_surface.zoom(0.8);
                    cam_wire.zoom(0.8);
                    cam_scatter.zoom(0.8);
                    cam_bar.zoom(0.8);
                    cam_quiver.zoom(0.8);
                }
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
