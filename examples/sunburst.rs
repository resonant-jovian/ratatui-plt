//! Sunburst chart example: world population by continent and region.
//!
//! Displays a hierarchical sunburst showing approximate world population
//! broken down by continent and then by sub-region, rendered as
//! concentric rings.

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

    // Population values in millions (approximate)
    let root = SunburstNode::new("World", 0.0)
        .child(
            SunburstNode::new("Asia", 0.0)
                .color(Color::Red)
                .child(SunburstNode::new("East Asia", 1700.0).color(Color::LightRed))
                .child(SunburstNode::new("South Asia", 2000.0).color(Color::Red))
                .child(SunburstNode::new("SE Asia", 700.0).color(Color::Rgb(200, 80, 80))),
        )
        .child(
            SunburstNode::new("Africa", 0.0)
                .color(Color::Yellow)
                .child(SunburstNode::new("East Africa", 500.0).color(Color::LightYellow))
                .child(SunburstNode::new("West Africa", 450.0).color(Color::Yellow))
                .child(SunburstNode::new("North Africa", 250.0).color(Color::Rgb(200, 200, 80))),
        )
        .child(
            SunburstNode::new("Europe", 0.0)
                .color(Color::Cyan)
                .child(SunburstNode::new("Western EU", 400.0).color(Color::LightCyan))
                .child(SunburstNode::new("Eastern EU", 300.0).color(Color::Cyan)),
        )
        .child(
            SunburstNode::new("Americas", 0.0)
                .color(Color::Green)
                .child(SunburstNode::new("N. America", 380.0).color(Color::LightGreen))
                .child(SunburstNode::new("S. America", 440.0).color(Color::Green))
                .child(SunburstNode::new("C. America", 180.0).color(Color::Rgb(80, 200, 80))),
        )
        .child(
            SunburstNode::new("Oceania", 0.0)
                .color(Color::Magenta)
                .child(SunburstNode::new("Australia/NZ", 32.0).color(Color::LightMagenta))
                .child(SunburstNode::new("Pacific Is.", 13.0).color(Color::Magenta)),
        );

    let chart = Sunburst::new(root).title("World Population Sunburst (q to quit)");

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
