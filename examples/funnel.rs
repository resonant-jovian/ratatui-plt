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

    let chart = FunnelChart::new()
        .entry(FunnelEntry::new("Visitors", 10000.0).color(Color::Cyan))
        .entry(FunnelEntry::new("Leads", 6500.0).color(Color::Blue))
        .entry(FunnelEntry::new("Qualified", 3200.0).color(Color::Yellow))
        .entry(FunnelEntry::new("Proposals", 1800.0).color(Color::Green))
        .entry(FunnelEntry::new("Closed Won", 950.0).color(Color::Magenta))
        .show_percentages(true)
        .show_values(true)
        .title("Sales Funnel (q to quit)");

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&chart, frame.area());
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
