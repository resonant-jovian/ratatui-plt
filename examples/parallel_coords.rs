//! Parallel coordinates example: Iris Dataset.
//!
//! Approximates the classic Iris dataset with 3 species (setosa, versicolor,
//! virginica) across 4 morphological measurements. Data is generated
//! deterministically using LCG-based pseudo-random offsets around known
//! species means, producing ~15 samples per species.

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

/// Generate `count` samples for a species, given mean values and spreads for
/// each of the 4 axes. Uses an LCG for deterministic pseudo-random offsets.
fn generate_species(
    seed: u64,
    count: usize,
    means: [f64; 4],
    spreads: [f64; 4],
    color: Color,
    name: &str,
) -> Vec<ParallelRecord> {
    let mut state = seed;
    let mut records = Vec::with_capacity(count);

    for i in 0..count {
        let values: Vec<f64> = (0..4)
            .map(|ax| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let u = ((state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0;
                (means[ax] + u * spreads[ax]).max(0.0)
            })
            .collect();

        let mut rec = ParallelRecord::new(values).color(color);
        // Name only the first record for a clean legend
        if i == 0 {
            rec = rec.name(name);
        }
        records.push(rec);
    }

    records
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    let theme = Theme::get_default();
    let mut cycle = theme.color_cycle.clone();
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Axes: Sepal Length, Sepal Width, Petal Length, Petal Width
    // Ranges approximate the real Iris dataset bounds
    let axes = vec![
        ParallelAxis::new("Sep.L", 4.0, 8.0),
        ParallelAxis::new("Sep.W", 2.0, 4.5),
        ParallelAxis::new("Pet.L", 1.0, 7.0),
        ParallelAxis::new("Pet.W", 0.0, 2.6),
    ];

    // Approximate species statistics from the real Iris dataset:
    //
    // Setosa:     SL~5.0 SW~3.4 PL~1.5 PW~0.2
    // Versicolor: SL~5.9 SW~2.8 PL~4.3 PW~1.3
    // Virginica:  SL~6.6 SW~3.0 PL~5.6 PW~2.0
    let setosa = generate_species(
        42,
        15,
        [5.0, 3.4, 1.5, 0.25],
        [0.35, 0.35, 0.18, 0.10],
        cycle.next_color(),
        "Setosa",
    );

    let versicolor = generate_species(
        137,
        15,
        [5.9, 2.8, 4.3, 1.3],
        [0.50, 0.30, 0.45, 0.20],
        cycle.next_color(),
        "Versicolor",
    );

    let virginica = generate_species(
        271,
        15,
        [6.6, 3.0, 5.6, 2.0],
        [0.60, 0.30, 0.50, 0.25],
        cycle.next_color(),
        "Virginica",
    );

    let mut all_records = Vec::new();
    all_records.extend(setosa);
    all_records.extend(versicolor);
    all_records.extend(virginica);

    let plot = ParallelCoords::new()
        .axes(axes)
        .records(all_records)
        .title("Iris Dataset - Parallel Coordinates (q to quit)")
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

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
