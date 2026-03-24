//! Joint plot example: scatter with marginal histogram/KDE distributions.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use rand::RngExt;
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

    // Generate correlated random data
    let mut rng = rand::rng();
    let n = 500;
    let mut points = Vec::with_capacity(n);
    for _ in 0..n {
        let x: f64 = rng.random_range(-3.0..3.0);
        let noise: f64 = rng.random_range(-1.0..1.0);
        let y = 0.5 * x + noise;
        points.push((x, y));
    }

    let theme = Theme::get_default();

    let series = Series::new("data")
        .data(points)
        .color(theme.primary)
        .marker(MarkerShape::Dot);

    let plot = JointPlot::new()
        .series(series)
        .marginal_x(MarginalType::Histogram)
        .marginal_y(MarginalType::Kde)
        .marginal_bins(25)
        .title("Joint Plot: Scatter + Marginals")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"))
        .show_legend(false);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&plot, square_area(frame.area()));
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
