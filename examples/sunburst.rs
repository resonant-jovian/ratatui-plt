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

    // Population values in millions (approximate)
    let root = SunburstNode::new("World", 0.0)
        .child(
            SunburstNode::new("Asia", 0.0)
                .color(Color::Rgb(220, 60, 60))
                .child(SunburstNode::new("East Asia", 1700.0).color(Color::Rgb(240, 100, 100)))
                .child(SunburstNode::new("South Asia", 2000.0).color(Color::Rgb(200, 50, 50)))
                .child(SunburstNode::new("SE Asia", 700.0).color(Color::Rgb(180, 70, 70))),
        )
        .child(
            SunburstNode::new("Africa", 0.0)
                .color(Color::Rgb(230, 190, 40))
                .child(SunburstNode::new("East Africa", 500.0).color(Color::Rgb(250, 220, 80)))
                .child(SunburstNode::new("West Africa", 450.0).color(Color::Rgb(220, 180, 30)))
                .child(SunburstNode::new("North Africa", 250.0).color(Color::Rgb(200, 170, 50))),
        )
        .child(
            SunburstNode::new("Europe", 0.0)
                .color(Color::Rgb(60, 180, 200))
                .child(SunburstNode::new("Western EU", 400.0).color(Color::Rgb(100, 210, 230)))
                .child(SunburstNode::new("Eastern EU", 300.0).color(Color::Rgb(50, 160, 180))),
        )
        .child(
            SunburstNode::new("Americas", 0.0)
                .color(Color::Rgb(60, 180, 80))
                .child(SunburstNode::new("N. America", 380.0).color(Color::Rgb(100, 220, 110)))
                .child(SunburstNode::new("S. America", 440.0).color(Color::Rgb(50, 170, 70)))
                .child(SunburstNode::new("C. America", 180.0).color(Color::Rgb(70, 200, 90))),
        )
        .child(
            SunburstNode::new("Oceania", 0.0)
                .color(Color::Rgb(180, 80, 200))
                .child(SunburstNode::new("Australia/NZ", 32.0).color(Color::Rgb(210, 120, 240)))
                .child(SunburstNode::new("Pacific Is.", 13.0).color(Color::Rgb(160, 70, 190))),
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
