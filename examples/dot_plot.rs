//! Dot plot example.
//!
//! Shows exam score distributions for two classes as stacked dots using the
//! `DotPlot` and `DotDataset` types.

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

    // Exam scores for two classes (pseudo-normal distribution around different means)
    let class_a = vec![
        62.0, 65.0, 67.0, 68.0, 70.0, 70.0, 71.0, 72.0, 72.0, 73.0, 73.0, 73.0, 74.0, 75.0, 75.0,
        75.0, 76.0, 76.0, 77.0, 78.0, 78.0, 79.0, 80.0, 80.0, 81.0, 82.0, 83.0, 85.0, 87.0, 90.0,
    ];

    let class_b = vec![
        55.0, 58.0, 60.0, 62.0, 63.0, 64.0, 65.0, 65.0, 66.0, 66.0, 67.0, 67.0, 67.0, 68.0, 68.0,
        69.0, 69.0, 70.0, 70.0, 71.0, 71.0, 72.0, 73.0, 74.0, 75.0, 76.0, 78.0, 80.0, 82.0, 85.0,
    ];

    let chart = DotPlot::new()
        .dataset(DotDataset::new("Class A", class_a).color(cycle.next_color()))
        .dataset(DotDataset::new("Class B", class_b).color(cycle.next_color()))
        .x_axis(Axis::new().label("Score"))
        .y_axis(Axis::new().label("Count"))
        .title("Exam Score Distribution (q to quit)");

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
