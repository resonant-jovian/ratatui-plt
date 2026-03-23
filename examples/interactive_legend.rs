//! Interactive legend example: toggle series visibility with number keys.
//!
//! Press 1/2/3 to toggle visibility of each series. The legend shows
//! hidden entries with a dimmed style. Press q to quit.

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

    let n = 100;
    let s1 = Series::new("sin(x)")
        .data(
            (0..n)
                .map(|i| {
                    let x = i as f64 * 0.1;
                    (x, x.sin())
                })
                .collect(),
        )
        .color(Color::Rgb(80, 200, 255));

    let s2 = Series::new("cos(x)")
        .data(
            (0..n)
                .map(|i| {
                    let x = i as f64 * 0.1;
                    (x, x.cos())
                })
                .collect(),
        )
        .color(Color::Rgb(255, 180, 50));

    let s3 = Series::new("sin(2x)")
        .data(
            (0..n)
                .map(|i| {
                    let x = i as f64 * 0.1;
                    (x, (2.0 * x).sin())
                })
                .collect(),
        )
        .color(Color::Rgb(100, 255, 100));

    let all_series = vec![s1, s2, s3];
    let legend_state = shared_legend_state(all_series.len());

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());

            // Only plot visible series
            let vis = legend_state.borrow();
            let mut visible_series = Vec::new();
            for (i, s) in all_series.iter().enumerate() {
                if vis.get(i).copied().unwrap_or(true) {
                    visible_series.push(s.clone());
                }
            }
            drop(vis);

            let mut plot = LinePlot::new()
                .title("Interactive Legend (1/2/3 to toggle, q to quit)")
                .x_axis(Axis::new().label("x").grid(true))
                .y_axis(Axis::new().label("y").grid(true))
                .show_legend(false); // We draw our own interactive legend

            for s in &visible_series {
                plot = plot.series(s.clone());
            }

            let buf = frame.buffer_mut();
            (&plot).render(area, buf);

            // Render the interactive legend as an overlay
            let legend = InteractiveLegend::from_series(&all_series, legend_state.clone())
                .position(LegendPosition::TopRight);
            let legend_area = Rect::new(
                area.x + 8,
                area.y + 1,
                area.width.saturating_sub(10),
                area.height.saturating_sub(3),
            );
            (&legend).render(legend_area, buf);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('1') => {
                    let mut st = legend_state.borrow_mut();
                    if let Some(v) = st.get_mut(0) {
                        *v = !*v;
                    }
                }
                KeyCode::Char('2') => {
                    let mut st = legend_state.borrow_mut();
                    if let Some(v) = st.get_mut(1) {
                        *v = !*v;
                    }
                }
                KeyCode::Char('3') => {
                    let mut st = legend_state.borrow_mut();
                    if let Some(v) = st.get_mut(2) {
                        *v = !*v;
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
