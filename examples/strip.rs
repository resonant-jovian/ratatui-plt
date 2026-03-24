//! Strip plot example: Gene Expression by Cell Type.
//!
//! Simulates single-cell gene expression measurements (log2 TPM) for a
//! marker gene across 5 immune cell types, each with distinct expression
//! distributions generated via a deterministic linear congruential generator.
//! Features custom styling with spine control and a reference threshold line.

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

    // Five immune cell types with different expression patterns for gene CD3E:
    //   T cells:        high expression (center 8.5, moderate spread)
    //   NK cells:       moderate expression (center 5.0, tight)
    //   B cells:        low expression (center 2.0, tight)
    //   Monocytes:      bimodal - some activated, some off (center 3.5, wide)
    //   Macrophages:    very low / off (center 0.8, wide due to noise)
    let theme = Theme::get_default();
    let mut cycle = theme.color_cycle.clone();
    let t_cells = StripGroup::new("T cells", lcg_data(42, 100, 8.5, 2.5), cycle.next_color());
    let nk_cells = StripGroup::new("NK cells", lcg_data(137, 80, 5.0, 1.8), cycle.next_color());
    let b_cells = StripGroup::new("B cells", lcg_data(271, 90, 2.0, 1.2), cycle.next_color());
    let monocytes = StripGroup::new(
        "Monocytes",
        lcg_data(503, 70, 3.5, 3.0),
        cycle.next_color(),
    );
    let macrophages = StripGroup::new("Macrophages", lcg_data(999, 60, 0.8, 2.0), cycle.next_color());

    let plot = StripPlot::new()
        .group(t_cells)
        .group(nk_cells)
        .group(b_cells)
        .group(monocytes)
        .group(macrophages)
        .jitter(0.38)
        .title("Single-Cell Gene Expression: CD3E (q to quit)")
        .y_axis(
            Axis::new()
                .label("log2(TPM + 1)")
                .label_position(LabelPosition::End)
                .grid(true),
        )
        .spines(Spines::new().top(false).right(false))
        .reference_line(ReferenceLine::hline_dashed(5.0, theme.muted));

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
