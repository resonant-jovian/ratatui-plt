//! Histogram gallery: overlapping, stacked, cumulative, and side-by-side modes.
//!
//! Four panels in a 2x2 grid showing the same pair of distributions:
//! - Top-left: Layered mode (overlapping) with KDE overlay
//! - Top-right: Stacked mode showing combined totals
//! - Bottom-left: Cumulative distribution view
//! - Bottom-right: Side-by-side bars for direct bin comparison
//!
//! The data uses a deterministic pseudo-normal generator for reproducibility.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::histogram::{HistDataset, HistMode};

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

/// Simple deterministic pseudo-normal generator using sum of sines (CLT-like).
fn pseudo_normal(n: usize, center: f64, spread: f64, seed: f64) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let t = (i as f64 + seed) * 0.1;
            let sum = (t * 1.0).sin()
                + (t * std::f64::consts::SQRT_2).sin()
                + (t * std::f64::consts::PI).sin()
                + (t * std::f64::consts::E).sin()
                + (t * 2.2360679).sin()
                + (t * 3.3166248).sin()
                + (t * 0.577).cos()
                + (t * 1.732).cos()
                + (t * 2.449).cos()
                + (t * 0.317).sin()
                + (t * 4.123).cos()
                + (t * 5.099).sin();
            center + sum * spread / 6.0
        })
        .collect()
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    let theme = Theme::get_default();
    let mut cycle = theme.color_cycle.clone();
    let c1 = cycle.next_color();
    let c2 = cycle.next_color();

    // Generate two overlapping distributions with different shapes
    let dist_a = pseudo_normal(3000, 0.0, 3.0, 0.0);
    let dist_b = pseudo_normal(2000, 2.5, 2.0, 100.0);

    let bins = 30;

    // ── Top-left: Layered mode with KDE overlay ────────────────────────
    let layered = Histogram::new(vec![])
        .dataset(HistDataset::new("Pop A (n=3k)", dist_a.clone(), c1))
        .dataset(HistDataset::new("Pop B (n=2k)", dist_b.clone(), c2))
        .bins(bins)
        .hist_mode(HistMode::Layered)
        .show_kde(true)
        .title("Layered + KDE")
        .x_axis(Axis::new().label("Value").grid(true))
        .y_axis(Axis::new().label("Count").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

    // ── Top-right: Stacked mode ────────────────────────────────────────
    let stacked = Histogram::new(vec![])
        .dataset(HistDataset::new("Pop A (n=3k)", dist_a.clone(), c1))
        .dataset(HistDataset::new("Pop B (n=2k)", dist_b.clone(), c2))
        .bins(bins)
        .hist_mode(HistMode::Stacked)
        .title("Stacked")
        .x_axis(Axis::new().label("Value").grid(true))
        .y_axis(Axis::new().label("Count").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

    // ── Bottom-left: Cumulative distribution ───────────────────────────
    let cumulative = Histogram::new(vec![])
        .dataset(HistDataset::new("Pop A (n=3k)", dist_a.clone(), c1))
        .dataset(HistDataset::new("Pop B (n=2k)", dist_b.clone(), c2))
        .bins(bins)
        .hist_mode(HistMode::Layered)
        .cumulative(true)
        .title("Cumulative Layered")
        .x_axis(Axis::new().label("Value").grid(true))
        .y_axis(
            Axis::new()
                .label("Cumulative")
                .label_position(LabelPosition::End)
                .grid(true),
        )
        .show_legend(true)
        .legend_position(LegendPosition::TopLeft);

    // ── Bottom-right: Side-by-side bars ────────────────────────────────
    let side_by_side = Histogram::new(vec![])
        .dataset(HistDataset::new("Pop A (n=3k)", dist_a, c1))
        .dataset(HistDataset::new("Pop B (n=2k)", dist_b, c2))
        .bins(bins)
        .hist_mode(HistMode::SideBySide)
        .title("Side-by-Side")
        .x_axis(Axis::new().label("Value").grid(true))
        .y_axis(Axis::new().label("Count").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

    // ── Event loop ─────────────────────────────────────────────────────
    if headless_export(|area, buf| {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(area);
        let top_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(rows[0]);
        let bot_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(rows[1]);
        (&layered).render(top_cols[0], buf);
        (&stacked).render(top_cols[1], buf);
        (&cumulative).render(bot_cols[0], buf);
        (&side_by_side).render(bot_cols[1], buf);
    })? {
        return Ok(());
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());

            // 2x2 grid layout
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(area);

            let top_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(rows[0]);

            let bot_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(rows[1]);

            frame.render_widget(&layered, top_cols[0]);
            frame.render_widget(&stacked, top_cols[1]);
            frame.render_widget(&cumulative, bot_cols[0]);
            frame.render_widget(&side_by_side, bot_cols[1]);
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
