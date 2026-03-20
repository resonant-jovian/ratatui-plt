//! Interactive crosshair overlay on a scatter plot.
//!
//! Arrow keys move the crosshair in data space. The current data coordinates
//! are displayed next to the cursor.

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

    // Generate deterministic scatter data: two clusters + a ring
    let mut points = Vec::new();
    let mut color_vals = Vec::new();

    // Cluster 1: centred at (2, 3)
    for i in 0..80 {
        let t = i as f64 * 0.079;
        let x = 2.0 + 1.2 * t.sin() + 0.3 * (t * 2.7).cos();
        let y = 3.0 + 0.9 * t.cos() + 0.3 * (t * 3.1).sin();
        points.push((x, y));
        color_vals.push(0.2);
    }

    // Cluster 2: centred at (7, 6)
    for i in 0..80 {
        let t = i as f64 * 0.079;
        let x = 7.0 + 1.0 * (t * 1.3).cos() + 0.4 * (t * 2.1).sin();
        let y = 6.0 + 1.1 * (t * 0.9).sin() + 0.3 * (t * 3.3).cos();
        points.push((x, y));
        color_vals.push(0.7);
    }

    // Ring pattern: centred at (5, 4)
    for i in 0..120 {
        let theta = i as f64 * std::f64::consts::TAU / 120.0;
        let r = 3.5 + 0.3 * (theta * 5.0).sin();
        let x = 5.0 + r * theta.cos();
        let y = 4.0 + r * theta.sin();
        points.push((x, y));
        color_vals.push((theta / std::f64::consts::TAU).fract());
    }

    let series = Series::new("scatter")
        .data(points)
        .marker(MarkerShape::FilledCircle);

    let x_axis = Axis::new().label("x").grid(true);
    let y_axis = Axis::new().label("y").grid(true);

    // Crosshair position in data coordinates
    let mut cursor_x = 5.0_f64;
    let mut cursor_y = 4.0_f64;
    let step = 0.2;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            // Render the scatter plot into the frame buffer using PlotFrame directly
            // so we can obtain the PlotArea for the crosshair overlay.
            let theme = Theme::get_default();
            let pf = PlotFrame::new(&x_axis, &y_axis, &theme)
                .title(Some("Interactive Crosshair (arrows to move, q to quit)"));

            // Compute data bounds
            let x_lo = 0.0_f64;
            let x_hi = 10.0_f64;
            let y_lo = -1.0_f64;
            let y_hi = 9.0_f64;

            let buf = frame.buffer_mut();
            if let Some(pa) = pf.render(area, buf, x_lo, x_hi, y_lo, y_hi) {
                // Draw scatter points with colormap
                let cmap = Viridis;
                let norm = LinearNorm::new(0.0, 1.0);
                let series_ref = &series;
                for (idx, &(x, y)) in series_ref.data.iter().enumerate() {
                    let sx = pa.screen_x(x);
                    let sy = pa.screen_y(y);
                    let xi = sx.round() as u16;
                    let yi = sy.round() as u16;
                    if pa.contains(xi, yi) {
                        let t = norm.normalize(color_vals[idx]);
                        let color = cmap.color_at(t);
                        let marker = series_ref.marker.unwrap_or(MarkerShape::Dot);
                        buf[(xi, yi)].set_char(marker.char()).set_fg(color);
                    }
                }

                // Render crosshair overlay
                let crosshair = Crosshair::new(cursor_x, cursor_y)
                    .color(Color::Yellow)
                    .show_labels(true)
                    .format(|x, y| format!("({:.1}, {:.1})", x, y));
                crosshair.render_on(&pa, buf);
            }
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Up => cursor_y += step,
                KeyCode::Down => cursor_y -= step,
                KeyCode::Right => cursor_x += step,
                KeyCode::Left => cursor_x -= step,
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
