use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::box_plot::BoxData;

/// Generate a deterministic pseudo-data sequence for a group.
fn generate_data(seed: u64, count: usize, center: f64, spread: f64) -> Vec<f64> {
    let mut values = Vec::with_capacity(count);
    let mut state = seed;
    for _ in 0..count {
        // Simple linear congruential generator
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        // Map to [-1, 1] then scale
        let normalized = ((state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0;
        values.push(center + normalized * spread);
    }
    values
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

    let group_a = BoxData::new("Control", generate_data(42, 50, 5.0, 2.0), Color::Cyan);
    let group_b = BoxData::new(
        "Treatment A",
        generate_data(123, 50, 7.5, 3.0),
        Color::Yellow,
    );
    let group_c = BoxData::new(
        "Treatment B",
        generate_data(999, 50, 6.0, 1.5),
        Color::Magenta,
    );

    let plot = BoxPlot::new()
        .box_data(group_a)
        .box_data(group_b)
        .box_data(group_c)
        .title("Distribution Comparison (q to quit)")
        .y_axis(Axis::new().label("Value"));

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(&plot, area);
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
