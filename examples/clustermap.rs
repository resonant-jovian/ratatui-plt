//! Clustered heatmap example.
//!
//! Shows an 8x8 symmetric correlation matrix with row and column dendrograms
//! using the `ClusterMap` widget.

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

    // Generate an 8x8 symmetric "correlation" matrix
    let n = 8;
    let labels: Vec<f64> = (0..n).map(|i| i as f64).collect();

    // Seed values for reproducible pseudo-correlations
    let base: [f64; 8] = [1.0, 0.85, 0.6, 0.3, -0.1, -0.4, -0.7, -0.9];
    let values: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j: usize| {
                    if i == j {
                        1.0
                    } else {
                        let diff = j.abs_diff(i);
                        base[diff % base.len()] * (1.0 - 0.05 * diff as f64)
                    }
                })
                .collect()
        })
        .collect();

    let data = GridData::new(labels.clone(), labels, values);

    // Build dendrogram linkage (7 merges for 8 leaves)
    let row_links = vec![
        DendroLink::new(0, 1, 0.15),
        DendroLink::new(2, 3, 0.30),
        DendroLink::new(8, 9, 0.50),
        DendroLink::new(4, 5, 0.40),
        DendroLink::new(6, 7, 0.35),
        DendroLink::new(11, 12, 0.70),
        DendroLink::new(10, 13, 1.00),
    ];

    let chart = ClusterMap::new(data)
        .row_links(row_links.clone())
        .col_links(row_links)
        .show_colorbar(true)
        .title("Correlation Cluster Map (q to quit)");

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
