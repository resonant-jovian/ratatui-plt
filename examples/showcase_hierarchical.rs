//! Showcase: Hierarchical data visualization widgets.
//!
//! 6 plots in a 2x3 mosaic grid (ABC / DEF):
//! A) Treemap, B) Sunburst, C) IcicleChart,
//! D) SankeyDiagram, E) Dendrogram, F) ClusterMap.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::clustermap::ClusterMap;
use ratatui_plt::widgets::dendrogram::{DendroLink, Dendrogram};
use ratatui_plt::widgets::icicle::IcicleChart;
use ratatui_plt::widgets::sankey::{SankeyDiagram, SankeyFlow, SankeyNode};
use ratatui_plt::widgets::sunburst::{Sunburst, SunburstNode};
use ratatui_plt::widgets::treemap::{Treemap, TreemapNode};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(Theme::light());
    let theme = Theme::get_default();
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let c0 = theme.color_cycle.at(0);
    let c1 = theme.color_cycle.at(1);
    let c2 = theme.color_cycle.at(2);
    let c3 = theme.color_cycle.at(3);
    let c4 = theme.color_cycle.at(4);
    let c5 = theme.color_cycle.at(5);

    // ---- Panel A: Treemap — budget allocation ----
    let root = TreemapNode::new("Budget", 0.0)
        .child(
            TreemapNode::new("Engineering", 0.0)
                .color(c0)
                .child(TreemapNode::new("Backend", 25.0).color(c0))
                .child(TreemapNode::new("Frontend", 20.0).color(c1))
                .child(TreemapNode::new("Infra", 15.0).color(c2)),
        )
        .child(
            TreemapNode::new("Marketing", 0.0)
                .color(c3)
                .child(TreemapNode::new("Digital", 18.0).color(c3))
                .child(TreemapNode::new("Events", 8.0).color(c4)),
        )
        .child(TreemapNode::new("Sales", 12.0).color(c5))
        .child(TreemapNode::new("HR", 7.0).color(theme.color_cycle.at(6)));

    let treemap = Treemap::new(root).title("Budget Allocation");

    // ---- Panel B: Sunburst — organizational hierarchy ----
    let org = SunburstNode::new("Org", 0.0)
        .child(
            SunburstNode::new("VP Eng", 0.0)
                .color(c0)
                .child(
                    SunburstNode::new("Dir BE", 0.0)
                        .color(c0)
                        .child(SunburstNode::new("Team A", 8.0).color(c0))
                        .child(SunburstNode::new("Team B", 6.0).color(c1)),
                )
                .child(SunburstNode::new("Dir FE", 5.0).color(c2)),
        )
        .child(
            SunburstNode::new("VP Sales", 0.0)
                .color(c3)
                .child(SunburstNode::new("East", 7.0).color(c3))
                .child(SunburstNode::new("West", 9.0).color(c4)),
        )
        .child(SunburstNode::new("VP Ops", 4.0).color(c5));

    let sunburst = Sunburst::new(org).title("Organization");

    // ---- Panel C: IcicleChart — file system ----
    let fs_root = TreemapNode::new("/project", 0.0)
        .child(
            TreemapNode::new("src", 0.0)
                .color(c0)
                .child(TreemapNode::new("main.rs", 12.0).color(c0))
                .child(TreemapNode::new("lib.rs", 8.0).color(c1))
                .child(TreemapNode::new("utils.rs", 5.0).color(c2)),
        )
        .child(
            TreemapNode::new("docs", 0.0)
                .color(c3)
                .child(TreemapNode::new("README", 3.0).color(c3))
                .child(TreemapNode::new("API.md", 4.0).color(c4)),
        )
        .child(
            TreemapNode::new("tests", 0.0)
                .color(c5)
                .child(TreemapNode::new("unit.rs", 6.0).color(c5))
                .child(TreemapNode::new("integ.rs", 4.0).color(theme.color_cycle.at(6))),
        )
        .child(TreemapNode::new("assets", 2.0).color(theme.color_cycle.at(7)));

    let icicle = IcicleChart::new(fs_root).title("File System");

    // ---- Panel D: SankeyDiagram — energy flow ----
    // Nodes: 0=Solar, 1=Wind, 2=Gas, 3=Grid, 4=Battery, 5=Heating, 6=Transport, 7=Industry
    let sankey = SankeyDiagram::new()
        .node(SankeyNode::new("Solar").color(c0))
        .node(SankeyNode::new("Wind").color(c1))
        .node(SankeyNode::new("Gas").color(c2))
        .node(SankeyNode::new("Grid").color(c3))
        .node(SankeyNode::new("Battery").color(c4))
        .node(SankeyNode::new("Heating").color(c5))
        .node(SankeyNode::new("Transport").color(theme.color_cycle.at(6)))
        .node(SankeyNode::new("Industry").color(theme.color_cycle.at(7)))
        .flow(SankeyFlow::new(0, 3, 30.0))
        .flow(SankeyFlow::new(0, 4, 10.0))
        .flow(SankeyFlow::new(1, 3, 25.0))
        .flow(SankeyFlow::new(2, 3, 20.0))
        .flow(SankeyFlow::new(3, 5, 25.0))
        .flow(SankeyFlow::new(3, 6, 30.0))
        .flow(SankeyFlow::new(3, 7, 20.0))
        .flow(SankeyFlow::new(4, 6, 10.0))
        .title("Energy Flow");

    // ---- Panel E: Dendrogram — clustering tree ----
    // 5 leaves: A, B, C, D, E (indices 0..5)
    // 4 links creating merged clusters 5, 6, 7, 8
    let dendro_links = vec![
        DendroLink::new(0, 1, 1.0), // merge A+B → cluster 5
        DendroLink::new(2, 3, 1.5), // merge C+D → cluster 6
        DendroLink::new(5, 4, 2.5), // merge (A+B)+E → cluster 7
        DendroLink::new(6, 7, 4.0), // merge (C+D)+(A+B+E) → cluster 8
    ];
    let dendro_labels: Vec<String> = ["Gene A", "Gene B", "Gene C", "Gene D", "Gene E"]
        .iter()
        .map(|s| (*s).into())
        .collect();

    let dendrogram = Dendrogram::new(dendro_links, dendro_labels)
        .color_threshold(2.0)
        .title("Clustering");

    // ---- Panel F: ClusterMap — 8x8 correlation matrix ----
    let size = 8;
    let x_vals: Vec<f64> = (0..size).map(|i| i as f64).collect();
    let y_vals: Vec<f64> = (0..size).map(|i| i as f64).collect();

    // Build a symmetric correlation matrix using a deterministic pattern
    let mut values = vec![vec![0.0; size]; size];
    for (i, row) in values.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            if i == j {
                *cell = 1.0;
            } else {
                // Distance-based correlation: closer indices are more correlated
                let dist = (i as f64 - j as f64).abs();
                *cell = (1.0 - dist / size as f64).max(0.05);
            }
        }
    }

    let grid = GridData::new(x_vals, y_vals, values);

    // Row and column dendrograms (7 links for 8 leaves)
    let cluster_links = vec![
        DendroLink::new(0, 1, 0.3),
        DendroLink::new(2, 3, 0.4),
        DendroLink::new(4, 5, 0.3),
        DendroLink::new(6, 7, 0.5),
        DendroLink::new(8, 9, 0.7),
        DendroLink::new(10, 11, 0.8),
        DendroLink::new(12, 13, 1.2),
    ];

    let clustermap = ClusterMap::new(grid)
        .row_links(cluster_links.clone())
        .col_links(cluster_links)
        .colormap(Viridis)
        .show_colorbar(false)
        .title("Cluster Map");

    // ---- Assemble 2x3 mosaic: ABC / DEF ----
    let panel = MultiPanel::from_mosaic("ABC\nDEF")
        .gap(1)
        .suptitle("Hierarchical Plots Showcase (q to quit)")
        .mosaic_panel('A', move |area: Rect, buf: &mut Buffer| {
            (&treemap).render(area, buf);
        })
        .mosaic_panel('B', move |area: Rect, buf: &mut Buffer| {
            (&sunburst).render(area, buf);
        })
        .mosaic_panel('C', move |area: Rect, buf: &mut Buffer| {
            (&icicle).render(area, buf);
        })
        .mosaic_panel('D', move |area: Rect, buf: &mut Buffer| {
            (&sankey).render(area, buf);
        })
        .mosaic_panel('E', move |area: Rect, buf: &mut Buffer| {
            (&dendrogram).render(area, buf);
        })
        .mosaic_panel('F', move |area: Rect, buf: &mut Buffer| {
            (&clustermap).render(area, buf);
        });

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
