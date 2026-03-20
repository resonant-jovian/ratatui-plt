//! Line plot example: step modes, reference lines, and annotations.
//!
//! Shows three series: regular sin(x), step-pre cos(x), and step-post
//! clipped tan(x). Includes a y=0 reference line and peak annotations.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::line_plot::StepMode;

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

    let n = 200;

    // Regular sin(x) with error band
    let sin_data: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let x = i as f64 * 0.05;
            (x, x.sin())
        })
        .collect();
    let sin_err: Vec<f64> = (0..n)
        .map(|i| 0.1 + 0.05 * (i as f64 * 0.05).abs().sin())
        .collect();

    let sin_series = Series::new("sin(x)")
        .data(sin_data)
        .color(Color::Cyan)
        .y_err(sin_err);

    // Step-pre cos(x) - sampled at fewer points for visible steps
    let step_n = 40;
    let cos_step_data: Vec<(f64, f64)> = (0..step_n)
        .map(|i| {
            let x = i as f64 * 0.25;
            (x, x.cos())
        })
        .collect();

    let cos_series = Series::new("cos(x) [step-pre]")
        .data(cos_step_data)
        .color(Color::Yellow)
        .marker(MarkerShape::Diamond);

    // Step-post tan(x) clipped to [-2, 2] - sampled at fewer points
    let tan_step_data: Vec<(f64, f64)> = (0..step_n)
        .map(|i| {
            let x = i as f64 * 0.25;
            let y = x.tan().clamp(-2.0, 2.0);
            (x, y)
        })
        .collect();

    let tan_series = Series::new("tan(x) clipped [step-post]")
        .data(tan_step_data)
        .color(Color::Magenta)
        .marker(MarkerShape::Triangle);

    // Find the first sin peak for annotation (near x = PI/2)
    let peak_x = std::f64::consts::FRAC_PI_2;
    let peak_y = 1.0;

    let plot = LinePlot::new()
        .series(sin_series)
        .series(cos_series.clone())
        .series(tan_series.clone())
        .title("Step Modes & Annotations (q to quit)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight)
        .step_mode(StepMode::None)
        .reference_line(ReferenceLine::hline_dashed(0.0, Color::DarkGray))
        .annotation(
            Annotation::new("sin peak", peak_x + 0.5, peak_y + 0.3)
                .arrow_to(peak_x, peak_y)
                .color(Color::Cyan),
        )
        .annotation(
            Annotation::new("cos=0", std::f64::consts::FRAC_PI_2 + 0.5, -0.4)
                .arrow_to(std::f64::consts::FRAC_PI_2, 0.0)
                .color(Color::Yellow),
        );

    // Also create step-pre and step-post variants for a split layout
    let plot_step_pre = LinePlot::new()
        .series(cos_series)
        .title("Step-Pre: cos(x)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y"))
        .step_mode(StepMode::Pre)
        .reference_line(ReferenceLine::hline_dashed(0.0, Color::DarkGray))
        .show_legend(true);

    let plot_step_post = LinePlot::new()
        .series(tan_series)
        .title("Step-Post: tan(x) clipped")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y"))
        .step_mode(StepMode::Post)
        .reference_line(ReferenceLine::hline_dashed(0.0, Color::DarkGray))
        .show_legend(true);

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(area);

            let bottom_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[1]);

            frame.render_widget(&plot, rows[0]);
            frame.render_widget(&plot_step_pre, bottom_cols[0]);
            frame.render_widget(&plot_step_post, bottom_cols[1]);
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
