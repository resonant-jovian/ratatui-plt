//! Band plot example: Model Uncertainty Band.
//!
//! Shows a damped oscillation model prediction (sin * exp decay) with
//! expanding confidence envelopes at +/-1 sigma and +/-2 sigma,
//! representing increasing prediction uncertainty over time.

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

    // Model: damped oscillation  y(t) = sin(2*pi*t/3) * exp(-t/8)
    // Uncertainty grows with time: sigma(t) = 0.08 + 0.10*t
    let n = 150;
    let t_max = 10.0;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * t_max / (n - 1) as f64).collect();

    let y_center: Vec<f64> = x
        .iter()
        .map(|&t| {
            let omega = 2.0 * std::f64::consts::PI / 3.0;
            (omega * t).sin() * (-t / 8.0).exp()
        })
        .collect();

    let sigma: Vec<f64> = x.iter().map(|&t| 0.08 + 0.10 * t).collect();

    // +/- 2 sigma band (outer)
    let y_2sig_lo: Vec<f64> = y_center
        .iter()
        .zip(sigma.iter())
        .map(|(&y, &s)| y - 2.0 * s)
        .collect();
    let y_2sig_hi: Vec<f64> = y_center
        .iter()
        .zip(sigma.iter())
        .map(|(&y, &s)| y + 2.0 * s)
        .collect();

    // +/- 1 sigma band (inner)
    let y_1sig_lo: Vec<f64> = y_center
        .iter()
        .zip(sigma.iter())
        .map(|(&y, &s)| y - s)
        .collect();
    let y_1sig_hi: Vec<f64> = y_center
        .iter()
        .zip(sigma.iter())
        .map(|(&y, &s)| y + s)
        .collect();

    // Center line as a very thin band (y_center +/- 0)
    let y_line_lo = y_center.clone();
    let y_line_hi = y_center;

    let band_2sig = Band::new("\u{00b1}2\u{03c3} (95%)", x.clone(), y_2sig_lo, y_2sig_hi)
        .color(Color::Blue)
        .alpha_char('\u{2591}'); // light shade

    let band_1sig = Band::new("\u{00b1}1\u{03c3} (68%)", x.clone(), y_1sig_lo, y_1sig_hi)
        .color(Color::Cyan)
        .alpha_char('\u{2592}'); // medium shade

    let center_line = Band::new("Prediction", x.clone(), y_line_lo, y_line_hi)
        .color(Color::White)
        .alpha_char('\u{2501}'); // heavy horizontal

    let plot = BandPlot::new()
        .band(band_2sig)
        .band(band_1sig)
        .band(center_line)
        .title("Model Uncertainty Band (q to quit)")
        .x_axis(Axis::new().label("Time [s]").grid(true))
        .y_axis(Axis::new().label("Amplitude").grid(true))
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
