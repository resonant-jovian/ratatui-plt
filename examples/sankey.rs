//! Sankey diagram example: energy flow visualization.
//!
//! Shows an energy flow diagram with sources (Solar, Wind, Grid) flowing
//! through to consumption sectors (Industrial, Residential, Transport)
//! with varying flow magnitudes.

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

    // Column 0: Energy sources
    // Column 1: Distribution / intermediate
    // Column 2: End-use sectors
    let diagram = SankeyDiagram::new()
        // Sources (column 0)
        .node(SankeyNode::new("Solar").color(Color::Yellow)) // 0
        .node(SankeyNode::new("Wind").color(Color::Cyan)) // 1
        .node(SankeyNode::new("Grid").color(Color::LightRed)) // 2
        // Intermediate (column 1)
        .node(SankeyNode::new("Electricity").color(Color::White)) // 3
        .node(SankeyNode::new("Heat").color(Color::Red)) // 4
        // End-use (column 2)
        .node(SankeyNode::new("Industrial").color(Color::Magenta)) // 5
        .node(SankeyNode::new("Residential").color(Color::Green)) // 6
        .node(SankeyNode::new("Transport").color(Color::Blue)) // 7
        // Flows: sources -> intermediate
        .flow(SankeyFlow::new(0, 3, 30.0)) // Solar -> Electricity
        .flow(SankeyFlow::new(1, 3, 25.0)) // Wind -> Electricity
        .flow(SankeyFlow::new(2, 3, 20.0)) // Grid -> Electricity
        .flow(SankeyFlow::new(2, 4, 15.0)) // Grid -> Heat
        // Flows: intermediate -> end-use
        .flow(SankeyFlow::new(3, 5, 35.0)) // Electricity -> Industrial
        .flow(SankeyFlow::new(3, 6, 25.0)) // Electricity -> Residential
        .flow(SankeyFlow::new(3, 7, 15.0)) // Electricity -> Transport
        .flow(SankeyFlow::new(4, 5, 8.0)) // Heat -> Industrial
        .flow(SankeyFlow::new(4, 6, 7.0)) // Heat -> Residential
        .title("Energy Flow Sankey Diagram (q to quit)");

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&diagram, frame.area());
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
