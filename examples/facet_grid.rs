//! Facet grid example: seaborn-style faceted scatter plot.
//!
//! Demonstrates FacetGrid with 2 row groups, 3 column groups, and 2 hue levels.

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

    // Build synthetic data: 2 rows x 3 cols x 2 hue levels
    let row_labels = ["Low", "High"];
    let col_labels = ["Alpha", "Beta", "Gamma"];
    let hue_labels = ["Group A", "Group B"];

    let mut data = FacetData::new();

    for (ri, row) in row_labels.iter().enumerate() {
        for (ci, col) in col_labels.iter().enumerate() {
            for (hi, hue) in hue_labels.iter().enumerate() {
                // Generate deterministic pseudo-random data per cell+hue
                let offset_x = ci as f64 * 2.0;
                let offset_y = ri as f64 * 3.0 + hi as f64 * 1.5;
                for i in 0..20 {
                    let t = i as f64 * 0.3;
                    let x = offset_x + t + (t * (hi as f64 + 1.0)).sin() * 0.5;
                    let y = offset_y + t * 0.5 + (t * 1.3 + ci as f64).cos() * 0.8;
                    data = data.record(FacetRecord {
                        row_key: (*row).to_string(),
                        col_key: (*col).to_string(),
                        hue_key: Some((*hue).to_string()),
                        x,
                        y,
                    });
                }
            }
        }
    }

    let grid = FacetGrid::new(data)
        .map_scatter()
        .share_x(true)
        .share_y(true)
        .suptitle("Faceted Scatter (q to quit)")
        .col_titles(true)
        .row_titles(true)
        .gap(1);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&grid, square_area(frame.area()));
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
