use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::violin_plot::ViolinData;

/// LCG pseudo-random data generator (deterministic, no rand dependency).
fn generate_data(seed: u64, count: usize, center: f64, spread: f64) -> Vec<f64> {
    let mut values = Vec::with_capacity(count);
    let mut state = seed;
    for _ in 0..count {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let normalized = ((state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0;
        values.push(center + normalized * spread);
    }
    values
}

/// Generate bimodal data by mixing two centers.
fn bimodal_data(seed: u64, count: usize) -> Vec<f64> {
    let mut a = generate_data(seed, count / 2, 3.0, 1.0);
    let b = generate_data(seed + 100, count / 2, 8.0, 1.0);
    a.extend(b);
    a
}

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

    let normal = ViolinData::new("Normal", generate_data(42, 80, 5.0, 2.0), Color::Cyan);
    let bimodal = ViolinData::new("Bimodal", bimodal_data(123, 80), Color::Yellow);
    let skewed = ViolinData::new("Skewed", generate_data(999, 80, 3.0, 1.0), Color::Magenta);
    let wide = ViolinData::new("Wide", generate_data(777, 80, 5.0, 5.0), Color::Green);

    let plot = ViolinPlot::new()
        .dataset(normal)
        .dataset(bimodal)
        .dataset(skewed)
        .dataset(wide)
        .show_box(false)
        .title("Distribution Shapes (q to quit)")
        .y_axis(Axis::new().label("Value"));

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
