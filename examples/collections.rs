//! Collections example: LineCollection and PathCollection for batch rendering.
//!
//! Left: a colorful line collection (100 random segments colored by angle).
//! Right: a path collection drawing several closed polygons.

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

    // Build a line collection: radial burst of colored segments from center
    let mut lc = LineCollection::new();
    for i in 0..60 {
        let theta = i as f64 * std::f64::consts::TAU / 60.0;
        let r_inner = 0.5;
        let r_outer = 2.0 + 0.8 * (3.0 * theta).sin();
        let x0 = r_inner * theta.cos();
        let y0 = r_inner * theta.sin();
        let x1 = r_outer * theta.cos();
        let y1 = r_outer * theta.sin();
        // Color by angle using HSV-like mapping
        let hue = i as f64 / 60.0;
        let r = ((hue * 6.0).sin() * 127.0 + 128.0) as u8;
        let g = (((hue * 6.0 + 2.0).sin()) * 127.0 + 128.0) as u8;
        let b = (((hue * 6.0 + 4.0).sin()) * 127.0 + 128.0) as u8;
        lc = lc.segment((x0, y0), (x1, y1), Color::Rgb(r, g, b));
    }

    // Build a path collection: concentric regular polygons
    let mut pc = PathCollection::new();
    let colors = [
        Color::Cyan,
        Color::Yellow,
        Color::Magenta,
        Color::Green,
        Color::Red,
    ];
    for (k, &color) in colors.iter().enumerate() {
        let n_sides = k + 3; // triangle, square, pentagon, hexagon, heptagon
        let radius = 0.5 + k as f64 * 0.5;
        let verts: Vec<(f64, f64)> = (0..=n_sides)
            .map(|i| {
                let theta = i as f64 * std::f64::consts::TAU / n_sides as f64 + k as f64 * 0.2; // slight rotation
                (radius * theta.cos(), radius * theta.sin())
            })
            .collect();
        pc = pc.path(verts, color, true);
    }

    // Wrap them in LinePlot frames so we get axes
    let lc_frame_plot = LinePlot::new()
        .title("LineCollection: Radial Burst (q to quit)")
        .x_axis(Axis::new().bounds(Bounds::Manual(-3.5, 3.5)).grid(true))
        .y_axis(Axis::new().bounds(Bounds::Manual(-3.5, 3.5)).grid(true))
        .show_legend(false);

    let pc_frame_plot = LinePlot::new()
        .title("PathCollection: Nested Polygons")
        .x_axis(Axis::new().bounds(Bounds::Manual(-3.5, 3.5)).grid(true))
        .y_axis(Axis::new().bounds(Bounds::Manual(-3.5, 3.5)).grid(true))
        .show_legend(false);

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            // Render the frame (axes/grid) then overlay collections
            let buf = frame.buffer_mut();

            // Left: LineCollection
            (&lc_frame_plot).render(cols[0], buf);
            // Compute a PlotArea manually for the collection
            let pa_left = PlotArea {
                x: cols[0].x + 8,
                y: cols[0].y + 1,
                width: cols[0].width.saturating_sub(10),
                height: cols[0].height.saturating_sub(3),
                x_lo: -3.5,
                x_hi: 3.5,
                y_lo: -3.5,
                y_hi: 3.5,
                area: cols[0],
            };
            lc.render(&pa_left, buf);

            // Right: PathCollection
            (&pc_frame_plot).render(cols[1], buf);
            let pa_right = PlotArea {
                x: cols[1].x + 8,
                y: cols[1].y + 1,
                width: cols[1].width.saturating_sub(10),
                height: cols[1].height.saturating_sub(3),
                x_lo: -3.5,
                x_hi: 3.5,
                y_lo: -3.5,
                y_hi: 3.5,
                area: cols[1],
            };
            pc.render(&pa_right, buf);
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
