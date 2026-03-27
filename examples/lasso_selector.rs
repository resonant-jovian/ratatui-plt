//! Lasso selector example: build a freehand polygon selection on a scatter plot.
//!
//! Press 1-6 to add predefined polygon vertices, c to close the polygon,
//! r to reset. Press q or Esc to quit.

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

    // Generate scatter data: ring of points plus inner cluster
    let mut scatter_points: Vec<(f64, f64)> = Vec::new();
    for i in 0..50 {
        let angle = i as f64 * 0.126;
        let r = 3.5 + (i as f64 * 0.3).sin() * 0.5;
        scatter_points.push((5.0 + r * angle.cos(), 5.0 + r * angle.sin()));
    }
    for i in 0..30 {
        let angle = i as f64 * 0.21;
        let r = 1.0 + (i as f64 * 0.5).cos() * 0.3;
        scatter_points.push((5.0 + r * angle.cos(), 5.0 + r * angle.sin()));
    }

    let scatter = Series::new("points")
        .data(scatter_points)
        .color(theme.primary);

    // Lasso state
    let mut lasso_state = LassoSelectorState::new();

    // Predefined polygon vertices for keyboard-driven selection
    let preset_vertices: [(f64, f64); 6] = [
        (3.0, 3.0),
        (7.0, 3.0),
        (8.0, 5.5),
        (7.0, 8.0),
        (3.0, 8.0),
        (2.0, 5.5),
    ];
    let mut next_vertex: usize = 0;

    let lasso = LassoSelector::new()
        .x_axis(
            Axis::new()
                .label("x")
                .bounds(Bounds::Manual(0.0, 10.0))
                .grid(true),
        )
        .y_axis(
            Axis::new()
                .label("y")
                .bounds(Bounds::Manual(0.0, 10.0))
                .grid(true),
        )
        .line_color(theme.accent)
        .fill_closed(true)
        .show_vertices(true);

    loop {
        // Count selected points
        let selected_count = scatter
            .data
            .iter()
            .filter(|&&(x, y)| lasso_state.contains(x, y))
            .count();

        terminal.draw(|frame| {
            let area = square_area(frame.area());

            // Render the scatter plot as a base layer
            let buf = frame.buffer_mut();
            let x_axis = Axis::new()
                .label("x")
                .bounds(Bounds::Manual(0.0, 10.0))
                .grid(true);
            let y_axis = Axis::new()
                .label("y")
                .bounds(Bounds::Manual(0.0, 10.0))
                .grid(true);

            let status = if lasso_state.closed {
                format!(
                    "Lasso (closed, {} selected, r: reset, q: quit)",
                    selected_count
                )
            } else {
                format!(
                    "Lasso ({} pts, 1-6: add vertex, c: close, r: reset, q: quit)",
                    lasso_state.points.len()
                )
            };

            let theme = Theme::get_default();
            let pf = PlotFrame::new(&x_axis, &y_axis, &theme).title(Some(&status));

            if let Some(pa) = pf.render(
                area,
                buf,
                DataBounds {
                    x_lo: 0.0,
                    x_hi: 10.0,
                    y_lo: 0.0,
                    y_hi: 10.0,
                },
            ) {
                // Draw scatter points, highlighting selected ones
                let theme = Theme::get_default();
                for &(x, y) in &scatter.data {
                    let sx = pa.screen_x(x).round() as u16;
                    let sy = pa.screen_y(y).round() as u16;
                    if pa.contains(sx, sy) {
                        let (ch, color) = if lasso_state.contains(x, y) {
                            ('O', theme.highlight)
                        } else {
                            ('.', scatter.color.unwrap_or(theme.foreground))
                        };
                        buf[(sx, sy)].set_char(ch).set_fg(color);
                    }
                }
            }

            // Overlay lasso widget using StatefulWidget
            frame.render_stateful_widget(&lasso, area, &mut lasso_state);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('1') => {
                    if !lasso_state.closed {
                        lasso_state.add_point(preset_vertices[0].0, preset_vertices[0].1);
                        next_vertex = 1;
                    }
                }
                KeyCode::Char('2') => {
                    if !lasso_state.closed && next_vertex >= 1 {
                        lasso_state.add_point(preset_vertices[1].0, preset_vertices[1].1);
                        next_vertex = 2;
                    }
                }
                KeyCode::Char('3') => {
                    if !lasso_state.closed && next_vertex >= 2 {
                        lasso_state.add_point(preset_vertices[2].0, preset_vertices[2].1);
                        next_vertex = 3;
                    }
                }
                KeyCode::Char('4') => {
                    if !lasso_state.closed && next_vertex >= 3 {
                        lasso_state.add_point(preset_vertices[3].0, preset_vertices[3].1);
                        next_vertex = 4;
                    }
                }
                KeyCode::Char('5') => {
                    if !lasso_state.closed && next_vertex >= 4 {
                        lasso_state.add_point(preset_vertices[4].0, preset_vertices[4].1);
                        next_vertex = 5;
                    }
                }
                KeyCode::Char('6') => {
                    if !lasso_state.closed && next_vertex >= 5 {
                        lasso_state.add_point(preset_vertices[5].0, preset_vertices[5].1);
                        next_vertex = 6;
                    }
                }
                KeyCode::Char('c') => {
                    lasso_state.close();
                }
                KeyCode::Char('r') => {
                    lasso_state.reset();
                    next_vertex = 0;
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
