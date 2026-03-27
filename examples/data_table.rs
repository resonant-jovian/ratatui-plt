//! Data table example.
//!
//! Shows a formatted metrics table with 5 columns and colormap-based cell
//! backgrounds highlighting numeric values.

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

    let table = DataTable::new()
        .headers(vec!["Model", "Accuracy", "Precision", "Recall", "F1"])
        .row(vec!["Random Forest", "0.943", "0.931", "0.958", "0.944"])
        .row(vec!["Gradient Boost", "0.961", "0.955", "0.970", "0.962"])
        .row(vec!["SVM (RBF)", "0.912", "0.898", "0.927", "0.912"])
        .row(vec!["Logistic Reg.", "0.874", "0.861", "0.890", "0.875"])
        .row(vec!["k-NN (k=5)", "0.889", "0.876", "0.905", "0.890"])
        .row(vec!["Neural Net", "0.955", "0.948", "0.963", "0.955"])
        .row(vec!["Naive Bayes", "0.831", "0.815", "0.852", "0.833"])
        .row(vec!["Decision Tree", "0.902", "0.891", "0.916", "0.903"])
        .cell_colormap(Viridis)
        .cell_values(vec![
            vec![0.943, 0.931, 0.958, 0.944],
            vec![0.961, 0.955, 0.970, 0.962],
            vec![0.912, 0.898, 0.927, 0.912],
            vec![0.874, 0.861, 0.890, 0.875],
            vec![0.889, 0.876, 0.905, 0.890],
            vec![0.955, 0.948, 0.963, 0.955],
            vec![0.831, 0.815, 0.852, 0.833],
            vec![0.902, 0.891, 0.916, 0.903],
        ])
        .header_color(theme.accent)
        .title("ML Model Comparison (q to quit)");

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&table, square_area(frame.area()));
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
