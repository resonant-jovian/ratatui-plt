use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::bar_chart::{BarChart, BarDataset};

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
    let theme = Theme::get_default();
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Panel (0,0): Heatmap with gradient
    let heatmap_data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 360, 360, |x, y| {
        (-(x * x + y * y) / 4.0).exp()
    });
    let heatmap = Heatmap::new(heatmap_data)
        .colormap(Viridis)
        .title("Gaussian Heatmap")
        .show_colorbar(false)
        .aspect_ratio(AspectRatio::Equal);

    // Panel (0,1): Scatter plot with point cloud
    let scatter_points: Vec<(f64, f64)> = (0..200)
        .map(|i| {
            let t = i as f64 * 0.0314;
            let r = 3.0 * (i as f64 * 0.017).sin().abs();
            (r * t.cos(), r * t.sin())
        })
        .collect();
    let scatter = ScatterPlot::new()
        .series(
            Series::new("spiral")
                .data(scatter_points)
                .color(theme.primary)
                .marker(MarkerShape::FilledCircle),
        )
        .title("Spiral Scatter")
        .show_legend(false)
        .aspect_ratio(AspectRatio::Equal);

    // Panel (1,0): Bar chart
    let bar_chart = BarChart::new()
        .categories(vec!["A", "B", "C", "D", "E"])
        .dataset(BarDataset::new(
            "Values",
            vec![8.0, 15.0, 12.0, 20.0, 6.0],
            theme.primary,
        ))
        .title("Sample Bars");

    // Panel (1,1): Line plot with sine wave
    let sine_data: Vec<(f64, f64)> = (0..150)
        .map(|i| {
            let x = i as f64 * 0.08;
            (x, x.sin())
        })
        .collect();
    let cosine_data: Vec<(f64, f64)> = (0..150)
        .map(|i| {
            let x = i as f64 * 0.08;
            (x, x.cos())
        })
        .collect();
    let line_plot = LinePlot::new()
        .series(Series::new("sin").data(sine_data).color(theme.primary))
        .series(Series::new("cos").data(cosine_data).color(theme.secondary))
        .title("Sine & Cosine")
        .show_legend(true);

    let panel = MultiPanel::new(2, 2)
        .width_ratios(vec![1.0, 1.0])
        .height_ratios(vec![1.0, 1.0])
        .gap(1)
        .panel(0, 0, move |area: Rect, buf: &mut Buffer| {
            (&heatmap).render(area, buf);
        })
        .panel(0, 1, move |area: Rect, buf: &mut Buffer| {
            (&scatter).render(area, buf);
        })
        .panel(1, 0, move |area: Rect, buf: &mut Buffer| {
            (&bar_chart).render(area, buf);
        })
        .panel(1, 1, move |area: Rect, buf: &mut Buffer| {
            (&line_plot).render(area, buf);
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
