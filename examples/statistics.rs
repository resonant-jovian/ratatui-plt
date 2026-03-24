//! Statistics example: regression, LOWESS smoothing, and bootstrap CI.
//!
//! Generates noisy sine data, fits linear and quadratic regressions,
//! computes LOWESS smoothing, and shows bootstrap confidence intervals
//! as a band plot.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::statistics::{
    Kde, bootstrap_ci, linear_regression, lowess, mean_estimator, poly_fit,
};

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

/// Simple PRNG for reproducible noisy data (SplitMix64).
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z = z ^ (z >> 31);
        (z as f64) / (u64::MAX as f64)
    }

    /// Approximate normal(0,1) via sum of 12 uniforms minus 6.
    fn next_normal(&mut self) -> f64 {
        let mut sum = 0.0;
        for _ in 0..12 {
            sum += self.next_f64();
        }
        sum - 6.0
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut rng = Rng::new(42);

    // Generate noisy sine data: y = sin(x) + noise
    let n = 80;
    let x_data: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    let y_data: Vec<f64> = x_data
        .iter()
        .map(|&x| x.sin() + 0.3 * rng.next_normal())
        .collect();

    let theme = Theme::get_default();

    // Scatter series for raw data
    let scatter = Series::new("Data")
        .data(x_data.iter().copied().zip(y_data.iter().copied()).collect())
        .color(theme.foreground)
        .marker(MarkerShape::Circle);

    // Linear regression overlay
    let lin_fit = linear_regression(&x_data, &y_data);
    let lin_series = if let Some(ref fit) = lin_fit {
        let fit_data: Vec<(f64, f64)> = x_data
            .iter()
            .map(|&x| (x, fit.slope * x + fit.intercept))
            .collect();
        Some(
            Series::new(format!("Linear (R\u{00b2}={:.3})", fit.r_squared))
                .data(fit_data)
                .color(theme.negative_color),
        )
    } else {
        None
    };

    // Quadratic fit overlay
    let quad_fit = poly_fit(&x_data, &y_data, 2);
    let quad_series = if let Some(ref fit) = quad_fit {
        Some(
            fit.to_series(
                x_data[0],
                x_data[x_data.len() - 1],
                200,
                &format!("Quadratic (R\u{00b2}={:.3})", fit.r_squared),
            )
            .color(theme.highlight),
        )
    } else {
        None
    };

    // LOWESS smoothing
    let lowess_result = lowess(&x_data, &y_data, 0.3);
    let lowess_series = lowess_result
        .as_ref()
        .map(|res| res.to_series("LOWESS (f=0.3)", theme.positive_color));

    // Build the regression plot
    let mut plot = LinePlot::new()
        .series(scatter)
        .title("Regression & Smoothing (q to quit)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

    if let Some(s) = lin_series {
        plot = plot.series(s);
    }
    if let Some(s) = quad_series {
        plot = plot.series(s);
    }
    if let Some(s) = lowess_series {
        plot = plot.series(s);
    }

    // Bootstrap CI for the mean at sliding windows -> shown as a band
    let window: usize = 15;
    let mut band_x = Vec::new();
    let mut band_lo = Vec::new();
    let mut band_hi = Vec::new();
    for (i, &xv) in x_data.iter().enumerate() {
        let start = i.saturating_sub(window / 2);
        let end = (i + window / 2 + 1).min(n);
        let slice = &y_data[start..end];
        let ci = bootstrap_ci(slice, mean_estimator, 200, 0.95, i as u64);
        band_x.push(xv);
        band_lo.push(ci.lower);
        band_hi.push(ci.upper);
    }

    let ci_band = Band::new("95% CI (mean)", band_x.clone(), band_lo, band_hi)
        .color(theme.primary)
        .alpha_char('\u{2591}');

    let band_plot = BandPlot::new()
        .band(ci_band)
        .title("Bootstrap 95% CI for Windowed Mean")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(true);

    // KDE of the y-values
    let kde = Kde::new().n_points(100);
    let (kde_x, kde_y) = kde.fit(&y_data);
    let kde_series = Series::new("KDE")
        .data(kde_x.into_iter().zip(kde_y).collect())
        .color(theme.accent);

    let kde_plot = LinePlot::new()
        .series(kde_series)
        .title("KDE of y-values")
        .x_axis(Axis::new().label("y").grid(true))
        .y_axis(
            Axis::new()
                .label("Density")
                .label_position(LabelPosition::End)
                .grid(true),
        )
        .show_legend(true);

    loop {
        terminal.draw(|frame| {
            let area = square_area(frame.area());

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(area);

            let bottom_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[1]);

            frame.render_widget(&plot, rows[0]);
            frame.render_widget(&band_plot, bottom_cols[0]);
            frame.render_widget(&kde_plot, bottom_cols[1]);
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
