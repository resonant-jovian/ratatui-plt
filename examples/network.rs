//! Network graph example: social network visualization.
//!
//! Displays a small social network with 8 people connected by edges
//! of varying weight. Uses force-directed layout to position nodes.
//! Nodes are colored by community membership.

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
    let theme = Theme::get_default();
    let community1 = theme.primary;
    let community2 = theme.secondary;
    let community3 = theme.accent;
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Community 1: Alice, Bob, Carol — upper left triangle
    // Community 2: Dave, Eve, Frank — upper right triangle
    // Community 3: Grace, Hank — bottom center pair
    //
    // Manual layout gives clean, readable edge orientations.
    let plot = NetworkPlot::new()
        // Community 1 — upper left
        .node(
            GraphNode::new("Alice")
                .color(community1)
                .position(-3.0, 2.0)
                .marker(MarkerShape::FilledCircle),
        ) // 0
        .node(
            GraphNode::new("Bob")
                .color(community1)
                .position(-4.0, 0.0)
                .marker(MarkerShape::FilledCircle),
        ) // 1
        .node(
            GraphNode::new("Carol")
                .color(community1)
                .position(-1.5, 0.0)
                .marker(MarkerShape::FilledCircle),
        ) // 2
        // Community 2 — upper right
        .node(
            GraphNode::new("Dave")
                .color(community2)
                .position(1.5, 0.0)
                .marker(MarkerShape::FilledCircle),
        ) // 3
        .node(
            GraphNode::new("Eve")
                .color(community2)
                .position(3.0, 2.0)
                .marker(MarkerShape::FilledCircle),
        ) // 4
        .node(
            GraphNode::new("Frank")
                .color(community2)
                .position(4.0, 0.0)
                .marker(MarkerShape::FilledCircle),
        ) // 5
        // Community 3 — bottom center
        .node(
            GraphNode::new("Grace")
                .color(community3)
                .position(-1.0, -2.5)
                .marker(MarkerShape::FilledCircle),
        ) // 6
        .node(
            GraphNode::new("Hank")
                .color(community3)
                .position(1.0, -2.5)
                .marker(MarkerShape::FilledCircle),
        ) // 7
        // Intra-community edges (strong ties)
        .edge(GraphEdge::new(0, 1, 3.0)) // Alice - Bob
        .edge(GraphEdge::new(1, 2, 3.0)) // Bob - Carol
        .edge(GraphEdge::new(0, 2, 2.5)) // Alice - Carol
        .edge(GraphEdge::new(3, 4, 3.0)) // Dave - Eve
        .edge(GraphEdge::new(4, 5, 2.5)) // Eve - Frank
        .edge(GraphEdge::new(3, 5, 2.0)) // Dave - Frank
        .edge(GraphEdge::new(6, 7, 3.0)) // Grace - Hank
        // Inter-community bridges (weaker ties)
        .edge(GraphEdge::new(2, 3, 1.0)) // Carol - Dave (bridge 1-2)
        .edge(GraphEdge::new(1, 6, 1.0)) // Bob - Grace (bridge 1-3)
        .edge(GraphEdge::new(5, 7, 1.0)) // Frank - Hank (bridge 2-3)
        .edge(GraphEdge::new(0, 4, 0.5)) // Alice - Eve (weak bridge)
        .layout(GraphLayout::Manual)
        .title("Social Network Graph (q to quit)")
        .show_labels(true);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&plot, square_area(frame.area()));
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
