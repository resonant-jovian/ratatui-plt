//! Candlestick chart example: OHLC Price Action.
//!
//! 50 candles showing a trending-then-mean-reverting price pattern.
//! Price is generated deterministically using an LCG-based random walk
//! with drift that reverses after a trend peak.

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

    // Generate 50 OHLC candles with a trending-then-reverting pattern.
    //
    // Phase 1 (candles 0-30): uptrend with positive drift
    // Phase 2 (candles 30-50): mean-reversion/pullback
    let n_candles = 50;
    let mut candles = Vec::with_capacity(n_candles);
    let mut state: u64 = 0xDEAD_BEEF_CAFE;
    let mut price = 100.0_f64;

    for i in 0..n_candles {
        let t = i as f64;

        // Drift: positive in phase 1, negative in phase 2
        let drift = if i < 30 {
            0.5 + 0.3 * (t * 0.15).sin()
        } else {
            -0.8 + 0.2 * (t * 0.2).cos()
        };

        // Volatility varies with a slow oscillation
        let vol = 1.5 + 0.8 * (t * 0.12).sin().abs();

        // Generate 4 pseudo-random increments for O, H, L, C
        let mut next_rand = || -> f64 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0
        };

        let open = price;
        let move1 = next_rand() * vol + drift;
        let move2 = next_rand() * vol * 0.5;
        let move3 = next_rand() * vol * 0.5;
        let close = open + move1;

        // High is above both open and close; low is below both
        let body_hi = open.max(close);
        let body_lo = open.min(close);
        let high = body_hi + move2.abs() * 0.8;
        let low = body_lo - move3.abs() * 0.8;

        candles.push(Candle::new(t, open, high, low, close));
        price = close;
    }

    let chart = CandlestickChart::new()
        .candles(candles)
        .bull_color(Color::Green)
        .bear_color(Color::Red)
        .title("OHLC Price Action (q to quit)")
        .x_axis(Axis::new().label("Session").grid(true))
        .y_axis(Axis::new().label("Price [$]").grid(true));

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&chart, frame.area());
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
