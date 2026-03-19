//! Heatmap example: 2D Gaussian with Viridis colormap, colorbar, and equal aspect ratio.

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

    // 2D Gaussian: z = exp(-(x^2 + y^2))
    let data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 80, 80, |x, y| {
        (-(x * x + y * y)).exp()
    });

    let heatmap = Heatmap::new(data)
        .colormap(Viridis)
        .title("2D Gaussian  z = exp(-(x^2 + y^2))")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"))
        .show_colorbar(true)
        .aspect_ratio(AspectRatio::Equal);

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&heatmap, frame.area());
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
