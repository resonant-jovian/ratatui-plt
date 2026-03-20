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

    // --- Energy tracking line plot data ---
    let energy_data: Vec<(f64, f64)> = (0..200)
        .map(|i| {
            let t = i as f64 * 0.05;
            let energy = 10.0 * (-0.03 * t).exp() * (2.0 * t).cos().powi(2) + 0.5;
            (t, energy)
        })
        .collect();

    let energy_plot = LinePlot::new()
        .series(
            Series::new("Kinetic Energy")
                .data(energy_data)
                .color(Color::Cyan),
        )
        .title("Energy vs Time")
        .x_axis(Axis::new().label("t"))
        .y_axis(Axis::new().label("E"))
        .show_legend(true);

    // --- Density heatmap data ---
    let density = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 300, 300, |x, y| {
        let r2 = x * x + y * y;
        (-r2 / 2.0).exp() + 0.3 * (-(((x - 1.0).powi(2) + (y - 1.0).powi(2)) / 0.5)).exp()
    });

    let heatmap = Heatmap::new(density)
        .colormap(Inferno)
        .title("Density Field")
        .show_colorbar(true)
        .aspect_ratio(AspectRatio::Equal);

    // --- Velocity distribution histogram data ---
    // Generate a deterministic pseudo-Gaussian-like distribution using simple arithmetic
    let velocity_data: Vec<f64> = (0..5000)
        .map(|i| {
            // Sum of several deterministic oscillations to approximate a bell curve
            let x = i as f64;

            (x * 0.1).sin()
                + (x * 0.037).cos()
                + (x * 0.071).sin()
                + (x * 0.023).cos()
                + (x * 0.113).sin()
                + (x * 0.059).cos()
        })
        .collect();

    let histogram = Histogram::new(velocity_data)
        .bins(25)
        .color(Color::Green)
        .title("Velocity Distribution");

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            // 2-row layout: top row for line plot, bottom row split into heatmap + histogram
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                .split(area);

            let bottom_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(rows[1]);

            frame.render_widget(&energy_plot, rows[0]);
            frame.render_widget(&heatmap, bottom_cols[0]);
            frame.render_widget(&histogram, bottom_cols[1]);
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
