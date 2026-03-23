//! Scientific dashboard example: four-panel layout with enhanced features.
//!
//! - Top-left: Energy tracking line plot with reference lines and annotations
//! - Top-right: Band plot showing confidence intervals
//! - Bottom-left: Density heatmap with Inferno colormap
//! - Bottom-right: ECDF comparison of two distributions

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

    // --- Panel 1: Energy tracking line plot with references and annotations ---
    let energy_data: Vec<(f64, f64)> = (0..200)
        .map(|i| {
            let t = i as f64 * 0.05;
            let energy = 10.0 * (-0.03 * t).exp() * (2.0 * t).cos().powi(2) + 0.5;
            (t, energy)
        })
        .collect();

    let energy_plot = LinePlot::new()
        .series(
            Series::new("Kinetic Energy")
                .data(energy_data)
                .color(Color::Cyan),
        )
        .title("Energy vs Time")
        .x_axis(Axis::new().label("t").grid(true))
        .y_axis(Axis::new().label("E").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight)
        .spines(Spines::new().top(false).right(false))
        .reference_line(ReferenceLine::hline_dashed(0.5, Color::DarkGray))
        .reference_line(ReferenceLine::hspan(0.0, 0.5, Color::Rgb(40, 20, 20)))
        .annotation(
            Annotation::new("equilibrium", 7.0, 1.5)
                .arrow_to(8.0, 0.5)
                .color(Color::Yellow),
        );

    // --- Panel 2: Band plot for confidence intervals ---
    let band_n = 100;
    let band_x: Vec<f64> = (0..band_n).map(|i| i as f64 * 0.1).collect();
    let band_center: Vec<f64> = band_x.iter().map(|&x| (x * 0.5).sin() * 2.0).collect();
    let band_lower_95: Vec<f64> = band_center
        .iter()
        .enumerate()
        .map(|(i, &c)| c - 0.8 - 0.2 * (i as f64 * 0.05).sin().abs())
        .collect();
    let band_upper_95: Vec<f64> = band_center
        .iter()
        .enumerate()
        .map(|(i, &c)| c + 0.8 + 0.2 * (i as f64 * 0.05).sin().abs())
        .collect();
    let band_lower_50: Vec<f64> = band_center
        .iter()
        .enumerate()
        .map(|(i, &c)| c - 0.3 - 0.1 * (i as f64 * 0.07).cos().abs())
        .collect();
    let band_upper_50: Vec<f64> = band_center
        .iter()
        .enumerate()
        .map(|(i, &c)| c + 0.3 + 0.1 * (i as f64 * 0.07).cos().abs())
        .collect();

    let band_plot = BandPlot::new()
        .band(
            Band::new("95% CI", band_x.clone(), band_lower_95, band_upper_95)
                .color(Color::Rgb(60, 60, 140))
                .alpha_char('\u{2591}'),
        )
        .band(
            Band::new("50% CI", band_x, band_lower_50, band_upper_50)
                .color(Color::Rgb(80, 80, 200))
                .alpha_char('\u{2592}'),
        )
        .title("Prediction Interval")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight)
        .spines(Spines::new().top(false).right(false))
        .reference_line(ReferenceLine::hline_dashed(0.0, Color::DarkGray));

    // --- Panel 3: Density heatmap ---
    let density = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 300, 300, |x, y| {
        let r2 = x * x + y * y;
        (-r2 / 2.0).exp() + 0.3 * (-(((x - 1.0).powi(2) + (y - 1.0).powi(2)) / 0.5)).exp()
    });

    let heatmap = Heatmap::new(density)
        .colormap(Inferno)
        .title("Density Field")
        .show_colorbar(true)
        .aspect_ratio(AspectRatio::Equal)
        .spines(Spines::all(true));

    // --- Panel 4: ECDF comparison ---
    let ecdf_n = 300;

    // Pseudo-Gaussian via sum of sines
    let gaussian_data: Vec<f64> = (0..ecdf_n)
        .map(|i| {
            let t = i as f64 * 0.1;
            let sum = (t * 1.0).sin()
                + (t * std::f64::consts::SQRT_2).sin()
                + (t * std::f64::consts::PI).sin()
                + (t * std::f64::consts::E).sin()
                + (t * 2.2360679).sin()
                + (t * 3.3166248).sin();
            sum * 0.5
        })
        .collect();

    // Pseudo-Exponential via LCG reciprocal
    let exponential_data: Vec<f64> = {
        let mut state: u64 = 12345;
        (0..ecdf_n)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let u = ((state >> 33) as f64) / (u32::MAX as f64);
                let u_safe = u.max(0.001);
                -u_safe.ln() * 1.5
            })
            .collect()
    };

    let ecdf_plot = EcdfPlot::new()
        .dataset(EcdfDataset::new("Gaussian", gaussian_data, Color::Cyan))
        .dataset(EcdfDataset::new(
            "Exponential",
            exponential_data,
            Color::Yellow,
        ))
        .title("ECDF Comparison")
        .x_axis(Axis::new().label("Value").grid(true))
        .y_axis(Axis::new().label("F(x)").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::BottomRight)
        .spines(Spines::new().top(false).right(false))
        .reference_line(ReferenceLine::hline_dashed(0.5, Color::DarkGray));

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());

            // 2x2 grid layout
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            let top_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(rows[0]);

            let bottom_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[1]);

            frame.render_widget(&energy_plot, top_cols[0]);
            frame.render_widget(&band_plot, top_cols[1]);
            frame.render_widget(&heatmap, bottom_cols[0]);
            frame.render_widget(&ecdf_plot, bottom_cols[1]);
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
