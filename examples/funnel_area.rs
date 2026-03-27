//! Funnel area chart example: sales pipeline with proportional trapezoid stages.
//!
//! Unlike the standard funnel chart which uses equal-height bars, the funnel
//! area chart renders each stage as a trapezoid whose width is proportional
//! to its value. Press q to quit.

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

    let theme = Theme::get_default();
    let mut cycle = theme.color_cycle.clone();

    let chart = FunnelArea::new()
        .entry(FunnelAreaEntry::new("Website Visitors", 12000.0).color(cycle.next_color()))
        .entry(FunnelAreaEntry::new("Sign-ups", 7500.0).color(cycle.next_color()))
        .entry(FunnelAreaEntry::new("Free Trial", 4200.0).color(cycle.next_color()))
        .entry(FunnelAreaEntry::new("Qualified Leads", 2100.0).color(cycle.next_color()))
        .entry(FunnelAreaEntry::new("Proposals Sent", 1200.0).color(cycle.next_color()))
        .entry(FunnelAreaEntry::new("Closed Won", 650.0).color(cycle.next_color()))
        .show_percentages(true)
        .show_values(true)
        .title("Sales Pipeline - Funnel Area (q to quit)");

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
