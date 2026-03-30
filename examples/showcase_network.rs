//! Showcase: Network and parallel coordinate visualization widgets.
//!
//! 3 plots in a 1x3 mosaic grid (ABC):
//! A) NetworkGraph, B) ParallelCoords, C) ParallelCategories.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::network::{GraphEdge, GraphLayout, GraphNode, NetworkPlot};
use ratatui_plt::widgets::parallel_categories::{
    CategoricalDimension, CategoricalRecord, ParallelCategories,
};
use ratatui_plt::widgets::parallel_coords::{ParallelAxis, ParallelCoords, ParallelRecord};

/// Simple deterministic LCG pseudo-random number generator.
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*seed >> 33) as f64 / (1u64 << 31) as f64
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(Theme::light());
    let theme = Theme::get_default();
    let c0 = theme.color_cycle.at(0);
    let c1 = theme.color_cycle.at(1);
    let c2 = theme.color_cycle.at(2);
    let c3 = theme.color_cycle.at(3);
    let c4 = theme.color_cycle.at(4);

    // ---- Panel A: NetworkGraph — 8-node social network ----
    let node_names = [
        "Alice", "Bob", "Carol", "Dave", "Eve", "Frank", "Grace", "Hank",
    ];
    let node_colors = [c0, c1, c2, c3, c4, c0, c1, c2];

    let mut network = NetworkPlot::new();
    for (i, &name) in node_names.iter().enumerate() {
        network = network.node(GraphNode::new(name).color(node_colors[i]));
    }

    // Create edges for a social network graph
    let edges = [
        (0, 1, 1.0), // Alice-Bob
        (0, 2, 1.0), // Alice-Carol
        (1, 3, 1.0), // Bob-Dave
        (2, 3, 1.0), // Carol-Dave
        (2, 4, 1.0), // Carol-Eve
        (3, 5, 1.0), // Dave-Frank
        (4, 5, 1.0), // Eve-Frank
        (5, 6, 1.0), // Frank-Grace
        (6, 7, 1.0), // Grace-Hank
        (0, 7, 0.5), // Alice-Hank (weak tie)
        (1, 4, 0.5), // Bob-Eve (weak tie)
        (3, 6, 0.5), // Dave-Grace (weak tie)
    ];

    for &(s, t, w) in &edges {
        network = network.edge(GraphEdge::new(s, t, w));
    }

    let network_plot = network
        .layout(GraphLayout::ForceDirected)
        .title("Social Network");

    // ---- Panel B: ParallelCoords — 5 axes, 15 records ----
    let axes = vec![
        ParallelAxis::new("CPU", 0.0, 100.0),
        ParallelAxis::new("Memory", 0.0, 64.0),
        ParallelAxis::new("Disk", 0.0, 1000.0),
        ParallelAxis::new("Network", 0.0, 10.0),
        ParallelAxis::new("Latency", 0.0, 200.0),
    ];

    let mut seed = 42u64;
    let record_colors = [c0, c1, c2, c3, c4];
    let mut records = Vec::new();
    for i in 0..15 {
        let cluster = i % 3;
        let base = match cluster {
            0 => [80.0, 48.0, 800.0, 8.0, 20.0],
            1 => [30.0, 16.0, 200.0, 2.0, 150.0],
            _ => [50.0, 32.0, 500.0, 5.0, 80.0],
        };
        let values: Vec<f64> = base
            .iter()
            .map(|&b| {
                let noise = (lcg(&mut seed) - 0.5) * b * 0.4;
                (b + noise).max(0.0)
            })
            .collect();
        let color = record_colors[cluster];
        let name = format!("Server {}", i + 1);
        records.push(ParallelRecord::new(values).color(color).name(name));
    }

    let parallel = ParallelCoords::new()
        .axes(axes)
        .records(records)
        .show_legend(false)
        .title("Server Metrics");

    // ---- Panel C: ParallelCategories — Department/Level/Mode ----
    let categories = ParallelCategories::new()
        .dimension(CategoricalDimension::new(
            "Department",
            vec!["Eng", "Sales", "Ops"],
        ))
        .dimension(CategoricalDimension::new(
            "Level",
            vec!["Junior", "Senior", "Lead"],
        ))
        .dimension(CategoricalDimension::new(
            "Mode",
            vec!["Remote", "Hybrid", "Office"],
        ))
        .record(CategoricalRecord::new(vec![0, 0, 0]).count(25))
        .record(CategoricalRecord::new(vec![0, 1, 0]).count(15))
        .record(CategoricalRecord::new(vec![0, 1, 1]).count(10))
        .record(CategoricalRecord::new(vec![0, 2, 0]).count(8))
        .record(CategoricalRecord::new(vec![1, 0, 2]).count(20))
        .record(CategoricalRecord::new(vec![1, 1, 1]).count(12))
        .record(CategoricalRecord::new(vec![2, 0, 1]).count(18))
        .record(CategoricalRecord::new(vec![2, 1, 2]).count(14))
        .title("Workforce Distribution");

    // ---- Assemble 1x3 mosaic: ABC ----
    let panel = MultiPanel::from_mosaic("ABC")
        .gap(1)
        .suptitle("Network & Parallel Plots Showcase (q to quit)")
        .mosaic_panel('A', move |area: Rect, buf: &mut Buffer| {
            (&network_plot).render(area, buf);
        })
        .mosaic_panel('B', move |area: Rect, buf: &mut Buffer| {
            (&parallel).render(area, buf);
        })
        .mosaic_panel('C', move |area: Rect, buf: &mut Buffer| {
            (&categories).render(area, buf);
        });

    if headless_export(|area, buf| (&panel).render(area, buf))? {
        return Ok(());
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&panel, square_area(frame.area()));
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
