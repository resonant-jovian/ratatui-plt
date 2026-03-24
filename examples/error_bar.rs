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

    // Noisy quadratic with asymmetric errors
    let n = 10;
    let points: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let x = i as f64;
            let noise = ((i * 7 + 3) % 5) as f64 * 0.4 - 1.0;
            (x, x * x * 0.5 + noise)
        })
        .collect();
    let err_low: Vec<f64> = (0..n).map(|i| 0.5 + (i as f64) * 0.2).collect();
    let err_high: Vec<f64> = (0..n).map(|i| 1.0 + (i as f64) * 0.3).collect();
    let x_err_low: Vec<f64> = vec![0.3; n];
    let x_err_high: Vec<f64> = vec![0.3; n];

    let theme = Theme::get_default();

    let plot = ErrorBarPlot::new()
        .data(points, err_low, err_high)
        .x_errors(x_err_low, x_err_high)
        .color(theme.primary)
        .title("Noisy Quadratic with Error Bars (q to quit)")
        .x_axis(Axis::new().label("x"))
        .y_axis(
            Axis::new()
                .label("y = 0.5x\u{00b2} + noise")
                .label_position(LabelPosition::End),
        );

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
