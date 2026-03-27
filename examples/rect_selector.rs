//! Rectangle selector example: select a rectangular region on a scatter plot.
//!
//! Use arrow keys to move the selection rectangle, +/- to resize.
//! Press r to reset, q or Esc to quit.

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

    // Generate scatter data: two clusters
    let mut points: Vec<(f64, f64)> = Vec::new();
    for i in 0..40 {
        let t = i as f64 * 0.15;
        points.push((2.0 + t.cos() * 1.5, 3.0 + t.sin() * 1.2));
    }
    for i in 0..40 {
        let t = i as f64 * 0.15;
        points.push((7.0 + t.sin() * 1.3, 7.0 + t.cos() * 1.0));
    }

    let scatter = Series::new("clusters").data(points).color(theme.primary);

    // Set up brush selection state
    let brush = shared_brush();
    brush.borrow_mut().set_selection(1.0, 2.0, 5.0, 5.0);

    let step = 0.3;

    let x_axis = Axis::new().label("x").grid(true);
    let y_axis = Axis::new().label("y").grid(true);

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            let theme = Theme::get_default();
            let buf = frame.buffer_mut();

            let pf = PlotFrame::new(&x_axis, &y_axis, &theme).title(Some(
                "Rect Selector (arrows: move, +/-: resize, r: reset, q: quit)",
            ));

            let x_lo = -1.0_f64;
            let x_hi = 11.0_f64;
            let y_lo = -1.0_f64;
            let y_hi = 11.0_f64;

            if let Some(pa) = pf.render(
                area,
                buf,
                DataBounds {
                    x_lo,
                    x_hi,
                    y_lo,
                    y_hi,
                },
            ) {
                // Draw scatter points
                for &(x, y) in &scatter.data {
                    let sx = pa.screen_x(x).round() as u16;
                    let sy = pa.screen_y(y).round() as u16;
                    if pa.contains(sx, sy) {
                        buf[(sx, sy)]
                            .set_char('*')
                            .set_fg(scatter.color.unwrap_or(theme.foreground));
                    }
                }

                // Draw rectangle selector overlay
                let selector = RectangleSelector::new(brush.clone())
                    .color(theme.accent)
                    .border(true);
                selector.render_on(&pa, buf);
            }
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Left => {
                    let mut st = brush.borrow_mut();
                    if let Some((x_min, y_min, x_max, y_max)) = st.selection {
                        st.set_selection(x_min - step, y_min, x_max - step, y_max);
                    }
                }
                KeyCode::Right => {
                    let mut st = brush.borrow_mut();
                    if let Some((x_min, y_min, x_max, y_max)) = st.selection {
                        st.set_selection(x_min + step, y_min, x_max + step, y_max);
                    }
                }
                KeyCode::Up => {
                    let mut st = brush.borrow_mut();
                    if let Some((x_min, y_min, x_max, y_max)) = st.selection {
                        st.set_selection(x_min, y_min + step, x_max, y_max + step);
                    }
                }
                KeyCode::Down => {
                    let mut st = brush.borrow_mut();
                    if let Some((x_min, y_min, x_max, y_max)) = st.selection {
                        st.set_selection(x_min, y_min - step, x_max, y_max - step);
                    }
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    let mut st = brush.borrow_mut();
                    if let Some((x_min, y_min, x_max, y_max)) = st.selection {
                        st.set_selection(x_min - step, y_min - step, x_max + step, y_max + step);
                    }
                }
                KeyCode::Char('-') => {
                    let mut st = brush.borrow_mut();
                    if let Some((x_min, y_min, x_max, y_max)) = st.selection {
                        st.set_selection(x_min + step, y_min + step, x_max - step, y_max - step);
                    }
                }
                KeyCode::Char('r') => {
                    brush.borrow_mut().set_selection(1.0, 2.0, 5.0, 5.0);
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
