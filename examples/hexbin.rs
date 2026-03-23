use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use rand::RngExt;
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

    // Generate 10000 points from 3 Gaussian clusters using Box-Muller transform
    let mut rng = rand::rng();
    let mut data = Vec::with_capacity(100000);

    let clusters: [(f64, f64, f64, usize); 3] = [
        (0.0, 0.0, 1.0, 50000),  // Dense core at origin
        (3.0, 3.0, 0.8, 30000),  // Tight secondary cluster
        (-2.0, 2.0, 1.5, 20000), // Diffuse spread
    ];

    for &(cx, cy, sigma, count) in &clusters {
        for _ in 0..count {
            let u1: f64 = rng.random::<f64>().max(1e-10);
            let u2: f64 = rng.random::<f64>();
            let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            let z1 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).sin();
            data.push((cx + z0 * sigma, cy + z1 * sigma));
        }
    }

    let plot = HexbinPlot::new(data)
        .gridsize(80)
        .colormap(Plasma)
        .title("Gaussian Cluster Hexbin Density (q to quit)")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"));

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&plot, square_area(frame.area()));
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
