//! Showcase: Basic 2D plots replicating matplotlib reference gallery.
//!
//! 7 plots in a 3x3 mosaic grid (ABC / DEF / GH.):
//! A) LinePlot, B) ScatterPlot, C) BarChart, D) Histogram,
//! E) PieChart, F) StairsPlot, G) StemPlot (spanning two columns).

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::bar_chart::BarDataset;
use ratatui_plt::widgets::histogram::HistDataset;

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

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(Theme::light());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Blue shades used throughout
    let blue_dark = Color::Rgb(31, 119, 180);
    let blue_mid = Color::Rgb(70, 130, 180);
    let blue_light = Color::Rgb(174, 199, 232);
    let blue_pale = Color::Rgb(198, 219, 239);

    // ---- Panel A: LinePlot (3 series) ----
    let mut seed_a = 42u64;

    // (a) Scattered X markers at random positions
    let scatter_data: Vec<(f64, f64)> = (0..20)
        .map(|_| {
            let x = lcg(&mut seed_a) * 10.0;
            let y = lcg(&mut seed_a) * 2.0 - 1.0;
            (x, y)
        })
        .collect();
    let scatter_series = Series::new("Random")
        .data(scatter_data)
        .color(blue_dark)
        .marker(MarkerShape::Cross);

    // (b) Smooth sine wave
    let sine_data: Vec<(f64, f64)> = (0..200)
        .map(|i| {
            let x = i as f64 * 0.05;
            (x, x.sin())
        })
        .collect();
    let sine_series = Series::new("sin(x)").data(sine_data).color(blue_mid);

    // (c) Filled sine with circle markers (fewer points)
    let filled_data: Vec<(f64, f64)> = (0..30)
        .map(|i| {
            let x = i as f64 * 0.33;
            (x, 0.5 * (x * 0.8).sin())
        })
        .collect();
    let filled_series = Series::new("0.5 sin(0.8x)")
        .data(filled_data)
        .color(blue_light)
        .marker(MarkerShape::FilledCircle);

    let line_plot = LinePlot::new()
        .series(scatter_series)
        .series(sine_series)
        .series(filled_series)
        .title("Line Plot")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

    // ---- Panel B: ScatterPlot ----
    let mut seed_b = 1337u64;
    let n_scatter = 30;
    let scatter_pts: Vec<(f64, f64)> = (0..n_scatter)
        .map(|_| (lcg(&mut seed_b) * 10.0, lcg(&mut seed_b) * 10.0))
        .collect();
    let scatter_colors: Vec<f64> = (0..n_scatter).map(|_| lcg(&mut seed_b)).collect();

    let scatter_s = Series::new("points")
        .data(scatter_pts)
        .marker(MarkerShape::FilledCircle);

    let scatter_plot = ScatterPlot::new()
        .series(scatter_s)
        .color_values(scatter_colors)
        .colormap(Blues)
        .title("Scatter Plot")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(false);

    // ---- Panel C: BarChart ----
    let bar_labels = vec!["A", "B", "C", "D", "E", "F", "G", "H"];
    let bar_values = vec![40.0, 55.0, 30.0, 50.0, 65.0, 45.0, 35.0, 50.0];

    let bar_chart = BarChart::new()
        .categories(bar_labels)
        .dataset(BarDataset::new("Values", bar_values, blue_dark))
        .title("Bar Chart");

    // ---- Panel D: Histogram ----
    let mut seed_d = 99u64;
    let hist_data: Vec<f64> = (0..200)
        .map(|_| box_muller(&mut seed_d, 0.0, 1.0))
        .collect();

    let histogram = Histogram::new(vec![])
        .dataset(HistDataset::new("Normal", hist_data, blue_dark))
        .bins(20)
        .title("Histogram")
        .x_axis(Axis::new().label("Value").grid(true))
        .y_axis(
            Axis::new()
                .label("Count")
                .label_position(LabelPosition::End)
                .grid(true),
        )
        .show_legend(false);

    // ---- Panel E: PieChart ----
    let pie = PieChart::new()
        .slice(PieSlice::new("Frogs", 35.0).color(blue_dark))
        .slice(PieSlice::new("Hogs", 25.0).color(blue_mid))
        .slice(PieSlice::new("Dogs", 20.0).color(blue_light))
        .slice(PieSlice::new("Logs", 20.0).color(blue_pale))
        .show_labels(true)
        .title("Pie Chart");

    // ---- Panel F: StairsPlot ----
    let mut seed_f = 555u64;
    let n_stairs = 10;
    let stair_edges: Vec<f64> = (0..=n_stairs).map(|i| i as f64).collect();
    let stair_values: Vec<f64> = (0..n_stairs)
        .map(|_| lcg(&mut seed_f) * 8.0 + 1.0)
        .collect();

    let stairs_ds = StairsDataset::new("Steps", stair_edges, stair_values, blue_dark);

    let stairs_plot = StairsPlot::new()
        .dataset(stairs_ds)
        .baseline(0.0)
        .title("Stairs Plot")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(false);

    // ---- Panel G: StemPlot (spans 2 columns) ----
    let stem_data: Vec<(f64, f64)> = (0..10)
        .map(|i| {
            let x = i as f64;
            let y = (x * 0.6).sin() * 3.0 + 1.0;
            (x, y)
        })
        .collect();

    let stem_plot = StemPlot::new(stem_data)
        .baseline(0.0)
        .color(blue_dark)
        .marker(MarkerShape::FilledCircle)
        .title("Stem Plot")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    // ---- MultiPanel mosaic layout: ABC / DEF / GH. ----
    let panel = MultiPanel::from_mosaic("ABC\nDEF\nGH.")
        .gap(1)
        .suptitle("Basic 2D Plots Showcase (q to quit)")
        .mosaic_panel('A', move |area: Rect, buf: &mut Buffer| {
            (&line_plot).render(area, buf);
        })
        .mosaic_panel('B', move |area: Rect, buf: &mut Buffer| {
            (&scatter_plot).render(area, buf);
        })
        .mosaic_panel('C', move |area: Rect, buf: &mut Buffer| {
            (&bar_chart).render(area, buf);
        })
        .mosaic_panel('D', move |area: Rect, buf: &mut Buffer| {
            (&histogram).render(area, buf);
        })
        .mosaic_panel('E', move |area: Rect, buf: &mut Buffer| {
            (&pie).render(area, buf);
        })
        .mosaic_panel('F', move |area: Rect, buf: &mut Buffer| {
            (&stairs_plot).render(area, buf);
        })
        .mosaic_panel('G', move |area: Rect, buf: &mut Buffer| {
            (&stem_plot).render(area, buf);
        });

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
