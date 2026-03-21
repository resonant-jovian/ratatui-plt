//! Radial (polar) plot example: four plot types and directional control.
//!
//! Shows a 2x2 grid with Line, Scatter, Bar, and FillBetween polar plot types.
//! Demonstrates theta_direction (clockwise compass style), theta_offset, and r_min.

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
        Some("dark") | None => Theme::dark(),
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

    let n = 3600;

    // Cardioid: r = 1 + cos(theta) — Line mode (default CCW)
    let cardioid: Vec<(f64, f64)> = (0..=n)
        .map(|i| {
            let theta = i as f64 * std::f64::consts::TAU / n as f64;
            (theta, 1.0 + theta.cos())
        })
        .collect();

    let plot_line = RadialPlot::new()
        .series(
            Series::new("Cardioid: r=1+cos(\u{03b8})")
                .data(cardioid)
                .color(Color::Rgb(0, 255, 255)), // bright cyan
        )
        .title("Line (CCW)")
        .plot_type(PolarPlotType::Line)
        .n_rings(4)
        .n_spokes(8)
        .r_max(2.5);

    // Rose curve: scatter points — Clockwise with North=0
    let rose: Vec<(f64, f64)> = (0..360)
        .map(|i| {
            let theta = i as f64 * std::f64::consts::TAU / 360.0;
            (theta, (3.0 * theta).cos().abs())
        })
        .collect();

    let plot_scatter = RadialPlot::new()
        .series(
            Series::new("Rose: r=|cos(3\u{03b8})|")
                .data(rose)
                .color(Color::Rgb(255, 255, 0)) // bright yellow
                .marker(MarkerShape::Diamond),
        )
        .title("Scatter (CW, N=0)")
        .plot_type(PolarPlotType::Scatter)
        .theta_direction(ThetaDirection::Clockwise)
        .theta_offset(std::f64::consts::FRAC_PI_2) // North at top
        .n_rings(3)
        .n_spokes(12);

    // Wind rose style — Bar mode
    let wind_data: Vec<(f64, f64)> = (0..16)
        .map(|i| {
            let theta = i as f64 * std::f64::consts::TAU / 16.0;
            let r = 3.0 + 2.0 * (theta * 2.0).cos() + 1.5 * (theta * 3.0).sin();
            (theta, r)
        })
        .collect();

    let plot_bar = RadialPlot::new()
        .series(
            Series::new("Wind speed")
                .data(wind_data)
                .color(Color::Rgb(0, 255, 100)), // bright green
        )
        .title("Bar (CW compass)")
        .plot_type(PolarPlotType::Bar)
        .theta_direction(ThetaDirection::Clockwise)
        .theta_offset(std::f64::consts::FRAC_PI_2)
        .n_rings(3)
        .n_spokes(8);

    // Filled area — with r_min offset
    let fill_data: Vec<(f64, f64)> = (0..=n)
        .map(|i| {
            let theta = i as f64 * std::f64::consts::TAU / n as f64;
            (theta, 1.0 + 0.3 * (5.0 * theta).sin())
        })
        .collect();

    let plot_fill = RadialPlot::new()
        .series(
            Series::new("r=1+0.3sin(5\u{03b8})")
                .data(fill_data)
                .color(Color::Rgb(255, 100, 255)), // bright magenta
        )
        .title("FillBetween (r_min=0.5)")
        .plot_type(PolarPlotType::FillBetween)
        .r_min(0.5)
        .n_rings(4)
        .n_spokes(8)
        .r_max(1.5);

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            let top = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[0]);
            let bot = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[1]);

            frame.render_widget(&plot_line, top[0]);
            frame.render_widget(&plot_scatter, top[1]);
            frame.render_widget(&plot_bar, bot[0]);
            frame.render_widget(&plot_fill, bot[1]);
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
