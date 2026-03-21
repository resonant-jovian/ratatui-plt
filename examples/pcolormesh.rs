//! Pcolormesh example: Irregular (deformed) grid.
//!
//! Creates a regular grid, applies a sinusoidal deformation to vertex positions,
//! and colors cells by the original function value. Displayed with colorbar.

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
        Some("dark") | None => Theme::dark(),
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

    // Grid dimensions: (nrows+1) x (ncols+1) vertices, nrows x ncols cells
    let nrows = 20;
    let ncols = 20;

    // Build deformed vertex grid
    let mut x_verts = vec![vec![0.0_f64; ncols + 1]; nrows + 1];
    let mut y_verts = vec![vec![0.0_f64; ncols + 1]; nrows + 1];

    for i in 0..=nrows {
        for j in 0..=ncols {
            let u = j as f64 / ncols as f64; // [0, 1]
            let v = i as f64 / nrows as f64; // [0, 1]

            // Base position scaled to [-2, 2]
            let bx = -2.0 + 4.0 * u;
            let by = -2.0 + 4.0 * v;

            // Sinusoidal deformation
            let deform_x = 0.3 * (by * std::f64::consts::PI).sin();
            let deform_y = 0.3 * (bx * std::f64::consts::PI).sin();

            x_verts[i][j] = bx + deform_x;
            y_verts[i][j] = by + deform_y;
        }
    }

    // Cell values: evaluate function at cell centers (average of the 4 base positions)
    let mut values = vec![vec![0.0_f64; ncols]; nrows];
    for (i, row) in values.iter_mut().enumerate() {
        for (j, val) in row.iter_mut().enumerate() {
            let u = (j as f64 + 0.5) / ncols as f64;
            let v = (i as f64 + 0.5) / nrows as f64;
            let cx = -2.0 + 4.0 * u;
            let cy = -2.0 + 4.0 * v;
            // Gaussian-like function
            *val = (-(cx * cx + cy * cy) / 2.0).exp();
        }
    }

    let plot = Pcolormesh::new(x_verts, y_verts, values)
        .colormap(Plasma)
        .show_colorbar(true)
        .title("Irregular Mesh: Deformed Grid (q to quit)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&plot, frame.area());
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
