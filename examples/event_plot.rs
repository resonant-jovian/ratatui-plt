use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;

/// Simple LCG pseudo-random number generator (deterministic, no rand dependency).
fn lcg_events(seed: u64, count: usize, range: f64) -> Vec<f64> {
    let mut state = seed;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let normalized = (state >> 33) as f64 / u32::MAX as f64;
        values.push(normalized * range);
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    values
}

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

    let colors = [
        Color::Cyan,
        Color::Yellow,
        Color::Magenta,
        Color::Green,
        Color::Red,
    ];
    let groups: Vec<EventGroup> = (0..5)
        .map(|i| {
            let spikes = lcg_events(42 + i * 17, 10 + (i as usize) * 2, 200.0);
            EventGroup::new(format!("Neuron {}", i + 1), spikes).color(colors[i as usize])
        })
        .collect();

    let plot = EventPlot::new()
        .groups(groups)
        .title("Neural Spike Raster (q to quit)")
        .x_axis(Axis::new().label("Time (ms)"));

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
