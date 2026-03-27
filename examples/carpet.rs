//! Carpet plot example: parametric grid with scalar field coloring.
//!
//! Displays a curvilinear (a, b) coordinate grid where x and y positions are
//! functions of the two parameters, colored by a scalar field. Press q to quit.

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

    // Parameter arrays
    let na = 12;
    let nb = 10;
    let a: Vec<f64> = (0..na).map(|i| i as f64 * 0.3).collect();
    let b: Vec<f64> = (0..nb).map(|i| i as f64 * 0.3).collect();

    // Build position grids: x[b_idx][a_idx], y[b_idx][a_idx]
    // Create a slightly warped parametric surface
    let mut x_grid = vec![vec![0.0; na]; nb];
    let mut y_grid = vec![vec![0.0; na]; nb];
    let mut values = vec![vec![0.0; na]; nb];

    for (bi, &bv) in b.iter().enumerate() {
        for (ai, &av) in a.iter().enumerate() {
            // Warped coordinates: slight curvature
            x_grid[bi][ai] = av + 0.15 * bv * bv;
            y_grid[bi][ai] = bv + 0.1 * av.sin();
            // Scalar field: radial wave pattern
            let r = ((av - 1.5) * (av - 1.5) + (bv - 1.2) * (bv - 1.2)).sqrt();
            values[bi][ai] = (r * 2.5).sin();
        }
    }

    let plot = CarpetPlot::new(a, b, x_grid, y_grid)
        .values(values)
        .colormap(Plasma)
        .show_grid(true)
        .show_colorbar(true)
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .title("Carpet Plot - Parametric Grid (q to quit)");

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&plot, square_area(frame.area()));
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
