//! Letter-value (boxen) plot example: Server latencies.
//!
//! Three server groups with different latency profiles (low, medium, high tail),
//! generated deterministically via LCG. 200+ points per group for deep quantile levels.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::boxen::BoxenGroup;

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
    mean + stddev * (sum - 3.0) / std::f64::consts::FRAC_1_SQRT_2
}

/// Generate pseudo-exponential tail values: -ln(u) * scale.
fn lcg_exponential(state: &mut u64, scale: f64) -> f64 {
    let u = lcg_next(state).max(0.001);
    -u.ln() * scale
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Server A: low latency, symmetric distribution (mean 20ms, stddev 5)
    let mut state: u64 = 10001;
    let server_a: Vec<f64> = (0..250)
        .map(|_| lcg_normal(&mut state, 20.0, 5.0))
        .collect();

    // Server B: moderate latency with some right skew
    // Base normal + occasional exponential spikes
    let mut state: u64 = 20002;
    let server_b: Vec<f64> = (0..250)
        .map(|_| {
            let base = lcg_normal(&mut state, 45.0, 10.0);
            let spike = lcg_exponential(&mut state, 8.0);
            // Mix: 80% base, 20% with extra spike
            if lcg_next(&mut state) < 0.2 {
                base + spike
            } else {
                base
            }
        })
        .collect();

    // Server C: high tail latency (heavy-tailed distribution)
    // Mostly normal, but with exponential tail events
    let mut state: u64 = 30003;
    let server_c: Vec<f64> = (0..250)
        .map(|_| {
            let base = lcg_normal(&mut state, 80.0, 15.0);
            let spike = lcg_exponential(&mut state, 30.0);
            if lcg_next(&mut state) < 0.3 {
                base + spike
            } else {
                base
            }
        })
        .collect();

    let theme = Theme::get_default();
    let mut cycle = theme.color_cycle.clone();

    let plot = BoxenPlot::new()
        .group(BoxenGroup::new("Server A", server_a, cycle.next_color()))
        .group(BoxenGroup::new("Server B", server_b, cycle.next_color()))
        .group(BoxenGroup::new("Server C", server_c, cycle.next_color()))
        .title("Letter-Value Plot: Server Latencies (q to quit)")
        .y_axis(
            Axis::new()
                .label("Latency (ms)")
                .grid(true)
                .label_position(LabelPosition::End),
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
