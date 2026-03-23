//! Detailed TriColor example: sin(x)*cos(y) on a 7x7 triangulated grid.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::triangulation::Triangulation;

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

    // Build a 7x7 regular grid on [-3, 3] x [-3, 3]: 49 vertices, 72 triangles.
    let n = 7;
    let lo = -3.0_f64;
    let hi = 3.0_f64;
    let step = (hi - lo) / (n - 1) as f64;

    let mut vertices = Vec::with_capacity(n * n);
    for row in 0..n {
        for col in 0..n {
            vertices.push((lo + col as f64 * step, lo + row as f64 * step));
        }
    }

    let mut triangles = Vec::with_capacity(2 * (n - 1) * (n - 1));
    for row in 0..(n - 1) {
        for col in 0..(n - 1) {
            let bl = row * n + col;
            let br = bl + 1;
            let tl = bl + n;
            let tr = tl + 1;
            triangles.push((bl, br, tl));
            triangles.push((br, tr, tl));
        }
    }

    // Face values: sin(cx) * cos(cy) evaluated at each triangle's centroid.
    let face_values: Vec<f64> = triangles
        .iter()
        .map(|&(a, b, c)| {
            let (ax, ay) = vertices[a];
            let (bx, by) = vertices[b];
            let (cx, cy) = vertices[c];
            let mx = (ax + bx + cx) / 3.0;
            let my = (ay + by + cy) / 3.0;
            mx.sin() * my.cos()
        })
        .collect();

    let tri_color = TriColor::new(Triangulation::from_explicit(vertices, triangles))
        .face_values(face_values)
        .colormap(Plasma)
        .title("TriColor: sin(x)cos(y) (q to quit)")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"));

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            (&tri_color).render(area, frame.buffer_mut());
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
