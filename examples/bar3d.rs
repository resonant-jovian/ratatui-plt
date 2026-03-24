//! 3D Bar Chart example: Regional sales by quarter.
//!
//! A 4x3 grid of bars (4 regions x 3 quarters) with varying heights.
//! Interactive: arrow keys rotate, +/- zoom, mouse scroll zoom.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind, MouseEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::bar3d::Bar3DData;

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
    io::stdout().execute(crossterm::event::EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let theme = Theme::get_default();
    let mut cycle = theme.color_cycle.clone();

    // Sales data: 4 regions x 3 quarters (deterministic values)
    // Regions along x-axis, quarters along y-axis
    let region_colors = [
        cycle.next_color(), // North
        cycle.next_color(), // South
        cycle.next_color(), // East
        cycle.next_color(), // West
    ];

    // Sales figures (in thousands): [region][quarter]
    let sales = [
        [120.0, 145.0, 165.0], // North: steady growth
        [90.0, 110.0, 85.0],   // South: dip in Q3
        [200.0, 180.0, 210.0], // East: strong overall
        [75.0, 95.0, 130.0],   // West: rapid growth
    ];

    let mut bars = Vec::new();
    for (r, row) in sales.iter().enumerate() {
        for (q, &height) in row.iter().enumerate() {
            bars.push(
                Bar3DData::new(r as f64 * 1.5, q as f64 * 1.5, height)
                    .color(region_colors[r])
                    .width(0.9),
            );
        }
    }

    let chart = Bar3D::new(bars)
        .camera(Camera3D::new().azimuth(-50.0).elevation(25.0))
        .title("3D Bar Chart: Regional Sales - Arrow keys: rotate, +/-: zoom, q: quit");

    let mut camera_state = Camera3DState::default();

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());
            frame.render_stateful_widget(&chart, area, &mut camera_state);
        })?;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Left => camera_state.rotate(-5.0, 0.0),
                KeyCode::Right => camera_state.rotate(5.0, 0.0),
                KeyCode::Up => camera_state.rotate(0.0, 5.0),
                KeyCode::Down => camera_state.rotate(0.0, -5.0),
                KeyCode::Char('+') | KeyCode::Char('=') => camera_state.zoom(1.2),
                KeyCode::Char('-') => camera_state.zoom(0.8),
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => camera_state.zoom(1.2),
                MouseEventKind::ScrollDown => camera_state.zoom(0.8),
                _ => {}
            },
            _ => {}
        }
    }

    io::stdout().execute(crossterm::event::DisableMouseCapture)?;
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
