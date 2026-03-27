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
    let theme = Theme::get_default();
    let mut cycle = theme.color_cycle.clone();

    // Temperature and humidity over 24 hours (inversely correlated)
    let temp = Series::new("Temperature")
        .data(
            (0..=240)
                .map(|h| {
                    let t = h as f64 * 0.1;
                    let temp = 15.0 + 10.0 * ((t - 14.0) * std::f64::consts::PI / 12.0).sin();
                    (t, temp)
                })
                .collect(),
        )
        .color(cycle.next_color());

    let humidity = Series::new("Humidity")
        .data(
            (0..=240)
                .map(|h| {
                    let t = h as f64 * 0.1;
                    let hum = 70.0 - 20.0 * ((t - 14.0) * std::f64::consts::PI / 12.0).sin();
                    (t, hum)
                })
                .collect(),
        )
        .color(cycle.next_color());

    let plot = TwinAxes::new()
        .primary(temp)
        .secondary(humidity)
        .x_axis(Axis::new().label("Hour"))
        .primary_y_axis(
            Axis::new()
                .label("Temperature (\u{00b0}C)")
                .label_position(LabelPosition::End),
        )
        .secondary_y_axis(
            Axis::new()
                .label("Humidity (%)")
                .label_position(LabelPosition::End),
        )
        .title("24h Weather: Temperature & Humidity (q to quit)");

    if headless_export(|area, buf| (&plot).render(area, buf))? {
        return Ok(());
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

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
