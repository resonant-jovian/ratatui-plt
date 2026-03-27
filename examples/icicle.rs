//! Icicle chart example.
//!
//! Shows a file system hierarchy as an icicle chart where the root spans the
//! full width and children subdivide proportionally.

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

    let src_color = cycle.next_color();
    let docs_color = cycle.next_color();
    let tests_color = cycle.next_color();
    let assets_color = cycle.next_color();

    let root = TreemapNode::new("/project", 0.0)
        .child(
            TreemapNode::new("src", 0.0)
                .color(src_color)
                .child(TreemapNode::new("main.rs", 120.0).color(src_color))
                .child(TreemapNode::new("lib.rs", 350.0).color(src_color))
                .child(
                    TreemapNode::new("widgets", 0.0)
                        .color(src_color)
                        .child(TreemapNode::new("chart.rs", 200.0).color(src_color))
                        .child(TreemapNode::new("table.rs", 150.0).color(src_color))
                        .child(TreemapNode::new("plot.rs", 180.0).color(src_color)),
                )
                .child(
                    TreemapNode::new("utils", 0.0)
                        .color(src_color)
                        .child(TreemapNode::new("math.rs", 90.0).color(src_color))
                        .child(TreemapNode::new("color.rs", 60.0).color(src_color)),
                ),
        )
        .child(
            TreemapNode::new("docs", 0.0)
                .color(docs_color)
                .child(TreemapNode::new("README.md", 80.0).color(docs_color))
                .child(TreemapNode::new("GUIDE.md", 200.0).color(docs_color))
                .child(TreemapNode::new("API.md", 150.0).color(docs_color)),
        )
        .child(
            TreemapNode::new("tests", 0.0)
                .color(tests_color)
                .child(TreemapNode::new("unit.rs", 250.0).color(tests_color))
                .child(TreemapNode::new("integration.rs", 300.0).color(tests_color)),
        )
        .child(
            TreemapNode::new("assets", 0.0)
                .color(assets_color)
                .child(TreemapNode::new("logo.png", 45.0).color(assets_color))
                .child(TreemapNode::new("style.css", 30.0).color(assets_color)),
        );

    let chart = IcicleChart::new(root)
        .orientation(IcicleOrientation::TopDown)
        .max_depth(4)
        .title("File System Hierarchy (q to quit)");

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
