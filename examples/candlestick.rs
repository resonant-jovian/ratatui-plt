//! Candlestick chart example: OHLC Price Action.
//!
//! 50 candles showing realistic price action with rallies, pullbacks,
//! and consolidation phases. Price is generated deterministically using
//! an LCG-based random walk with phase-dependent drift and volatility.

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

    // Generate 50 OHLC candles with realistic multi-phase price action.
    //
    // Phase 1 (candles 1-12):  Accumulation / consolidation around 100
    // Phase 2 (candles 13-25): Rally with increasing momentum
    // Phase 3 (candles 26-35): Distribution / volatile topping
    // Phase 4 (candles 36-42): Sharp pullback / sell-off
    // Phase 5 (candles 43-50): Recovery bounce with decreasing volatility
    let n_candles = 50;
    let mut candles = Vec::with_capacity(n_candles);
    let mut state: u64 = 0xDEAD_BEEF_CAFE;
    let mut price = 100.0_f64;

    for i in 0..n_candles {
        let session = (i + 1) as f64; // Sessions start at 1
        let t = i as f64;

        // Phase-dependent drift and volatility for realistic price action
        let (drift, vol) = if i < 12 {
            // Accumulation: slight upward bias, low volatility
            let d = 0.15 + 0.1 * (t * 0.3).sin();
            let v = 0.8 + 0.2 * (t * 0.2).cos().abs();
            (d, v)
        } else if i < 25 {
            // Rally: strong upward drift, moderate volatility
            let momentum = (i as f64 - 12.0) / 13.0; // 0..1
            let d = 0.8 + 1.2 * momentum + 0.3 * (t * 0.25).sin();
            let v = 1.2 + 0.5 * momentum;
            (d, v)
        } else if i < 35 {
            // Distribution: choppy, high volatility, near-zero drift
            let d = 0.1 * (t * 0.5).sin() - 0.1;
            let v = 2.2 + 0.8 * (t * 0.35).cos().abs();
            (d, v)
        } else if i < 42 {
            // Pullback: strong downward drift, high volatility
            let panic = (i as f64 - 35.0) / 7.0;
            let d = -1.5 - 1.0 * panic;
            let v = 2.5 + 0.5 * panic;
            (d, v)
        } else {
            // Recovery bounce: moderate up, declining volatility
            let recovery = (i as f64 - 42.0) / 8.0;
            let d = 0.6 + 0.4 * (1.0 - recovery);
            let v = 1.8 - 0.8 * recovery;
            (d, v)
        };

        // Generate pseudo-random increments for O, H, L, C
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
        // Wicks extend 1-3x the body size for visible shadow lines
        let body_hi = open.max(close);
        let body_lo = open.min(close);
        let body_range = (body_hi - body_lo).max(0.3);
        let high = body_hi + move2.abs() * body_range * 1.5 + 0.3;
        let low = body_lo - move3.abs() * body_range * 1.5 - 0.3;

        candles.push(Candle::new(session, open, high, low, close));
        price = close;
    }

    let chart = CandlestickChart::new()
        .candles(candles)
        .bull_color(Color::Green)
        .bear_color(Color::Red)
        .title("OHLC Price Action (q to quit)")
        .x_axis(Axis::new().label("Session").grid(true))
        .y_axis(
            Axis::new()
                .label("Price [$]")
                .grid(true)
                .label_position(LabelPosition::End),
        );

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
