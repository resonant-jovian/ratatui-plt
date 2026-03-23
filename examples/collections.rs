//! Collections example: LineCollection and PathCollection with Braille rendering.
//!
//! Left: a radial burst of 12 colored line segments from the center.
//! Right: a path collection drawing three distinct closed polygons.

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

    // Build a line collection: 12 radial segments, each with a distinct color
    let segment_colors = [
        Color::Red,
        Color::Rgb(255, 127, 0), // orange
        Color::Yellow,
        Color::Rgb(127, 255, 0), // lime
        Color::Green,
        Color::Rgb(0, 255, 127), // spring
        Color::Cyan,
        Color::Rgb(0, 127, 255), // azure
        Color::Blue,
        Color::Rgb(127, 0, 255), // violet
        Color::Magenta,
        Color::Rgb(255, 0, 127), // rose
    ];
    let mut lc = LineCollection::new();
    for (i, &color) in segment_colors.iter().enumerate() {
        let theta = i as f64 * std::f64::consts::TAU / 12.0;
        let x0 = 0.3 * theta.cos();
        let y0 = 0.3 * theta.sin();
        let x1 = 2.5 * theta.cos();
        let y1 = 2.5 * theta.sin();
        lc = lc.segment((x0, y0), (x1, y1), color);
    }

    // Build a path collection: triangle, square, hexagon
    let mut pc = PathCollection::new();
    let shapes: [(usize, f64, Color); 3] = [
        (3, 1.0, Color::Cyan),    // triangle
        (4, 1.8, Color::Yellow),  // square
        (6, 2.5, Color::Magenta), // hexagon
    ];
    for (n_sides, radius, color) in shapes {
        let verts: Vec<(f64, f64)> = (0..=n_sides)
            .map(|i| {
                let theta = i as f64 * std::f64::consts::TAU / n_sides as f64;
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
            let area = square_area(frame.area());
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
