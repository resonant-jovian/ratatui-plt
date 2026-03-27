//! Ridgeline plot example: overlapping density curves for monthly data.
//!
//! Shows six months of distributions as vertically stacked, partially
//! overlapping KDE density curves (a "joy division" style plot).

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

    // Generate 6 months of data with shifting centers
    let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];
    let centers = [3.0, 4.5, 5.0, 6.5, 7.0, 8.5];
    let spreads = [1.0, 1.2, 0.8, 1.5, 1.0, 1.3];

    let mut rng = Rng::new(42);

    let mut plot = RidgelinePlot::new()
        .title("Monthly Distribution (q to quit)")
        .x_axis(Axis::new().label("Value").grid(true))
        .overlap(0.5)
        .fill(true);

    for (i, &month) in months.iter().enumerate() {
        let data: Vec<f64> = (0..150)
            .map(|_| centers[i] + spreads[i] * rng.next_normal())
            .collect();
        let color = theme.color_cycle.at(i);
        plot = plot.group(RidgelineGroup::new(month, data).color(color));
    }

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
