//! Beeswarm example: Response times by browser.
//!
//! Four browser groups with different response time distributions,
//! generated deterministically via a linear congruential generator.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::swarm::SwarmGroup;

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

/// Simple LCG pseudo-random number generator returning values in [0, 1).
fn lcg_next(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 33) as f64) / (u32::MAX as f64)
}

/// Generate pseudo-normal samples using sum of 6 LCG draws (CLT approximation).
fn lcg_normal(state: &mut u64, mean: f64, stddev: f64) -> f64 {
    let sum: f64 = (0..6).map(|_| lcg_next(state)).sum();
    // sum of 6 uniform [0,1) has mean 3, stddev sqrt(6/12) = sqrt(0.5) ~ 0.707
    mean + stddev * (sum - 3.0) / std::f64::consts::FRAC_1_SQRT_2
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Chrome: low latency, tight distribution (mean 80ms, stddev 12)
    let mut state: u64 = 111;
    let chrome: Vec<f64> = (0..80)
        .map(|_| lcg_normal(&mut state, 80.0, 12.0))
        .collect();

    // Firefox: moderate latency (mean 110ms, stddev 18)
    let mut state: u64 = 222;
    let firefox: Vec<f64> = (0..75)
        .map(|_| lcg_normal(&mut state, 110.0, 18.0))
        .collect();

    // Safari: bimodal-ish via two clusters (mean 95 + some at 140)
    let mut state: u64 = 333;
    let safari: Vec<f64> = (0..70)
        .map(|i| {
            if i % 4 == 0 {
                lcg_normal(&mut state, 140.0, 10.0)
            } else {
                lcg_normal(&mut state, 95.0, 14.0)
            }
        })
        .collect();

    // Edge: higher latency, wider spread (mean 130ms, stddev 25)
    let mut state: u64 = 444;
    let edge: Vec<f64> = (0..60)
        .map(|_| lcg_normal(&mut state, 130.0, 25.0))
        .collect();

    let plot = SwarmPlot::new()
        .group(SwarmGroup::new("Chrome", chrome, Color::Rgb(80, 200, 255)))
        .group(SwarmGroup::new(
            "Firefox",
            firefox,
            Color::Rgb(255, 160, 40),
        ))
        .group(SwarmGroup::new("Safari", safari, Color::Rgb(220, 100, 255)))
        .group(SwarmGroup::new("Edge", edge, Color::Rgb(100, 220, 100)))
        .title("Beeswarm: Response Times by Browser (q to quit)")
        .point_size(2)
        .y_axis(
            Axis::new()
                .label("Response Time (ms)")
                .label_position(LabelPosition::End)
                .grid(true),
        );

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
