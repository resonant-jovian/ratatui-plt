use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use ratatui_sim::prelude::*;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Cardioid: r = 1 + cos(theta)
    let cardioid: Vec<(f64, f64)> = (0..=360)
        .map(|i| {
            let theta = i as f64 * std::f64::consts::PI / 180.0;
            (theta, 1.0 + theta.cos())
        })
        .collect();

    // Rose curve: r = cos(2*theta)
    let rose: Vec<(f64, f64)> = (0..=360)
        .map(|i| {
            let theta = i as f64 * std::f64::consts::PI / 180.0;
            (theta, (2.0 * theta).cos().abs())
        })
        .collect();

    let plot = RadialPlot::new()
        .series(
            Series::new("Cardioid: r=1+cos(\u{03b8})")
                .data(cardioid)
                .color(Color::Cyan),
        )
        .series(
            Series::new("Rose: r=|cos(2\u{03b8})|")
                .data(rose)
                .color(Color::Yellow),
        )
        .title("Polar Coordinate Plot (q to quit)")
        .n_rings(4)
        .n_spokes(8)
        .r_max(2.5);

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(&plot, area);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press
                && (key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
            {
                break;
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
