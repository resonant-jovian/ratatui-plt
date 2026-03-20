//! Treemap example: disk space usage visualization.
//!
//! Shows hierarchical storage usage with nested categories for
//! Documents, Media, and Code directories, each with sub-items.

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

    let root = TreemapNode::new("Storage", 0.0)
        .child(
            TreemapNode::new("Documents", 0.0)
                .color(Color::Cyan)
                .child(TreemapNode::new("PDFs", 450.0))
                .child(TreemapNode::new("Spreadsheets", 280.0))
                .child(TreemapNode::new("Reports", 190.0)),
        )
        .child(
            TreemapNode::new("Media", 0.0)
                .color(Color::Yellow)
                .child(TreemapNode::new("Photos", 1200.0))
                .child(TreemapNode::new("Videos", 3400.0))
                .child(TreemapNode::new("Music", 800.0)),
        )
        .child(
            TreemapNode::new("Code", 0.0)
                .color(Color::Green)
                .child(TreemapNode::new("Repos", 650.0))
                .child(TreemapNode::new("Dependencies", 1100.0)),
        );

    let chart = Treemap::new(root).title("Disk Usage Treemap (q to quit)");

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
