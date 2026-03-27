//! Confidence ellipse example: scatter with 1-sigma and 2-sigma ellipses.
//!
//! Generates a 2D approximately Gaussian cloud and overlays confidence
//! ellipses at the 68% (1 sigma) and 95% (2 sigma) levels.

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
    let mut cycle = theme.color_cycle.clone();

    // Generate 2D approximately Gaussian data with correlation
    let mut rng = Rng::new(42);
    let n = 150;
    let mut x_data = Vec::with_capacity(n);
    let mut y_data = Vec::with_capacity(n);

    for _ in 0..n {
        let a = rng.next_normal();
        let b = rng.next_normal();
        // Correlated: x depends on a, y depends on a and b
        x_data.push(2.0 + a * 1.5);
        y_data.push(3.0 + a * 1.0 + b * 0.8);
    }

    let color_1s = cycle.next_color();
    let color_2s = cycle.next_color();

    // 1-sigma ellipse (68.27% confidence)
    let ellipse_1s = ConfidenceEllipse::new(x_data.clone(), y_data.clone())
        .level(0.6827)
        .show_points(true)
        .point_color(theme.foreground)
        .ellipse_color(color_1s)
        .marker(MarkerShape::Dot)
        .title("Confidence Ellipses (q to quit)")
        .x_axis(Axis::new().label("X").grid(true))
        .y_axis(Axis::new().label("Y").grid(true));

    // 2-sigma ellipse (95.45% confidence)
    let ellipse_2s = ConfidenceEllipse::new(x_data, y_data)
        .level(0.9545)
        .show_points(false)
        .ellipse_color(color_2s)
        .title("95% Confidence Ellipse")
        .x_axis(Axis::new().label("X").grid(true))
        .y_axis(Axis::new().label("Y").grid(true));

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(area);

            frame.render_widget(&ellipse_1s, cols[0]);
            frame.render_widget(&ellipse_2s, cols[1]);
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
