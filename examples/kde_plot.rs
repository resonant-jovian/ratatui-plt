//! KDE plot example: three overlapping density curves with fill.
//!
//! Shows kernel density estimation for three groups of data with
//! different centers and spreads, rendered as filled density curves.

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

    // Generate three groups of 200 values with different centers
    let mut rng = Rng::new(42);
    let group_a: Vec<f64> = (0..200).map(|_| 2.0 + 0.8 * rng.next_normal()).collect();
    let group_b: Vec<f64> = (0..200).map(|_| 5.0 + 1.2 * rng.next_normal()).collect();
    let group_c: Vec<f64> = (0..200).map(|_| 3.5 + 0.5 * rng.next_normal()).collect();

    let plot = KDEPlot::new()
        .dataset(KdeDataset::new("Group A", group_a).color(cycle.next_color()))
        .dataset(KdeDataset::new("Group B", group_b).color(cycle.next_color()))
        .dataset(KdeDataset::new("Group C", group_c).color(cycle.next_color()))
        .fill(true)
        .title("Kernel Density Estimation (q to quit)")
        .x_axis(Axis::new().label("Value").grid(true))
        .y_axis(
            Axis::new()
                .label("Density")
                .label_position(LabelPosition::End)
                .grid(true),
        )
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

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
