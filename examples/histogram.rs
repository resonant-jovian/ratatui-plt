//! Histogram example: multi-dataset stacked histogram with two overlapping
//! distributions demonstrating the HistMode::Stacked display.

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
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Distribution A: centred at 0, moderate spread
    let dist_a = pseudo_normal(3000, 0.0, 3.0, 0.0);
    // Distribution B: centred at 2, narrower spread
    let dist_b = pseudo_normal(2000, 2.0, 2.0, 100.0);

    let hist = Histogram::new(vec![])
        .dataset(HistDataset::new(
            "Population A (n=3000)",
            dist_a,
            Color::Cyan,
        ))
        .dataset(HistDataset::new(
            "Population B (n=2000)",
            dist_b,
            Color::Magenta,
        ))
        .bins(35)
        .hist_mode(HistMode::Stacked)
        .title("Stacked Histogram: Two Overlapping Distributions (q to quit)")
        .x_axis(Axis::new().label("value").grid(true))
        .y_axis(Axis::new().label("count").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&hist, square_area(frame.area()));
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
