//! Box plot example: three comparison modes.
//!
//! Shows a 1x3 layout comparing standard boxes, notched boxes (1.57*IQR/sqrt(n)),
//! and bootstrap confidence interval boxes.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::box_plot::BoxData;

/// Generate deterministic pseudo-data with an LCG.
fn generate_data(seed: u64, count: usize, center: f64, spread: f64) -> Vec<f64> {
    let mut values = Vec::with_capacity(count);
    let mut state = seed;
    for _ in 0..count {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let normalized = ((state >> 33) as f64) / (u32::MAX as f64) * 2.0 - 1.0;
        values.push(center + normalized * spread);
    }
    values
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

    let make_groups = || {
        vec![
            BoxData::new("Ctrl", generate_data(42, 500, 5.0, 2.0), Color::Cyan),
            BoxData::new("DrA", generate_data(123, 500, 7.5, 3.0), Color::Yellow),
            BoxData::new("DrB", generate_data(999, 500, 6.0, 1.5), Color::Magenta),
            BoxData::new("DrC", generate_data(7777, 500, 8.0, 2.5), Color::Green),
        ]
    };

    // Standard box plot with means
    let standard = {
        let mut p = BoxPlot::new()
            .title("Standard")
            .y_axis(Axis::new().label("Value").grid(true))
            .show_means(true)
            .reference_line(ReferenceLine::hline_dashed(6.5, Color::DarkGray));
        for g in make_groups() {
            p = p.box_data(g);
        }
        p
    };

    // Notched box plot (1.57*IQR/sqrt(n))
    let notched = {
        let mut p = BoxPlot::new()
            .title("Notched")
            .y_axis(Axis::new().label("Value").grid(true))
            .notch(true)
            .fill_boxes(false)
            .show_means(true);
        for g in make_groups() {
            p = p.box_data(g);
        }
        p
    };

    // Bootstrap CI box plot
    let bootstrap = {
        let mut p = BoxPlot::new()
            .title("Bootstrap CI")
            .y_axis(Axis::new().label("Value").grid(true))
            .bootstrap_ci(true)
            .bootstrap_n(1000)
            .show_means(true)
            .fill_boxes(true);
        for g in make_groups() {
            p = p.box_data(g);
        }
        p
    };

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(area);

            frame.render_widget(&standard, cols[0]);
            frame.render_widget(&notched, cols[1]);
            frame.render_widget(&bootstrap, cols[2]);
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
