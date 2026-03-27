//! Smith chart example.
//!
//! Shows RF impedance points on a Smith chart with constant-resistance circles
//! and constant-reactance arcs, connected by a trace line.

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

    let theme = Theme::get_default();
    let mut cycle = theme.color_cycle.clone();
    let trace_color = cycle.next_color();

    // Simulate an impedance sweep: points along a frequency sweep of an
    // antenna matching network (normalized to Z0 = 50 ohms)
    let chart = SmithChart::new()
        .point(
            SmithChartPoint::new(0.2, -0.5)
                .label("100 MHz")
                .color(trace_color),
        )
        .point(SmithChartPoint::new(0.4, -0.3).color(trace_color))
        .point(SmithChartPoint::new(0.7, -0.1).color(trace_color))
        .point(
            SmithChartPoint::new(1.0, 0.0)
                .label("f0 (matched)")
                .color(trace_color),
        )
        .point(SmithChartPoint::new(1.3, 0.2).color(trace_color))
        .point(SmithChartPoint::new(1.8, 0.6).color(trace_color))
        .point(
            SmithChartPoint::new(2.5, 1.2)
                .label("500 MHz")
                .color(trace_color),
        )
        .point(SmithChartPoint::new(3.0, 2.0).color(trace_color))
        .point(
            SmithChartPoint::new(0.5, 1.0)
                .label("Load A")
                .color(cycle.next_color()),
        )
        .point(
            SmithChartPoint::new(2.0, -1.5)
                .label("Load B")
                .color(cycle.next_color()),
        )
        .show_trace(true)
        .n_r_circles(6)
        .n_x_arcs(6)
        .title("S11 Impedance Chart (q to quit)");

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&chart, square_area(frame.area()));
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
