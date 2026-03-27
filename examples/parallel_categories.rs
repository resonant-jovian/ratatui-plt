//! Parallel categories example.
//!
//! Shows categorical flow data across three dimensions: department, role level,
//! and work mode, demonstrating how records flow between categories.

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

    let chart = ParallelCategories::new()
        .dimension(CategoricalDimension::new(
            "Department",
            vec!["Engineering", "Marketing", "Sales"],
        ))
        .dimension(CategoricalDimension::new(
            "Level",
            vec!["Junior", "Mid", "Senior"],
        ))
        .dimension(CategoricalDimension::new(
            "Work Mode",
            vec!["Remote", "Hybrid", "Office"],
        ))
        // Engineering flows
        .record(CategoricalRecord::new(vec![0, 0, 0]).count(35))
        .record(CategoricalRecord::new(vec![0, 0, 1]).count(15))
        .record(CategoricalRecord::new(vec![0, 1, 0]).count(45))
        .record(CategoricalRecord::new(vec![0, 1, 1]).count(25))
        .record(CategoricalRecord::new(vec![0, 1, 2]).count(10))
        .record(CategoricalRecord::new(vec![0, 2, 0]).count(30))
        .record(CategoricalRecord::new(vec![0, 2, 1]).count(20))
        // Marketing flows
        .record(CategoricalRecord::new(vec![1, 0, 1]).count(20))
        .record(CategoricalRecord::new(vec![1, 0, 2]).count(10))
        .record(CategoricalRecord::new(vec![1, 1, 0]).count(15))
        .record(CategoricalRecord::new(vec![1, 1, 1]).count(25))
        .record(CategoricalRecord::new(vec![1, 1, 2]).count(15))
        .record(CategoricalRecord::new(vec![1, 2, 1]).count(18))
        .record(CategoricalRecord::new(vec![1, 2, 2]).count(12))
        // Sales flows
        .record(CategoricalRecord::new(vec![2, 0, 1]).count(15))
        .record(CategoricalRecord::new(vec![2, 0, 2]).count(25))
        .record(CategoricalRecord::new(vec![2, 1, 1]).count(20))
        .record(CategoricalRecord::new(vec![2, 1, 2]).count(30))
        .record(CategoricalRecord::new(vec![2, 2, 1]).count(10))
        .record(CategoricalRecord::new(vec![2, 2, 2]).count(20))
        .title("Workforce Distribution (q to quit)");

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
