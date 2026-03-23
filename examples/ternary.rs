//! Ternary plot example: soil texture classification.
//!
//! Plots three soil types (Sandy Loam, Clay, Silt Loam) on a ternary
//! diagram with Sand, Silt, and Clay axes. Each dataset uses
//! deterministic pseudo-random points clustered in the appropriate region.

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

/// Simple deterministic pseudo-random number generator (xorshift32).
/// Returns a value in [0, 1).
fn pseudo_rand(state: &mut u32) -> f64 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state as f64) / (u32::MAX as f64)
}

/// Generate ternary points clustered around a center (a, b, c) with some spread.
fn generate_cluster(
    center: (f64, f64, f64),
    n: usize,
    spread: f64,
    seed: u32,
) -> Vec<(f64, f64, f64)> {
    let mut state = seed;
    let mut points = Vec::with_capacity(n);
    for _ in 0..n {
        let da = (pseudo_rand(&mut state) - 0.5) * spread;
        let db = (pseudo_rand(&mut state) - 0.5) * spread;
        let dc = (pseudo_rand(&mut state) - 0.5) * spread;
        let a = (center.0 + da).max(0.01);
        let b = (center.1 + db).max(0.01);
        let c = (center.2 + dc).max(0.01);
        // Normalize so a + b + c = 1
        let sum = a + b + c;
        points.push((a / sum, b / sum, c / sum));
    }
    points
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Sandy Loam: high sand (a), moderate silt (b), low clay (c)
    let sandy_loam = TernaryData::new("Sandy Loam")
        .points(generate_cluster((0.70, 0.18, 0.12), 10, 0.20, 42))
        .color(Color::Yellow)
        .marker(MarkerShape::FilledCircle);

    // Clay: low sand (a), low silt (b), high clay (c)
    let clay = TernaryData::new("Clay")
        .points(generate_cluster((0.12, 0.15, 0.73), 10, 0.18, 137))
        .color(Color::Red)
        .marker(MarkerShape::Triangle);

    // Silt Loam: low sand (a), high silt (b), moderate clay (c)
    let silt_loam = TernaryData::new("Silt Loam")
        .points(generate_cluster((0.15, 0.68, 0.17), 10, 0.18, 256))
        .color(Color::Cyan)
        .marker(MarkerShape::Diamond);

    let plot = TernaryPlot::new()
        .dataset(sandy_loam)
        .dataset(clay)
        .dataset(silt_loam)
        .corner_labels("Sand", "Silt", "Clay")
        .grid_divisions(4)
        .show_tick_labels(true)
        .title("Soil Texture Classification (q to quit)");

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
