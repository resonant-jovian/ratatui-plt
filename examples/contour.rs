//! Contour plot example: 2D potential field from two Gaussian peaks with filled contours.

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

    // Two-peak Gaussian potential field
    let data = GridData::from_fn((-4.0, 4.0), (-4.0, 4.0), 60, 60, |x, y| {
        let peak1 = (-((x - 1.5).powi(2) + (y - 1.0).powi(2)) / 1.5).exp();
        let peak2 = 0.8 * (-((x + 1.0).powi(2) + (y + 1.5).powi(2)) / 2.0).exp();
        peak1 + peak2
    });

    let plot = ContourPlot::new(data)
        .levels(12)
        .filled(true)
        .colormap(Viridis)
        .title("2D Potential Field (two Gaussian peaks)")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"));

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&plot, frame.area());
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
