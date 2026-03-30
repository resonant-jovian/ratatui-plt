//! Pair plot example: pairwise scatter matrix with KDE diagonals.
//!
//! Generates a 4-variable dataset with correlated values and displays
//! an N x N grid of scatter plots with KDE density on the diagonal.

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

    // Generate correlated 4-variable dataset (80 samples)
    let mut rng = Rng::new(42);
    let n = 80;

    let mut x1 = Vec::with_capacity(n);
    let mut x2 = Vec::with_capacity(n);
    let mut x3 = Vec::with_capacity(n);
    let mut x4 = Vec::with_capacity(n);

    for _ in 0..n {
        let a = rng.next_normal();
        let b = rng.next_normal();
        // x1 and x2 are correlated through a
        x1.push(2.0 + a * 1.0);
        x2.push(5.0 + a * 0.8 + b * 0.4);
        // x3 depends on x1 with noise
        x3.push(1.0 + a * 0.5 + rng.next_normal() * 0.6);
        // x4 is independent
        x4.push(3.0 + rng.next_normal() * 1.2);
    }

    let plot = PairPlot::new()
        .column(PairPlotColumn::new("Height", x1))
        .column(PairPlotColumn::new("Weight", x2))
        .column(PairPlotColumn::new("Age", x3))
        .column(PairPlotColumn::new("Score", x4))
        .diag(DiagType::Kde)
        .marker(MarkerShape::Dot)
        .color(theme.primary)
        .title("Pair Plot (q to quit)");

    if headless_export(|area, buf| (&plot).render(area, buf))? {
        return Ok(());
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

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
