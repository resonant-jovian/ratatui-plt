//! Bar chart gallery: grouped, stacked, and horizontal stacked modes.
//!
//! Three panels in a 2-row layout:
//! - Top row (2 side-by-side): Grouped bars | Stacked bars with value labels
//! - Bottom row (full width): Horizontal stacked bars with value labels
//!
//! All panels share the same quarterly revenue dataset across three product
//! lines, demonstrating how the same data looks under different bar modes.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::bar_chart::{BarDataset, Orientation};

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

    // Shared data: quarterly revenue across three product lines
    let categories = vec!["Q1", "Q2", "Q3", "Q4"];
    let c1 = cycle.next_color();
    let c2 = cycle.next_color();
    let c3 = cycle.next_color();

    let hardware_vals = vec![120.0, 150.0, 180.0, 140.0];
    let software_vals = vec![90.0, 130.0, 160.0, 175.0];
    let services_vals = vec![60.0, 80.0, 100.0, 120.0];

    // ── Top-left panel: Grouped bars ───────────────────────────────────
    // Each category shows three side-by-side bars, making it easy to
    // compare individual product lines within a quarter.
    let grouped = BarChart::new()
        .categories(categories.clone())
        .dataset(BarDataset::new("Hardware", hardware_vals.clone(), c1))
        .dataset(BarDataset::new("Software", software_vals.clone(), c2))
        .dataset(BarDataset::new("Services", services_vals.clone(), c3))
        .mode(BarMode::Grouped)
        .bar_gap(1)
        .title("Grouped: Per-Quarter Comparison")
        .show_legend(true)
        .legend_position(LegendPosition::TopLeft)
        .reference_line(ReferenceLine::hline_dashed(150.0, theme.muted));

    // ── Top-right panel: Stacked bars with value labels ────────────────
    // Bars are stacked to show total quarterly revenue. Value labels
    // display the stack total above each bar.
    let stacked = BarChart::new()
        .categories(categories.clone())
        .dataset(BarDataset::new("Hardware", hardware_vals.clone(), c1))
        .dataset(BarDataset::new("Software", software_vals.clone(), c2))
        .dataset(BarDataset::new("Services", services_vals.clone(), c3))
        .mode(BarMode::Stacked)
        .bar_gap(1)
        .show_values(true)
        .title("Stacked: Quarterly Totals")
        .show_legend(true)
        .legend_position(LegendPosition::TopLeft);

    // ── Bottom panel: Horizontal stacked bars ──────────────────────────
    // The same stacked data rendered horizontally, which works better
    // when category labels are long or when comparing totals visually.
    let horizontal = BarChart::new()
        .categories(categories.clone())
        .dataset(BarDataset::new("Hardware", hardware_vals.clone(), c1))
        .dataset(BarDataset::new("Software", software_vals.clone(), c2))
        .dataset(BarDataset::new("Services", services_vals.clone(), c3))
        .mode(BarMode::Stacked)
        .orientation(Orientation::Horizontal)
        .bar_gap(1)
        .show_values(true)
        .title("Horizontal Stacked: Revenue by Quarter")
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

    // ── Event loop ─────────────────────────────────────────────────────
    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());

            // Split into two rows: top (55%) for grouped/stacked, bottom (45%) for horizontal
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(area);

            // Split top row into two equal columns
            let top_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(rows[0]);

            frame.render_widget(&grouped, top_cols[0]);
            frame.render_widget(&stacked, top_cols[1]);
            frame.render_widget(&horizontal, rows[1]);
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
