//! Pie chart gallery: standard pie, exploded pie, and donut chart.
//!
//! Three panels in a 2-row layout:
//! - Top row (2 side-by-side): Standard pie with percentages | Donut chart
//! - Bottom row (full width): Exploded pie highlighting the top three slices
//!
//! All panels share the same programming language market-share dataset.

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

/// Build pie slices from parallel label/value/color arrays.
fn make_slices(
    labels: &[&str],
    values: &[f64],
    colors: &[Color],
    explode: &[f64],
) -> Vec<PieSlice> {
    labels
        .iter()
        .zip(values.iter())
        .zip(colors.iter())
        .zip(explode.iter())
        .map(|(((&label, &val), &color), &expl)| {
            let mut s = PieSlice::new(label, val).color(color);
            if expl > 0.0 {
                s = s.explode(expl);
            }
            s
        })
        .collect()
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    let theme = Theme::get_default();

    // Shared data: programming language market share (6 slices)
    let labels = ["Python", "JavaScript", "Java", "C/C++", "Rust", "Other"];
    let values = [28.1, 17.4, 15.8, 12.3, 8.5, 17.9];

    // Primary color palette from theme cycle
    let primary_colors = [
        theme.color_cycle.at(0),
        theme.color_cycle.at(1),
        theme.color_cycle.at(2),
        theme.color_cycle.at(3),
        theme.color_cycle.at(4),
        theme.muted,
    ];

    // Reversed palette for the donut chart (visual contrast)
    let donut_colors = [
        theme.color_cycle.at(5),
        theme.color_cycle.at(4),
        theme.color_cycle.at(3),
        theme.color_cycle.at(2),
        theme.color_cycle.at(1),
        theme.muted,
    ];

    let no_explode = [0.0; 6];

    // ── Top-left panel: Standard pie with labels and percentages ───────
    let standard_slices = make_slices(&labels, &values, &primary_colors, &no_explode);
    let standard_pie = PieChart::new()
        .slices(standard_slices)
        .show_labels(true)
        .show_percentages(true)
        .title("Standard Pie (labels + percentages)");

    // ── Top-right panel: Donut chart ───────────────────────────────────
    let donut_slices = make_slices(&labels, &values, &donut_colors, &no_explode);
    let donut_pie = PieChart::new()
        .slices(donut_slices)
        .donut_ratio(0.4)
        .show_labels(true)
        .show_percentages(true)
        .title("Donut Chart (ratio 0.4)");

    // ── Bottom panel: Exploded pie highlighting the top 3 slices ───────
    let explode_offsets = [0.10, 0.06, 0.06, 0.0, 0.0, 0.0];
    let exploded_slices = make_slices(&labels, &values, &primary_colors, &explode_offsets);
    let exploded_pie = PieChart::new()
        .slices(exploded_slices)
        .show_labels(true)
        .show_percentages(true)
        .title("Exploded: Top 3 Languages Highlighted");

    // ── Event loop ─────────────────────────────────────────────────────
    if headless_export(|area, buf| {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);
        let top_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(rows[0]);
        (&standard_pie).render(top_cols[0], buf);
        (&donut_pie).render(top_cols[1], buf);
        (&exploded_pie).render(rows[1], buf);
    })? {
        return Ok(());
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());

            // Split into two rows: top (55%) and bottom (45%)
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(area);

            // Split top row into two equal columns
            let top_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(rows[0]);

            frame.render_widget(&standard_pie, top_cols[0]);
            frame.render_widget(&donut_pie, top_cols[1]);
            frame.render_widget(&exploded_pie, rows[1]);
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
