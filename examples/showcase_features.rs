//! Showcase: Statistics and interactivity features in ratatui-plt.
//!
//! Demonstrates statistical and dashboard features in a 2x2 MultiPanel:
//!   A: ScatterPlot with polynomial(2) trendline overlay
//!   B: JointPlot with histogram marginals on correlated data
//!   C: WaterfallChart showing revenue breakdown
//!   D: GaugeChart showing a KPI value with colored sectors
//!
//! Requires the `statistics` feature:
//!   cargo run --features statistics --example showcase_features

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;

/// Simple deterministic LCG pseudo-random number generator.
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*seed >> 33) as f64 / (1u64 << 31) as f64
}

/// Box-Muller approximation for pseudo-normal values.
fn box_muller(seed: &mut u64, mean: f64, std: f64) -> f64 {
    let u1 = lcg(seed).max(1e-10);
    let u2 = lcg(seed);
    let val = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    val * std + mean
}

fn parse_theme() -> Theme {
    match std::env::args().nth(1).as_deref() {
        Some("dark") => Theme::dark(),
        Some("minimal") => Theme::minimal(),
        Some("publication") => Theme::publication(),
        Some("solarized") => Theme::solarized(),
        Some("light") | None => Theme::light(),
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

    let blue_dark = Color::Rgb(31, 119, 180);

    // ---- Panel A: ScatterPlot with polynomial(2) trendline ----
    let mut seed_a = 42u64;
    let n_scatter = 150;
    let scatter_data: Vec<(f64, f64)> = (0..n_scatter)
        .map(|_| {
            let x = lcg(&mut seed_a) * 6.0 - 3.0;
            let noise = box_muller(&mut seed_a, 0.0, 1.5);
            let y = 0.5 * x * x - x + 1.0 + noise;
            (x, y)
        })
        .collect();

    let scatter_series = Series::new("data")
        .data(scatter_data)
        .color(blue_dark)
        .marker(MarkerShape::Dot);

    let trendline_plot = ScatterPlot::new()
        .series(scatter_series)
        .trendline(TrendlineType::Polynomial(2))
        .trendline_color(Color::Rgb(255, 127, 14))
        .title("A: Quadratic Trendline")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(false);

    // ---- Panel B: JointPlot with histogram marginals ----
    let mut seed_b = 1337u64;
    let n_joint = 400;
    let joint_data: Vec<(f64, f64)> = (0..n_joint)
        .map(|_| {
            let x = box_muller(&mut seed_b, 0.0, 1.5);
            let noise = box_muller(&mut seed_b, 0.0, 0.8);
            let y = 0.6 * x + noise;
            (x, y)
        })
        .collect();

    let joint_series = Series::new("correlated")
        .data(joint_data)
        .color(blue_dark)
        .marker(MarkerShape::Dot);

    let joint_plot = JointPlot::new()
        .series(joint_series)
        .marginal_x(MarginalType::Histogram)
        .marginal_y(MarginalType::Histogram)
        .marginal_bins(25)
        .title("B: JointPlot + Histograms")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(false);

    // ---- Panel C: WaterfallChart — revenue breakdown ----
    let waterfall = WaterfallChart::new()
        .entry(WaterfallEntry::new("Revenue", 500.0))
        .entry(WaterfallEntry::new("COGS", -180.0))
        .entry(WaterfallEntry::new("Gross", 320.0))
        .entry(WaterfallEntry::new("Marketing", -75.0))
        .entry(WaterfallEntry::new("R&D", -60.0))
        .entry(WaterfallEntry::new("Admin", -35.0))
        .entry(WaterfallEntry::new("OpIncome", 150.0))
        .entry(WaterfallEntry::new("Tax", -40.0))
        .entry(WaterfallEntry::total("Net", 110.0))
        .title("C: Waterfall (P&L Breakdown)");

    // ---- Panel D: GaugeChart — KPI at 73% ----
    let gauge = GaugeChart::new(73.0)
        .min(0.0)
        .max(100.0)
        .sector(GaugeSector::new(0.0, 40.0, Color::Green))
        .sector(GaugeSector::new(40.0, 70.0, Color::Yellow))
        .sector(GaugeSector::new(70.0, 100.0, Color::Red))
        .title("D: Gauge (KPI = 73%)");

    // ---- Assemble 2x2 MultiPanel ----
    let panel = MultiPanel::new(2, 2)
        .width_ratios(vec![1.0, 1.0])
        .height_ratios(vec![1.0, 1.0])
        .gap(1)
        .suptitle("Statistics & Dashboard Showcase  (q to quit)")
        .panel(0, 0, move |area: Rect, buf: &mut Buffer| {
            (&trendline_plot).render(area, buf);
        })
        .panel(0, 1, move |area: Rect, buf: &mut Buffer| {
            (&joint_plot).render(area, buf);
        })
        .panel(1, 0, move |area: Rect, buf: &mut Buffer| {
            (&waterfall).render(area, buf);
        })
        .panel(1, 1, move |area: Rect, buf: &mut Buffer| {
            (&gauge).render(area, buf);
        });

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&panel, frame.area());
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
