//! Regression plot example: scatter with OLS fit and confidence band.
//!
//! Generates a noisy linear relationship y = 2x + 1 + noise and shows
//! the scatter points, fitted regression line, and 95% confidence band.

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
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let theme = Theme::get_default();

    // Generate noisy linear data: y = 2x + 1 + noise
    let mut rng = Rng::new(42);
    let n = 60;
    let data: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let x = i as f64 * 0.15;
            let y = 2.0 * x + 1.0 + 1.5 * rng.next_normal();
            (x, y)
        })
        .collect();

    let series = Series::new("Observations")
        .data(data)
        .color(theme.primary)
        .marker(MarkerShape::Circle);

    let plot = RegressionPlot::new(series)
        .order(1)
        .ci(0.95)
        .show_scatter(true)
        .show_ci(true)
        .line_color(theme.accent)
        .title("Linear Regression: y = 2x + 1 + noise (q to quit)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            frame.render_widget(&plot, area);
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
