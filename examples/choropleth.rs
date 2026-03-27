//! Choropleth map example: world population by region.
//!
//! Displays a tile-based world map with regions colored by population data,
//! mapped through a colormap with a colorbar. Press q to quit.

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

    // World population by region (millions, approximate 2024 data)
    let map = ChoroplethMap::new()
        .region(ChoroplethRegion::new("North America", 380.0))
        .region(ChoroplethRegion::new("South America", 430.0))
        .region(ChoroplethRegion::new("Europe", 450.0))
        .region(ChoroplethRegion::new("Africa", 1460.0))
        .region(ChoroplethRegion::new("Asia", 4750.0))
        .region(ChoroplethRegion::new("Oceania", 46.0))
        .map_type(MapType::World)
        .colormap(Viridis)
        .show_colorbar(true)
        .title("World Population by Region (millions) - q to quit");

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&map, square_area(frame.area()));
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
