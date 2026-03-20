//! ECDF example: Gaussian vs Exponential distribution comparison.
//!
//! Two pseudo-distributions generated deterministically:
//! - Pseudo-Gaussian: sum of 6 sine waves at incommensurate frequencies (CLT-like)
//! - Pseudo-Exponential: reciprocal transform of uniform-like sequence

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

    // Generate pseudo-Gaussian data using sum of sines at incommensurate frequencies.
    // By the Central Limit Theorem, summing independent oscillations yields an
    // approximately Gaussian distribution centred at 0 with spread ~1.2.
    let n = 300;
    let gaussian_data: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 * 0.1;
            let sum = (t * 1.0).sin()
                + (t * std::f64::consts::SQRT_2).sin()
                + (t * std::f64::consts::PI).sin()
                + (t * std::f64::consts::E).sin()
                + (t * 2.2360679).sin()  // sqrt(5)
                + (t * 3.3166248).sin(); // sqrt(11)
            sum * 0.5 // scale to roughly [-3, 3]
        })
        .collect();

    // Generate pseudo-Exponential data via reciprocal transform of an LCG.
    // Uses -ln(u) where u is a pseudo-uniform sequence from a linear congruential generator.
    let exponential_data: Vec<f64> = {
        let mut state: u64 = 12345;
        (0..n)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let u = ((state >> 33) as f64) / (u32::MAX as f64);
                // Clamp away from 0 to avoid infinity
                let u_safe = u.max(0.001);
                -u_safe.ln() * 1.5 // rate parameter ~1/1.5
            })
            .collect()
    };

    let gaussian_ds = EcdfDataset::new("Gaussian (sum of sines)", gaussian_data, Color::Cyan);
    let exponential_ds = EcdfDataset::new(
        "Exponential (LCG reciprocal)",
        exponential_data,
        Color::Yellow,
    );

    let plot = EcdfPlot::new()
        .dataset(gaussian_ds)
        .dataset(exponential_ds)
        .title("ECDF: Gaussian vs Exponential (q to quit)")
        .x_axis(Axis::new().label("Value").grid(true))
        .y_axis(Axis::new().label("F(x)").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::BottomRight);

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
