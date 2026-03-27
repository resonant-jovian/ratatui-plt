//! Heatmap example: multi-panel layout with a large 2D Gaussian heatmap and
//! a small correlation matrix with show_values and triangular mask.

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
    // Auto-detect terminal cell aspect ratio for accurate equal aspect plots
    if let Ok(size) = crossterm::terminal::window_size()
        && size.width > 0
        && size.height > 0
        && size.columns > 0
        && size.rows > 0
    {
        let cell_w = size.width as f64 / size.columns as f64;
        let cell_h = size.height as f64 / size.rows as f64;
        set_cell_aspect(cell_w / cell_h);
    }

    // Left panel: large 2D Gaussian heatmap
    let data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 500, 500, |x, y| {
        (-(x * x + y * y)).exp()
    });

    let heatmap = Heatmap::new(data)
        .colormap(Viridis)
        .title("2D Gaussian  z = exp(-(x\u{00b2} + y\u{00b2}))")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"))
        .show_colorbar(true)
        .aspect_ratio(AspectRatio::Equal);

    // Right panel: 8x8 correlation matrix with show_values and upper-triangular mask
    let size = 8;
    let labels = ["A", "B", "C", "D", "E", "F", "G", "H"];
    let _ = labels; // Labels for context; actual axis ticks use numeric coords

    // Generate a deterministic pseudo-correlation matrix (symmetric).
    // Compute upper triangle pairs, then scatter into the matrix.
    let mut pairs: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..size {
        for j in (i + 1)..size {
            let seed = (i * 31 + j * 17) as f64;
            let val = ((seed * 0.7).sin() * 0.6 + (seed * 1.3).cos() * 0.3).clamp(-0.95, 0.95);
            pairs.push((i, j, val));
        }
    }
    let mut corr = vec![vec![0.0_f64; size]; size];
    for (i, row) in corr.iter_mut().enumerate() {
        row[i] = 1.0; // diagonal
    }
    for &(i, j, val) in &pairs {
        corr[i][j] = val;
        corr[j][i] = val;
    }

    // Build GridData from the correlation matrix
    let x_coords: Vec<f64> = (0..size).map(|i| i as f64).collect();
    let y_coords: Vec<f64> = (0..size).map(|i| i as f64).collect();
    let corr_grid = GridData {
        x: x_coords,
        y: y_coords,
        values: corr,
    };

    // Upper-triangular mask: hide cells where row < col
    let mask: Vec<Vec<bool>> = (0..size)
        .map(|row| (0..size).map(|col| row < col).collect())
        .collect();

    let corr_heatmap = Heatmap::new(corr_grid)
        .colormap(Coolwarm)
        .title("Correlation Matrix (lower triangle)")
        .show_colorbar(true)
        .show_values(true)
        .mask(mask);

    if headless_export(|area, buf| {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);
        (&heatmap).render(cols[0], buf);
        (&corr_heatmap).render(cols[1], buf);
    })? {
        return Ok(());
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());

            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(area);

            frame.render_widget(&heatmap, cols[0]);
            frame.render_widget(&corr_heatmap, cols[1]);
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
