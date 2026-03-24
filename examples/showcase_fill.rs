//! Showcase: Fill plots replicating matplotlib reference gallery.
//!
//! 2 plots stacked vertically in a 2x1 MultiPanel:
//! Top) BandPlot (fill_between), Bottom) StackedArea (stackplot).

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(Theme::light());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Blue shades
    let blue_dark = Color::Rgb(31, 119, 180);
    let blue_mid = Color::Rgb(70, 130, 180);
    let blue_light = Color::Rgb(174, 199, 232);

    // ---- Panel A: BandPlot (fill_between two curves) ----
    let n_band = 100;
    let x_band: Vec<f64> = (0..n_band).map(|i| i as f64 * 0.1).collect();

    // Upper curve: sin(x) + 1.5
    let y_upper: Vec<f64> = x_band
        .iter()
        .map(|&x| x.sin() + 1.5 + 0.3 * (2.0 * x).sin())
        .collect();

    // Lower curve: sin(x) - offset
    let y_lower: Vec<f64> = x_band
        .iter()
        .map(|&x| x.sin() - 0.5 - 0.2 * (3.0 * x).cos())
        .collect();

    let filled_band =
        Band::new("Filled region", x_band.clone(), y_lower, y_upper).color(blue_light);

    // Center reference line (y_center for visual reference)
    let y_center_lo: Vec<f64> = x_band.iter().map(|&x| x.sin() + 0.5).collect();
    let y_center_hi = y_center_lo.clone();

    let center_line =
        Band::new("Center", x_band.clone(), y_center_lo, y_center_hi).color(blue_dark);

    let band_plot = BandPlot::new()
        .band(filled_band)
        .band(center_line)
        .title("Band Plot (fill_between)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

    // ---- Panel B: StackedArea (3 series) ----
    let n_stack = 200;

    let series_a = Series::new("Research")
        .data(
            (0..n_stack)
                .map(|i| {
                    let t = i as f64;
                    (t, 10.0 + 5.0 * (t * std::f64::consts::TAU / 60.0).sin())
                })
                .collect(),
        )
        .color(blue_dark);

    let series_b = Series::new("Development")
        .data(
            (0..n_stack)
                .map(|i| {
                    let t = i as f64;
                    (t, 15.0 + 4.0 * (t * std::f64::consts::TAU / 40.0).cos())
                })
                .collect(),
        )
        .color(blue_mid);

    let series_c = Series::new("Operations")
        .data(
            (0..n_stack)
                .map(|i| {
                    let t = i as f64;
                    (t, 8.0 + 3.0 * (t * std::f64::consts::TAU / 80.0).sin())
                })
                .collect(),
        )
        .color(blue_light);

    let stacked = StackedArea::new()
        .series(series_a)
        .series(series_b)
        .series(series_c)
        .title("Stacked Area Plot")
        .x_axis(Axis::new().label("Time").grid(true))
        .y_axis(
            Axis::new()
                .label("Value")
                .label_position(LabelPosition::End)
                .grid(true),
        );

    // ---- MultiPanel: 2 rows x 1 col ----
    let panel = MultiPanel::new(2, 1)
        .gap(1)
        .suptitle("Fill Plots Showcase (q to quit)")
        .panel(0, 0, move |area: Rect, buf: &mut Buffer| {
            (&band_plot).render(area, buf);
        })
        .panel(1, 0, move |area: Rect, buf: &mut Buffer| {
            (&stacked).render(area, buf);
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
