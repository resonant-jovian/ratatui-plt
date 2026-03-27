//! Regression plot gallery: linear vs polynomial fit comparison.
//!
//! Two side-by-side panels showing different regression scenarios:
//! - Left: Linear regression (order=1) on noisy linear data with 95% CI band
//! - Right: Polynomial regression (order=3) on noisy cubic data with 95% CI band
//!
//! Demonstrates when each fit type is appropriate.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;

/// Simple LCG-based PRNG for reproducible data.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z = z ^ (z >> 31);
        (z as f64) / (u64::MAX as f64)
    }

    /// Approximate normal(0,1) via sum of 12 uniforms minus 6.
    fn next_normal(&mut self) -> f64 {
        let mut sum = 0.0;
        for _ in 0..12 {
            sum += self.next_f64();
        }
        sum - 6.0
    }
}

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
    let theme = Theme::get_default();

    // ── Left panel: Linear regression on y = 2x + 3 + noise ───────────
    let mut rng = Rng::new(42);
    let n_linear = 80;
    let linear_data: Vec<(f64, f64)> = (0..n_linear)
        .map(|i| {
            let x = i as f64 * 0.12;
            let y = 2.0 * x + 3.0 + 1.5 * rng.next_normal();
            (x, y)
        })
        .collect();

    let linear_series = Series::new("Observations")
        .data(linear_data)
        .color(theme.primary)
        .marker(MarkerShape::Circle);

    let linear_plot = RegressionPlot::new(linear_series)
        .order(1)
        .ci(0.95)
        .show_scatter(true)
        .show_ci(true)
        .line_color(theme.accent)
        .title("Linear: y = 2x + 3 + noise")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    // ── Right panel: Polynomial regression on cubic data ───────────────
    // y = 0.02*x^3 - 0.3*x^2 + x + 2 + noise
    let mut rng2 = Rng::new(1337);
    let n_poly = 80;
    let poly_data: Vec<(f64, f64)> = (0..n_poly)
        .map(|i| {
            let x = -4.0 + i as f64 * 0.12;
            let y = 0.02 * x.powi(3) - 0.3 * x * x + x + 2.0 + 0.8 * rng2.next_normal();
            (x, y)
        })
        .collect();

    let poly_series = Series::new("Observations")
        .data(poly_data)
        .color(theme.secondary)
        .marker(MarkerShape::Diamond);

    let poly_plot = RegressionPlot::new(poly_series)
        .order(3)
        .ci(0.95)
        .show_scatter(true)
        .show_ci(true)
        .line_color(theme.accent)
        .title("Cubic: 0.02x\u{00b3} - 0.3x\u{00b2} + x + 2")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    // ── Headless export ────────────────────────────────────────────────
    if headless_export(|area, buf| {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(area);
        (&linear_plot).render(cols[0], buf);
        (&poly_plot).render(cols[1], buf);
    })? {
        return Ok(());
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // ── Event loop ─────────────────────────────────────────────────────
    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(area);

            frame.render_widget(&linear_plot, cols[0]);
            frame.render_widget(&poly_plot, cols[1]);
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
