//! Tri-contour plot example.
//!
//! Shows contour lines on scattered elevation data using the marching triangles
//! algorithm on a Delaunay triangulation.

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
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Generate scattered elevation data points on a grid with slight jitter
    let mut points = Vec::new();
    let mut values = Vec::new();
    let steps = 12;

    for i in 0..=steps {
        for j in 0..=steps {
            let x = i as f64 / steps as f64 * 10.0;
            let y = j as f64 / steps as f64 * 10.0;

            // Small deterministic displacement based on position
            let jx = 0.15 * ((x * 3.7 + y * 2.1).sin());
            let jy = 0.15 * ((x * 1.3 + y * 4.9).cos());

            let px = x + jx;
            let py = y + jy;

            // Elevation: sum of two Gaussian-like peaks and a ridge
            let peak1 = 8.0 * (-((px - 3.0).powi(2) + (py - 3.0).powi(2)) / 4.0).exp();
            let peak2 = 6.0 * (-((px - 7.0).powi(2) + (py - 7.0).powi(2)) / 3.0).exp();
            let ridge = 2.0 * (-(py - 5.0).powi(2) / 8.0).exp();

            points.push((px, py));
            values.push(peak1 + peak2 + ridge);
        }
    }

    let tri = Triangulation::from_points(&points);

    let chart = TriContour::new(tri)
        .vertex_values(values)
        .levels_auto(10)
        .x_axis(Axis::new().label("X (km)"))
        .y_axis(Axis::new().label("Y (km)"))
        .title("Terrain Elevation Contours (q to quit)");

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&chart, square_area(frame.area()));
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
