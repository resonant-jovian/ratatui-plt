//! Showcase: Polar and ternary visualization widgets.
//!
//! 5 plots in a 2x3 mosaic grid (ABC / DE.):
//! A) RadialPlot (line), B) RadialPlot (scatter),
//! C) RadialPlot (bar/rose), D) RadialPlot (fill-between),
//! E) TernaryPlot.

use std::f64::consts::PI;
use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::radial::PolarPlotType;
use ratatui_plt::widgets::ternary::{TernaryData, TernaryPlot};

/// Simple deterministic LCG pseudo-random number generator.
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*seed >> 33) as f64 / (1u64 << 31) as f64
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(Theme::light());
    let theme = Theme::get_default();
    let c0 = theme.color_cycle.at(0);
    let c1 = theme.color_cycle.at(1);
    let c2 = theme.color_cycle.at(2);
    let c3 = theme.color_cycle.at(3);

    // ---- Panel A: RadialPlot line mode — sinusoidal polar pattern ----
    // Cardioid: r = 1 + cos(theta)
    let line_data: Vec<(f64, f64)> = (0..=360)
        .map(|i| {
            let theta = i as f64 * PI / 180.0;
            (theta, 1.0 + theta.cos())
        })
        .collect();

    let radial_line = RadialPlot::new()
        .series(Series::new("r = 1+cos(t)").data(line_data).color(c0))
        .plot_type(PolarPlotType::Line)
        .n_rings(4)
        .title("Polar Line (Cardioid)");

    // ---- Panel B: RadialPlot scatter mode — random polar points ----
    let mut seed = 42u64;
    let scatter_data: Vec<(f64, f64)> = (0..60)
        .map(|_| {
            let theta = lcg(&mut seed) * 2.0 * PI;
            let r = lcg(&mut seed) * 3.0;
            (theta, r)
        })
        .collect();

    let radial_scatter = RadialPlot::new()
        .series(
            Series::new("Random")
                .data(scatter_data)
                .color(c1)
                .marker(MarkerShape::FilledCircle),
        )
        .plot_type(PolarPlotType::Scatter)
        .n_rings(3)
        .title("Polar Scatter");

    // ---- Panel C: RadialPlot bar/rose mode — wind direction histogram ----
    // 8 sectors (N, NE, E, SE, S, SW, W, NW) with simulated wind counts
    let sector_values = [12.0, 8.0, 5.0, 3.0, 7.0, 15.0, 10.0, 6.0];
    let bar_data: Vec<(f64, f64)> = sector_values
        .iter()
        .enumerate()
        .map(|(i, &val)| {
            let theta = i as f64 * PI / 4.0;
            (theta, val)
        })
        .collect();

    let radial_bar = RadialPlot::new()
        .series(Series::new("Wind").data(bar_data).color(c2))
        .plot_type(PolarPlotType::Bar)
        .n_spokes(8)
        .n_rings(4)
        .theta_offset(PI / 2.0)
        .title("Wind Rose");

    // ---- Panel D: RadialPlot fill-between mode — filled polar region ----
    // Limacon: r = 0.5 + 1.5*cos(theta)
    let fill_data: Vec<(f64, f64)> = (0..=360)
        .map(|i| {
            let theta = i as f64 * PI / 180.0;
            (theta, (0.5 + 1.5 * theta.cos()).max(0.0))
        })
        .collect();

    let radial_fill = RadialPlot::new()
        .series(Series::new("Limacon").data(fill_data).color(c3))
        .plot_type(PolarPlotType::FillBetween)
        .n_rings(4)
        .title("Polar Fill");

    // ---- Panel E: TernaryPlot — soil texture triangle ----
    // Points: (Sand, Silt, Clay) fractions summing to 1.0
    let soil_points = vec![
        (0.40, 0.40, 0.20), // Loam
        (0.85, 0.10, 0.05), // Sand
        (0.10, 0.10, 0.80), // Clay
        (0.20, 0.65, 0.15), // Silt Loam
        (0.55, 0.25, 0.20), // Sandy Loam
        (0.10, 0.50, 0.40), // Silty Clay Loam
        (0.35, 0.30, 0.35), // Clay Loam
        (0.70, 0.15, 0.15), // Sandy Loam
        (0.25, 0.50, 0.25), // Silt Loam
        (0.50, 0.15, 0.35), // Sandy Clay Loam
    ];

    let ternary_data = TernaryData::new("Samples")
        .points(soil_points)
        .color(c0)
        .marker(MarkerShape::FilledCircle);

    let ternary = TernaryPlot::new()
        .dataset(ternary_data)
        .corner_labels("Sand", "Silt", "Clay")
        .show_grid(true)
        .grid_divisions(5)
        .title("Soil Texture");

    // ---- Assemble 2x3 mosaic: ABC / DE. ----
    let panel = MultiPanel::from_mosaic("ABC\nDE.")
        .gap(1)
        .suptitle("Polar & Ternary Plots Showcase (q to quit)")
        .mosaic_panel('A', move |area: Rect, buf: &mut Buffer| {
            (&radial_line).render(area, buf);
        })
        .mosaic_panel('B', move |area: Rect, buf: &mut Buffer| {
            (&radial_scatter).render(area, buf);
        })
        .mosaic_panel('C', move |area: Rect, buf: &mut Buffer| {
            (&radial_bar).render(area, buf);
        })
        .mosaic_panel('D', move |area: Rect, buf: &mut Buffer| {
            (&radial_fill).render(area, buf);
        })
        .mosaic_panel('E', move |area: Rect, buf: &mut Buffer| {
            (&ternary).render(area, buf);
        });

    if headless_export(|area, buf| (&panel).render(area, buf))? {
        return Ok(());
    }

    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&panel, square_area(frame.area()));
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
