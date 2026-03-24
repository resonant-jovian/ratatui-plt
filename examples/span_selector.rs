//! Span selector example: select a horizontal range with h/l keys.
//!
//! Press h/l to move the start, H/L to move the end, r to reset. The
//! selected span is drawn as a filled overlay on the line plot. Press q to quit.

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

    let n = 200;
    let s = Series::new("sin(x)")
        .data(
            (0..n)
                .map(|i| {
                    let x = i as f64 * 0.05;
                    (x, x.sin())
                })
                .collect(),
        )
        .color(Color::Rgb(80, 200, 255));

    let span_state = shared_span_state();
    span_state.borrow_mut().start = Some(2.0);
    span_state.borrow_mut().end = Some(5.0);

    let step = 0.2;

    let x_axis = Axis::new().label("x").grid(true);
    let y_axis = Axis::new().label("y").grid(true);

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            let theme = Theme::get_default();

            let buf = frame.buffer_mut();

            // Render the plot frame manually so we get a PlotArea for the overlay
            let pf = PlotFrame::new(&x_axis, &y_axis, &theme)
                .title(Some("Span Selector (h/l start, H/L end, r reset, q quit)"));

            let x_lo = -0.5_f64;
            let x_hi = 10.5_f64;
            let y_lo = -1.3_f64;
            let y_hi = 1.3_f64;

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
                // Draw line data
                for i in 0..s.data.len().saturating_sub(1) {
                    let (x0, y0) = s.data[i];
                    let (x1, y1) = s.data[i + 1];
                    let sx0 = pa.screen_x(x0).round() as u16;
                    let sy0 = pa.screen_y(y0).round() as u16;
                    let sx1 = pa.screen_x(x1).round() as u16;
                    let sy1 = pa.screen_y(y1).round() as u16;
                    // Simple point-based rendering
                    if pa.contains(sx0, sy0) {
                        buf[(sx0, sy0)].set_char('·').set_fg(s.color.unwrap_or(Color::White));
                    }
                    if pa.contains(sx1, sy1) {
                        buf[(sx1, sy1)].set_char('·').set_fg(s.color.unwrap_or(Color::White));
                    }
                }

                // Draw span selector overlay
                let selector = SpanSelector::new(span_state.clone())
                    .direction(SpanDirection::Horizontal)
                    .color(Color::Rgb(100, 100, 200));
                selector.render_on(&pa, buf);
            }
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('h') => {
                    let mut st = span_state.borrow_mut();
                    if let Some(ref mut v) = st.start {
                        *v -= step;
                    }
                }
                KeyCode::Char('l') => {
                    let mut st = span_state.borrow_mut();
                    if let Some(ref mut v) = st.start {
                        *v += step;
                    }
                }
                KeyCode::Char('H') => {
                    let mut st = span_state.borrow_mut();
                    if let Some(ref mut v) = st.end {
                        *v -= step;
                    }
                }
                KeyCode::Char('L') => {
                    let mut st = span_state.borrow_mut();
                    if let Some(ref mut v) = st.end {
                        *v += step;
                    }
                }
                KeyCode::Char('r') => {
                    let mut st = span_state.borrow_mut();
                    st.start = Some(2.0);
                    st.end = Some(5.0);
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
