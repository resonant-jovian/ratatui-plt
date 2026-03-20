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

    let chart = PieChart::new()
        .slice(PieSlice::new("Python", 28.1).color(Color::Cyan))
        .slice(PieSlice::new("JS", 17.4).color(Color::Yellow))
        .slice(PieSlice::new("Java", 15.8).color(Color::Red))
        .slice(PieSlice::new("C/C++", 12.3).color(Color::Green))
        .slice(PieSlice::new("C#", 7.5).color(Color::Magenta))
        .slice(PieSlice::new("Other", 18.9).color(Color::Gray))
        .donut_ratio(0.35)
        .show_labels(true)
        .title("Programming Language Market Share (q to quit)");

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
