use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::bar_chart::{BarDataset, BarMode};

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

    let categories = vec!["Q1", "Q2", "Q3", "Q4", "Q5"];
    let widgets = BarDataset::new(
        "Widgets",
        vec![120.0, 150.0, 180.0, 140.0, 200.0],
        Color::Cyan,
    );
    let gadgets = BarDataset::new(
        "Gadgets",
        vec![90.0, 110.0, 130.0, 160.0, 175.0],
        Color::Yellow,
    );
    let gizmos = BarDataset::new(
        "Gizmos",
        vec![60.0, 80.0, 100.0, 120.0, 90.0],
        Color::Magenta,
    );

    let chart = BarChart::new()
        .categories(categories)
        .dataset(widgets)
        .dataset(gadgets)
        .dataset(gizmos)
        .mode(BarMode::Grouped)
        .bar_gap(1)
        .title("Quarterly Revenue by Product (q to quit)");

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
