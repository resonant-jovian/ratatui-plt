//! Stairs plot example: Photon Energy Spectrum.
//!
//! Simulates a measured gamma-ray spectrum with characteristic peaks
//! (511 keV annihilation, 662 keV Cs-137, 1173/1332 keV Co-60)
//! on a Compton continuum background.

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

/// Gaussian peak contribution at energy `e` with center `mu`, width `sigma`, and amplitude `amp`.
fn gauss_peak(e: f64, mu: f64, sigma: f64, amp: f64) -> f64 {
    amp * (-0.5 * ((e - mu) / sigma).powi(2)).exp()
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Energy bins from 0 to 2000 keV in 40 keV steps
    let n_bins = 50;
    let e_min = 0.0;
    let e_max = 2000.0;
    let bin_width = (e_max - e_min) / n_bins as f64;

    // n+1 edges
    let edges: Vec<f64> = (0..=n_bins).map(|i| e_min + i as f64 * bin_width).collect();

    // Compute counts per bin: Compton continuum + characteristic peaks
    let values: Vec<f64> = (0..n_bins)
        .map(|i| {
            let e_center = edges[i] + bin_width / 2.0;

            // Compton continuum: 1/E falloff
            let continuum = 800.0 / (1.0 + e_center / 200.0);

            // Photopeaks from common gamma sources:
            // 511 keV - positron annihilation
            let peak_511 = gauss_peak(e_center, 511.0, 25.0, 350.0);
            // 662 keV - Cs-137
            let peak_662 = gauss_peak(e_center, 662.0, 30.0, 500.0);
            // 1173 keV - Co-60 gamma 1
            let peak_1173 = gauss_peak(e_center, 1173.0, 35.0, 200.0);
            // 1332 keV - Co-60 gamma 2
            let peak_1332 = gauss_peak(e_center, 1332.0, 35.0, 180.0);

            (continuum + peak_511 + peak_662 + peak_1173 + peak_1332).max(0.0)
        })
        .collect();

    let spectrum = StairsDataset::new("Measured Spectrum", edges, values, Color::Cyan);

    let plot = StairsPlot::new()
        .dataset(spectrum)
        .baseline(0.0)
        .title("Photon Energy Spectrum (q to quit)")
        .x_axis(Axis::new().label("Energy [keV]").grid(true))
        .y_axis(Axis::new().label("Counts").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

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
