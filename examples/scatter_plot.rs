//! Scatter plot gallery: color-mapped bubble chart and multi-series comparison.
//!
//! Two side-by-side panels:
//! - Left: Radial intensity point cloud with per-point color mapping and bubble sizes
//! - Right: Multi-series scatter with different marker shapes and a LOWESS trendline

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

/// Simple deterministic pseudo-normal generator using sum of sines (CLT-like).
fn pseudo_normal(seed: f64) -> f64 {
    let t = seed * 0.1;
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
    sum / 6.0
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let theme = Theme::get_default();
    let mut cycle = theme.color_cycle.clone();

    // ── Left panel: Color-mapped bubble scatter ────────────────────────
    let n = 2000;
    let mut points = Vec::with_capacity(n);
    let mut color_vals = Vec::with_capacity(n);
    let mut size_vals = Vec::with_capacity(n);

    for i in 0..n {
        let x = pseudo_normal(i as f64) * 3.0;
        let y = pseudo_normal(i as f64 + 500.0) * 3.0;
        let r2 = x * x + y * y;
        let intensity = (-r2 / 6.0).exp();
        points.push((x, y));
        color_vals.push(intensity);
        size_vals.push(intensity);
    }

    let bubble_series = Series::new("intensity")
        .data(points)
        .marker(MarkerShape::FilledCircle);

    let left_plot = ScatterPlot::new()
        .series(bubble_series)
        .color_values(color_vals)
        .size_values(size_vals)
        .size_range(1.0, 3.0)
        .colormap(Plasma)
        .title("Radial Intensity Bubble Chart")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .aspect_ratio(AspectRatio::Equal)
        .show_legend(false);

    // ── Right panel: Multi-series scatter ──────────────────────────────
    let c1 = cycle.next_color();
    let c2 = cycle.next_color();
    let c3 = cycle.next_color();

    // Cluster A: tight group at lower-left
    let cluster_a: Vec<(f64, f64)> = (0..120)
        .map(|i| {
            let x = 2.0 + pseudo_normal(i as f64 * 1.1) * 0.8;
            let y = 3.0 + pseudo_normal(i as f64 * 1.3 + 200.0) * 0.6;
            (x, y)
        })
        .collect();

    // Cluster B: spread group at upper-right
    let cluster_b: Vec<(f64, f64)> = (0..120)
        .map(|i| {
            let x = 6.0 + pseudo_normal(i as f64 * 0.9 + 300.0) * 1.2;
            let y = 7.0 + pseudo_normal(i as f64 * 1.1 + 400.0) * 1.0;
            (x, y)
        })
        .collect();

    // Cluster C: elongated group at middle
    let cluster_c: Vec<(f64, f64)> = (0..120)
        .map(|i| {
            let x = 4.0 + pseudo_normal(i as f64 * 1.2 + 600.0) * 1.5;
            let y = 5.0 + pseudo_normal(i as f64 * 0.8 + 700.0) * 0.4;
            (x, y)
        })
        .collect();

    let series_a = Series::new("Cluster A")
        .data(cluster_a)
        .color(c1)
        .marker(MarkerShape::Circle);
    let series_b = Series::new("Cluster B")
        .data(cluster_b)
        .color(c2)
        .marker(MarkerShape::Diamond);
    let series_c = Series::new("Cluster C")
        .data(cluster_c)
        .color(c3)
        .marker(MarkerShape::Triangle);

    let right_plot = ScatterPlot::new()
        .series(series_a)
        .series(series_b)
        .series(series_c)
        .title("Multi-Series Cluster Comparison")
        .x_axis(Axis::new().label("Feature 1").grid(true))
        .y_axis(Axis::new().label("Feature 2").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopLeft);

    // ── Event loop ─────────────────────────────────────────────────────
    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(area);

            frame.render_widget(&left_plot, cols[0]);
            frame.render_widget(&right_plot, cols[1]);
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
