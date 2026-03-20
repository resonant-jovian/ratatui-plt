//! Strip plot example: Gene Expression by Cell Type.
//!
//! Simulates single-cell gene expression measurements (log2 TPM) for a
//! marker gene across 4 immune cell types, each with distinct expression
//! distributions generated via a deterministic linear congruential generator.

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

/// Deterministic pseudo-random data using a linear congruential generator.
/// Returns values centred at `center` with spread `spread`.
fn lcg_data(seed: u64, count: usize, center: f64, spread: f64) -> Vec<f64> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            // Sum two draws for a more bell-shaped distribution (Irwin-Hall n=2)
            let u1 = ((state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0;
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let u2 = ((state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0;
            center + (u1 + u2) * 0.5 * spread
        })
        .collect()
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Four immune cell types with different expression patterns for gene CD3E:
    //   T cells:        high expression (center 8.5, moderate spread)
    //   NK cells:       moderate expression (center 5.0, tight)
    //   B cells:        low expression (center 2.0, tight)
    //   Macrophages:    very low / off (center 0.8, wide due to noise)
    let t_cells = StripGroup::new("T cells", lcg_data(42, 80, 8.5, 2.5), Color::Cyan);
    let nk_cells = StripGroup::new("NK cells", lcg_data(137, 60, 5.0, 1.8), Color::Green);
    let b_cells = StripGroup::new("B cells", lcg_data(271, 70, 2.0, 1.2), Color::Yellow);
    let macrophages = StripGroup::new("Macrophages", lcg_data(999, 50, 0.8, 2.0), Color::Magenta);

    let plot = StripPlot::new()
        .group(t_cells)
        .group(nk_cells)
        .group(b_cells)
        .group(macrophages)
        .jitter(0.35)
        .title("Gene Expression by Cell Type: CD3E (q to quit)")
        .y_axis(Axis::new().label("log2(TPM + 1)").grid(true));

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
