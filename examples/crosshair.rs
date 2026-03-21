//! Interactive crosshair overlay on a galaxy-like scatter plot.
//!
//! Arrow keys move the crosshair in data space. The current data coordinates
//! are displayed next to the cursor. The scatter pattern shows a spiral
//! galaxy with a central bulge, two arms, and a sparse halo.

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

    // Generate deterministic scatter data simulating a galaxy-like spiral pattern
    // with a central bulge, two spiral arms, and scattered halo stars.
    let mut points = Vec::new();
    let mut color_vals = Vec::new();

    // Central bulge: dense cluster at origin with a 2D pseudo-Gaussian spread
    for i in 0..100 {
        let t = i as f64 * 0.063;
        let r = 0.8 * (1.0 - (t * 0.5).cos().abs()) + 0.1 * (t * 3.7).sin();
        let angle = t * 7.3;
        let x = 5.0 + r * angle.cos();
        let y = 5.0 + r * angle.sin();
        points.push((x, y));
        color_vals.push(0.9); // bright yellow/white core
    }

    // Spiral arm 1: logarithmic spiral with scatter
    for i in 0..120 {
        let t = i as f64 * 0.05;
        let r = 0.8 + 1.8 * t;
        let angle = t * 2.5 + 0.3;
        let scatter_r = 0.3 + 0.15 * (t * 4.1).sin();
        let scatter_a = (i as f64 * 0.17).sin() * 0.2;
        let x = 5.0 + (r + scatter_r) * (angle + scatter_a).cos();
        let y = 5.0 + (r + scatter_r) * (angle + scatter_a).sin();
        points.push((x, y));
        color_vals.push(0.3 + 0.2 * (t / 6.0)); // blue-ish arm
    }

    // Spiral arm 2: opposite side, tighter winding
    for i in 0..120 {
        let t = i as f64 * 0.05;
        let r = 0.8 + 1.8 * t;
        let angle = t * 2.5 + std::f64::consts::PI + 0.1;
        let scatter_r = 0.25 + 0.2 * (t * 3.3).cos();
        let scatter_a = (i as f64 * 0.23).cos() * 0.15;
        let x = 5.0 + (r + scatter_r) * (angle + scatter_a).cos();
        let y = 5.0 + (r + scatter_r) * (angle + scatter_a).sin();
        points.push((x, y));
        color_vals.push(0.15 + 0.2 * (t / 6.0)); // cyan arm
    }

    // Halo: sparse outer stars
    for i in 0..60 {
        let t = i as f64 * 0.105;
        let r = 5.0 + 2.5 * (t * 0.7).sin().abs() + 1.0 * (t * 1.9).cos();
        let angle = t * 4.1;
        let x = 5.0 + r * angle.cos();
        let y = 5.0 + r * angle.sin();
        points.push((x, y));
        color_vals.push(0.6 + 0.1 * (t * 0.3).sin()); // reddish halo
    }

    let series = Series::new("galaxy")
        .data(points)
        .marker(MarkerShape::FilledCircle);

    let x_axis = Axis::new().label("RA offset (kpc)").grid(true);
    let y_axis = Axis::new().label("Dec offset (kpc)").grid(true);

    // Crosshair position in data coordinates, start at galactic center
    let mut cursor_x = 5.0_f64;
    let mut cursor_y = 5.0_f64;
    let step = 0.2;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            // Render the scatter plot into the frame buffer using PlotFrame directly
            // so we can obtain the PlotArea for the crosshair overlay.
            let theme = Theme::get_default();
            let pf = PlotFrame::new(&x_axis, &y_axis, &theme)
                .title(Some("Galaxy Crosshair (arrows to move, q to quit)"));

            // Compute data bounds
            let x_lo = -4.0_f64;
            let x_hi = 14.0_f64;
            let y_lo = -4.0_f64;
            let y_hi = 14.0_f64;

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
