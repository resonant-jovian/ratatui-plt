//! Inset axes example: a zoomed view embedded inside a main plot.
//!
//! Shows a full-range sine wave with an inset zoomed into the peak region.

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
    let n = 500;
    let data: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let x = i as f64 * 0.04;
            (x, x.sin() * (-x * 0.1).exp())
        })
        .collect();

    let theme = Theme::get_default();

    // Main plot: full range
    let main_series = Series::new("Damped sine")
        .data(data.clone())
        .color(theme.primary);

    // Inset: zoomed into x=[1,3] region around the first peak
    let inset_data: Vec<(f64, f64)> = data
        .iter()
        .filter(|&&(x, _)| (1.0..=3.0).contains(&x))
        .copied()
        .collect();

    // Compute the Y extent of the inset data for the highlighted span
    let inset_y_min = inset_data
        .iter()
        .map(|&(_, y)| y)
        .fold(f64::INFINITY, f64::min);
    let inset_y_max = inset_data
        .iter()
        .map(|&(_, y)| y)
        .fold(f64::NEG_INFINITY, f64::max);
    // Add a small margin around the Y extent
    let y_margin = (inset_y_max - inset_y_min) * 0.05;

    let main_plot = LinePlot::new()
        .series(main_series)
        .title("Damped Sine + Inset (q to quit)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .reference_line(ReferenceLine::hline_dashed(0.0, theme.muted))
        .reference_line(ReferenceLine::vspan_bounded(
            1.0,
            3.0,
            inset_y_min - y_margin,
            inset_y_max + y_margin,
            theme.surface,
        ));

    let inset_series = Series::new("Peak").data(inset_data).color(theme.highlight);

    let inset_plot = LinePlot::new()
        .series(inset_series)
        .title("x=[1,3]")
        .x_axis(Axis::new().bounds(Bounds::Manual(1.0, 3.0)).grid(true))
        .y_axis(Axis::new().grid(true))
        .show_legend(false);

    // Place inset in the right half, below the title, with enough room for labels
    let inset = InsetAxes::new(0.55, 0.10, 0.40, 0.42)
        .border(true)
        .border_color(theme.highlight);

    if headless_export(|area, buf| {
        (&main_plot).render(area, buf);
        inset.render_with(area, buf, |inset_area, buf| {
            (&inset_plot).render(inset_area, buf);
        });
    })? {
        return Ok(());
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());

            // Render main plot
            frame.render_widget(&main_plot, area);

            // Render inset on top
            let buf = frame.buffer_mut();
            inset.render_with(area, buf, |inset_area, buf| {
                (&inset_plot).render(inset_area, buf);
            });
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
