//! Rug plot example: Earthquake magnitudes with histogram and KDE overlay.
//!
//! A combined visualization showing:
//! - Histogram of earthquake magnitudes in the main plot area
//! - KDE (kernel density estimate) curve overlaid on the histogram
//! - Rug ticks along the bottom showing individual observations
//!
//! All magnitudes are deterministic constants (no random generation needed).

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::histogram::HistNorm;

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

/// Simple Gaussian KDE: returns (x, density) pairs for plotting.
fn gaussian_kde(data: &[f64], bandwidth: f64, n_points: usize) -> Vec<(f64, f64)> {
    if data.is_empty() {
        return Vec::new();
    }
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let pad = 3.0 * bandwidth;
    let lo = min - pad;
    let hi = max + pad;
    let step = (hi - lo) / (n_points - 1) as f64;
    let n = data.len() as f64;
    let norm_factor = 1.0 / (n * bandwidth * (2.0 * std::f64::consts::PI).sqrt());

    (0..n_points)
        .map(|i| {
            let x = lo + i as f64 * step;
            let density: f64 = data
                .iter()
                .map(|&xi| {
                    let z = (x - xi) / bandwidth;
                    (-0.5 * z * z).exp()
                })
                .sum::<f64>()
                * norm_factor;
            (x, density)
        })
        .collect()
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Deterministic earthquake magnitude dataset (~56 values in Richter scale range 2-8)
    // Inspired by realistic Gutenberg-Richter magnitude-frequency distribution:
    // many small quakes, exponentially fewer large ones
    let magnitudes: Vec<f64> = vec![
        2.1, 2.1, 2.2, 2.3, 2.3, 2.4, 2.4, 2.5, 2.5, 2.6, 2.6, 2.7, 2.7, 2.8, 2.9, 2.9, 3.0,
        3.0, 3.1, 3.1, 3.2, 3.2, 3.3, 3.3, 3.4, 3.5, 3.5, 3.6, 3.7, 3.8, 3.9, 4.0, 4.1, 4.2,
        4.3, 4.4, 4.5, 4.7, 4.9, 5.0, 5.1, 5.3, 5.5, 5.6, 5.8, 6.0, 6.2, 6.5, 6.7, 7.0, 7.1,
        7.3, 7.5, 7.8, 8.0, 8.1,
    ];

    // Compute KDE curve
    let kde_points = gaussian_kde(&magnitudes, 0.3, 200);
    let kde_y_max = kde_points
        .iter()
        .map(|&(_, y)| y)
        .fold(0.0_f64, f64::max);

    // Shared axis bounds for histogram and KDE overlay
    let x_lo = 1.5;
    let x_hi = 9.0;
    let y_lo = 0.0;
    let y_hi = kde_y_max * 1.1;

    // Build density-normalized histogram
    let hist = Histogram::new(magnitudes.clone())
        .bins(20)
        .norm_mode(HistNorm::Density)
        .color(Color::Rgb(80, 120, 180))
        .title("Earthquake Magnitudes: Histogram + KDE + Rug (q to quit)")
        .x_axis(
            Axis::new()
                .label("Magnitude (Richter)")
                .bounds(Bounds::Manual(x_lo, x_hi))
                .grid(true),
        )
        .y_axis(
            Axis::new()
                .label("Density")
                .bounds(Bounds::Manual(y_lo, y_hi))
                .grid(true),
        );

    // Build rug plot for individual observations
    let rug_ds = RugDataset::new("Earthquakes", magnitudes, Color::Red);
    let rug = RugPlot::new()
        .dataset(rug_ds)
        .side(RugSide::Bottom)
        .height(2)
        .x_axis(
            Axis::new()
                .label("Magnitude (Richter)")
                .bounds(Bounds::Manual(x_lo, x_hi))
                .grid(true),
        )
        .y_axis(Axis::new());

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            // Split into main plot area (80%) and rug strip (20%)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .split(area);

            // Draw histogram in the main area
            frame.render_widget(&hist, chunks[0]);

            // Overlay KDE curve on top of the histogram.
            // We compute a PlotArea matching the histogram's layout to map
            // data coordinates to screen positions without rendering a second
            // set of axis tick labels (which would overwrite the histogram's).
            let hist_area = chunks[0];
            let y_label_width: u16 = 8;
            let title_height: u16 = 1; // histogram has a title
            let tick_height: u16 = 1;
            let x_label_height: u16 = 1; // histogram has an x-axis label

            let plot_x = hist_area.x + y_label_width;
            let plot_y = hist_area.y + title_height;
            let plot_width = hist_area
                .width
                .saturating_sub(y_label_width + 1);
            let plot_height = hist_area
                .height
                .saturating_sub(title_height + tick_height + x_label_height);

            if plot_width >= 2 && plot_height >= 2 {
                let pa = PlotArea {
                    x: plot_x,
                    y: plot_y,
                    width: plot_width,
                    height: plot_height,
                    x_lo,
                    x_hi,
                    y_lo,
                    y_hi,
                    area: hist_area,
                };

                let buf = frame.buffer_mut();
                // Draw KDE line by connecting consecutive points with Bresenham lines
                for pair in kde_points.windows(2) {
                    let (kx0, ky0) = pair[0];
                    let (kx1, ky1) = pair[1];

                    let sx0 = pa.screen_x(kx0).round() as i32;
                    let sy0 = pa.screen_y(ky0).round() as i32;
                    let sx1 = pa.screen_x(kx1).round() as i32;
                    let sy1 = pa.screen_y(ky1).round() as i32;

                    let dx = (sx1 - sx0).abs();
                    let dy = (sy1 - sy0).abs();
                    let steps = dx.max(dy).max(1);
                    for s in 0..=steps {
                        let t = s as f64 / steps as f64;
                        let px = (sx0 as f64 + t * (sx1 - sx0) as f64).round() as u16;
                        let py = (sy0 as f64 + t * (sy1 - sy0) as f64).round() as u16;
                        if pa.contains(px, py) {
                            buf[(px, py)].set_char('*').set_fg(Color::Yellow);
                        }
                    }
                }
            }

            // Draw rug ticks in the bottom strip
            frame.render_widget(&rug, chunks[1]);
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
