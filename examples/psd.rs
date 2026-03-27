//! Power spectral density example.
//!
//! Shows the PSD of a composite signal containing a 440 Hz tone and a 1000 Hz
//! tone, computed via Welch's method.

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

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let theme = Theme::get_default();
    let mut cycle = theme.color_cycle.clone();

    // Generate a composite signal: 440 Hz + 1000 Hz + noise-like component
    let sample_rate = 8000.0;
    let n_samples = 2048;
    let signal: Vec<f64> = (0..n_samples)
        .map(|i| {
            let t = i as f64 / sample_rate;
            let tone_a = 1.0 * (2.0 * std::f64::consts::PI * 440.0 * t).sin();
            let tone_b = 0.5 * (2.0 * std::f64::consts::PI * 1000.0 * t).sin();
            // Deterministic pseudo-noise based on sample index
            let noise = 0.2 * ((i as f64 * 0.1).sin() * (i as f64 * 0.037).cos());
            tone_a + tone_b + noise
        })
        .collect();

    let psd_series = psd(&signal, sample_rate).color(cycle.next_color());

    let chart = PsdPlot::new()
        .series(psd_series)
        .x_axis(Axis::new().label("Frequency (Hz)"))
        .y_axis(Axis::new().label("Power (dB)"))
        .title("Power Spectral Density (q to quit)");

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&chart, square_area(frame.area()));
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
