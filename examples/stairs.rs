//! Stairs plot example: sine and cosine step functions.
//!
//! Demonstrates the StairsPlot widget with two simple mathematical
//! functions rendered as step plots with filled baseline.

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

    // 30 bins covering 0 to 2*pi
    let n_bins = 30;
    let x_min = 0.0;
    let x_max = 2.0 * std::f64::consts::PI;
    let bin_width = (x_max - x_min) / n_bins as f64;

    let edges: Vec<f64> = (0..=n_bins).map(|i| x_min + i as f64 * bin_width).collect();

    // sin(x) evaluated at bin centers
    let sin_values: Vec<f64> = (0..n_bins)
        .map(|i| {
            let x_center = edges[i] + bin_width / 2.0;
            x_center.sin()
        })
        .collect();

    // cos(x) evaluated at bin centers
    let cos_values: Vec<f64> = (0..n_bins)
        .map(|i| {
            let x_center = edges[i] + bin_width / 2.0;
            x_center.cos()
        })
        .collect();

    let sin_ds = StairsDataset::new("sin(x)", edges.clone(), sin_values, Color::Cyan);
    let cos_ds = StairsDataset::new("cos(x)", edges, cos_values, Color::Yellow);

    let plot = StairsPlot::new()
        .dataset(sin_ds)
        .dataset(cos_ds)
        .baseline(0.0)
        .title("Stairs Plot: sin(x) and cos(x) (q to quit)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&plot, frame.area());
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
