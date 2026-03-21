//! Data picking example: nearest-point detection across series.
//!
//! Move the crosshair with arrow keys (or h/j/k/l) to see the nearest data
//! point highlighted with its coordinates. Demonstrates pick_nearest() and
//! the Crosshair overlay working together.

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

    let n = 80;
    let s1 = Series::new("sin(x)")
        .data(
            (0..n)
                .map(|i| {
                    let x = i as f64 * 0.1;
                    (x, x.sin())
                })
                .collect(),
        )
        .color(Color::Rgb(80, 200, 255))
        .marker(MarkerShape::FilledCircle);

    let s2 = Series::new("cos(x)")
        .data(
            (0..n)
                .map(|i| {
                    let x = i as f64 * 0.1;
                    (x, x.cos())
                })
                .collect(),
        )
        .color(Color::Rgb(255, 180, 50))
        .marker(MarkerShape::FilledCircle);

    let all_series = vec![s1.clone(), s2.clone()];

    // Cursor position in data coordinates
    let mut cursor_x: f64 = 4.0;
    let mut cursor_y: f64 = 0.0;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            // Build a PlotArea for picking (approximate — matching the LinePlot frame)
            let pa = PlotArea {
                x: area.x + 8,
                y: area.y + 1,
                width: area.width.saturating_sub(10),
                height: area.height.saturating_sub(3),
                x_lo: -0.4,
                x_hi: 8.4,
                y_lo: -1.1,
                y_hi: 1.1,
                area,
            };

            // Find nearest point
            let pick = pick_nearest(
                pa.screen_x(cursor_x).round() as u16,
                pa.screen_y(cursor_y).round() as u16,
                &all_series,
                &pa,
            );

            let info = if let Some(ref p) = pick {
                format!(
                    " Nearest: {} #{} ({:.2}, {:.2}) dist={:.1}",
                    p.series_name, p.point_index, p.data_x, p.data_y, p.distance
                )
            } else {
                String::new()
            };

            let plot = LinePlot::new()
                .series(s1.clone())
                .series(s2.clone())
                .title(format!("Arrow keys to move crosshair | q to quit{info}"))
                .x_axis(
                    Axis::new()
                        .label("x")
                        .grid(true)
                        .bounds(Bounds::Manual(-0.4, 8.4)),
                )
                .y_axis(
                    Axis::new()
                        .label("y")
                        .grid(true)
                        .bounds(Bounds::Manual(-1.1, 1.1)),
                )
                .show_legend(true);

            let buf = frame.buffer_mut();
            (&plot).render(area, buf);

            // Draw crosshair at cursor
            let ch = Crosshair::new(cursor_x, cursor_y)
                .color(Color::Gray)
                .show_labels(true);
            ch.render_on(&pa, buf);

            // Highlight the picked point
            if let Some(ref p) = pick {
                let sx = pa.screen_x(p.data_x).round() as u16;
                let sy = pa.screen_y(p.data_y).round() as u16;
                if pa.contains(sx, sy) {
                    buf[(sx, sy)].set_char('*').set_fg(Color::White);
                }
            }
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Left | KeyCode::Char('h') => cursor_x -= 0.2,
                KeyCode::Right | KeyCode::Char('l') => cursor_x += 0.2,
                KeyCode::Up | KeyCode::Char('k') => cursor_y += 0.1,
                KeyCode::Down | KeyCode::Char('j') => cursor_y -= 0.1,
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
