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

    let chart = GanttChart::new()
        .task(
            GanttTask::new("Research")
                .segment(0.0, 3.0)
                .color(Color::Cyan),
        )
        .task(
            GanttTask::new("Design")
                .segment(2.0, 4.0)
                .color(Color::Blue),
        )
        .task(
            GanttTask::new("Develop")
                .segment(5.0, 6.0)
                .segment(12.0, 2.0)
                .color(Color::Green),
        )
        .task(
            GanttTask::new("Testing")
                .segment(10.0, 3.0)
                .color(Color::Yellow),
        )
        .task(
            GanttTask::new("Deploy")
                .segment(13.0, 1.0)
                .color(Color::Magenta),
        )
        .show_grid(true)
        .title("Project Timeline (q to quit)");

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
