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
    let n = 500;
    let solar = Series::new("Solar")
        .data(
            (0..n)
                .map(|i| {
                    let t = i as f64;
                    (t, 20.0 + 15.0 * (t * std::f64::consts::TAU / 50.0).sin())
                })
                .collect(),
        )
        .color(cycle.next_color());

    let wind = Series::new("Wind")
        .data(
            (0..n)
                .map(|i| {
                    let t = i as f64;
                    (t, 15.0 + 8.0 * (t * std::f64::consts::TAU / 30.0).cos())
                })
                .collect(),
        )
        .color(cycle.next_color());

    let hydro = Series::new("Hydro")
        .data(
            (0..n)
                .map(|i| {
                    let t = i as f64;
                    (t, 10.0 + 3.0 * (t * std::f64::consts::TAU / 80.0).sin())
                })
                .collect(),
        )
        .color(cycle.next_color());

    let nuclear = Series::new("Nuclear")
        .data((0..n).map(|i| (i as f64, 25.0)).collect())
        .color(cycle.next_color());

    let plot = StackedArea::new()
        .series(solar)
        .series(wind)
        .series(hydro)
        .series(nuclear)
        .title("Energy Production by Source (q to quit)")
        .x_axis(Axis::new().label("Time (days)"))
        .y_axis(
            Axis::new()
                .label("Output (GW)")
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
