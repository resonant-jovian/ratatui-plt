//! Stem plot example: discrete event sequence with 15 events at varying positions and heights.

use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use ratatui_sim::prelude::*;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // 15 discrete events at pseudo-random positions with varying heights.
    // Uses a simple deterministic pattern so no extra RNG crate is needed.
    let events: Vec<(f64, f64)> = (0..15)
        .map(|i| {
            let x = i as f64 * 1.3 + 0.5;
            // Heights from a mix of sin + sawtooth for variety
            let y = 2.0 * (i as f64 * 0.7).sin() + 1.5 * ((i as f64 * 0.4).cos()).abs() + 0.5;
            (x, y)
        })
        .collect();

    let plot = StemPlot::new(events)
        .baseline(0.0)
        .color(Color::Cyan)
        .marker(MarkerShape::FilledCircle)
        .title("Discrete Event Sequence (15 events)")
        .x_axis(Axis::new().label("time"))
        .y_axis(Axis::new().label("amplitude"));

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&plot, frame.area());
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press
                && (key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
            {
                break;
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
