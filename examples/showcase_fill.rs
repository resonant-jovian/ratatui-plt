//! Showcase: Fill plots replicating matplotlib reference gallery.
//!
//! 5 plots in a mosaic grid (ABC / DE.):
//! A) BandPlot (fill_between), B) AreaChart plain fill,
//! C) AreaChart stacked, D) AreaChart normalized, E) AreaChart streamgraph.

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
    let theme = Theme::get_default();
    // Series colors from theme color cycle
    let blue_dark = theme.color_cycle.at(0);
    let blue_mid = theme.color_cycle.at(1);
    let blue_light = theme.color_cycle.at(2);

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
        .title("A: Band Plot (fill_between)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(true)
        .legend_position(LegendPosition::TopRight);

    // ---- Shared data for area chart modes (B-E) ----
    let n_stack = 200;

    // Helper to build the 3 area series with fresh clones
    let make_area_series = || {
        let s_a = Series::new("Research")
            .data(
                (0..n_stack)
                    .map(|i| {
                        let t = i as f64;
                        (t, 10.0 + 5.0 * (t * std::f64::consts::TAU / 60.0).sin())
                    })
                    .collect(),
            )
            .color(blue_dark);

        let s_b = Series::new("Development")
            .data(
                (0..n_stack)
                    .map(|i| {
                        let t = i as f64;
                        (t, 15.0 + 4.0 * (t * std::f64::consts::TAU / 40.0).cos())
                    })
                    .collect(),
            )
            .color(blue_mid);

        let s_c = Series::new("Operations")
            .data(
                (0..n_stack)
                    .map(|i| {
                        let t = i as f64;
                        (t, 8.0 + 3.0 * (t * std::f64::consts::TAU / 80.0).sin())
                    })
                    .collect(),
            )
            .color(blue_light);

        (s_a, s_b, s_c)
    };

    // ---- Panel B: AreaChart plain fill (single series) ----
    let plain_series = Series::new("Signal")
        .data(
            (0..n_stack)
                .map(|i| {
                    let t = i as f64;
                    (t, 10.0 + 5.0 * (t * std::f64::consts::TAU / 60.0).sin())
                })
                .collect(),
        )
        .color(blue_dark);

    let area_plain = AreaChart::new()
        .series(plain_series)
        .mode(AreaMode::Plain)
        .title("B: Area (Plain)")
        .x_axis(Axis::new().label("Time").grid(true))
        .y_axis(
            Axis::new()
                .label("Value")
                .label_position(LabelPosition::End)
                .grid(true),
        )
        .show_legend(false);

    // ---- Panel C: AreaChart stacked (3 series) ----
    let (sc_a, sc_b, sc_c) = make_area_series();
    let area_stacked = AreaChart::new()
        .series(sc_a)
        .series(sc_b)
        .series(sc_c)
        .mode(AreaMode::Stacked)
        .title("C: Area (Stacked)")
        .x_axis(Axis::new().label("Time").grid(true))
        .y_axis(
            Axis::new()
                .label("Value")
                .label_position(LabelPosition::End)
                .grid(true),
        );

    // ---- Panel D: AreaChart normalized (3 series) ----
    let (sd_a, sd_b, sd_c) = make_area_series();
    let area_normalized = AreaChart::new()
        .series(sd_a)
        .series(sd_b)
        .series(sd_c)
        .mode(AreaMode::Normalized)
        .title("D: Area (Normalized)")
        .x_axis(Axis::new().label("Time").grid(true))
        .y_axis(
            Axis::new()
                .label("Fraction")
                .label_position(LabelPosition::End)
                .grid(true),
        );

    // ---- Panel E: AreaChart streamgraph (3 series) ----
    let (se_a, se_b, se_c) = make_area_series();
    let area_stream = AreaChart::new()
        .series(se_a)
        .series(se_b)
        .series(se_c)
        .mode(AreaMode::StreamGraph)
        .title("E: Area (StreamGraph)")
        .x_axis(Axis::new().label("Time").grid(true))
        .y_axis(
            Axis::new()
                .label("Value")
                .label_position(LabelPosition::End)
                .grid(true),
        );

    // ---- MultiPanel mosaic: ABC / DE. ----
    let panel = MultiPanel::from_mosaic("ABC\nDE.")
        .gap(1)
        .suptitle("Fill Plots Showcase (q to quit)")
        .mosaic_panel('A', move |area: Rect, buf: &mut Buffer| {
            (&band_plot).render(area, buf);
        })
        .mosaic_panel('B', move |area: Rect, buf: &mut Buffer| {
            (&area_plain).render(area, buf);
        })
        .mosaic_panel('C', move |area: Rect, buf: &mut Buffer| {
            (&area_stacked).render(area, buf);
        })
        .mosaic_panel('D', move |area: Rect, buf: &mut Buffer| {
            (&area_normalized).render(area, buf);
        })
        .mosaic_panel('E', move |area: Rect, buf: &mut Buffer| {
            (&area_stream).render(area, buf);
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
