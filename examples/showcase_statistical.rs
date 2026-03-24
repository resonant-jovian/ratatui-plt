//! Showcase: Statistical plots replicating matplotlib reference gallery.
//!
//! 5 plots in a 2x3 mosaic grid (ABC / DE.):
//! A) BoxPlot, B) ViolinPlot, C) ErrorBarPlot,
//! D) EcdfPlot, E) EventPlot.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::box_plot::BoxData;
use ratatui_plt::widgets::violin_plot::ViolinData;

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

/// Generate a vector of pseudo-normal values.
fn normal_vec(seed: &mut u64, n: usize, mean: f64, std: f64) -> Vec<f64> {
    (0..n).map(|_| box_muller(seed, mean, std)).collect()
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(Theme::light());
    let theme = Theme::get_default();
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // Series colors from theme color cycle
    let blue_dark = theme.color_cycle.at(0);
    let blue_mid = theme.color_cycle.at(1);
    let blue_light = theme.color_cycle.at(2);
    let blue_steel = theme.color_cycle.at(3);

    // ---- Panel A: BoxPlot (4 groups) ----
    let mut seed_box = 42u64;
    let box_group1 = BoxData::new(
        "Group 1",
        normal_vec(&mut seed_box, 50, 5.0, 1.0),
        blue_dark,
    );
    let box_group2 = BoxData::new("Group 2", normal_vec(&mut seed_box, 50, 7.0, 2.0), blue_mid);
    let box_group3 = BoxData::new(
        "Group 3",
        normal_vec(&mut seed_box, 50, 4.0, 1.5),
        blue_light,
    );
    let box_group4 = BoxData::new(
        "Group 4",
        normal_vec(&mut seed_box, 50, 6.0, 0.8),
        blue_steel,
    );

    let box_plot = BoxPlot::new()
        .box_data(box_group1)
        .box_data(box_group2)
        .box_data(box_group3)
        .box_data(box_group4)
        .show_means(true)
        .title("Box Plot")
        .y_axis(
            Axis::new()
                .label("Value")
                .label_position(LabelPosition::End)
                .grid(true),
        );

    // ---- Panel B: ViolinPlot (3 distributions) ----
    let mut seed_v = 777u64;
    // Narrow distribution
    let violin_narrow =
        ViolinData::new("Narrow", normal_vec(&mut seed_v, 200, 5.0, 0.5), blue_dark);
    // Medium distribution
    let violin_medium = ViolinData::new("Medium", normal_vec(&mut seed_v, 200, 5.0, 1.5), blue_mid);
    // Wide distribution
    let violin_wide = ViolinData::new("Wide", normal_vec(&mut seed_v, 200, 5.0, 3.0), blue_light);

    let violin_plot = ViolinPlot::new()
        .dataset(violin_narrow)
        .dataset(violin_medium)
        .dataset(violin_wide)
        .show_box(false)
        .title("Violin Plot")
        .y_axis(
            Axis::new()
                .label("Value")
                .label_position(LabelPosition::End)
                .grid(true),
        );

    // ---- Panel C: ErrorBarPlot ----
    let n_err = 10;
    let mut seed_e = 256u64;
    let err_points: Vec<(f64, f64)> = (0..n_err)
        .map(|i| {
            let x = i as f64;
            let noise = (lcg(&mut seed_e) - 0.5) * 2.0;
            (x, 0.3 * x * x + noise)
        })
        .collect();
    let err_low: Vec<f64> = (0..n_err).map(|i| 0.5 + (i as f64) * 0.15).collect();
    let err_high: Vec<f64> = (0..n_err).map(|i| 0.8 + (i as f64) * 0.2).collect();

    let error_bar_plot = ErrorBarPlot::new()
        .data(err_points, err_low, err_high)
        .color(blue_dark)
        .title("Error Bar Plot")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    // ---- Panel D: EcdfPlot ----
    let mut seed_ecdf = 1024u64;
    let ecdf_data: Vec<f64> = normal_vec(&mut seed_ecdf, 100, 0.0, 1.0);

    let ecdf_ds = EcdfDataset::new("Normal(0,1)", ecdf_data, blue_dark);

    let ecdf_plot = EcdfPlot::new()
        .dataset(ecdf_ds)
        .title("ECDF Plot")
        .x_axis(Axis::new().label("Value").grid(true))
        .y_axis(
            Axis::new()
                .label("F(x)")
                .label_position(LabelPosition::End)
                .grid(true),
        )
        .show_legend(true)
        .legend_position(LegendPosition::BottomRight);

    // ---- Panel E: EventPlot (7 groups) ----
    let event_colors = [
        blue_dark,
        blue_mid,
        blue_light,
        blue_steel,
        theme.color_cycle.at(4),
        theme.color_cycle.at(5),
        theme.color_cycle.at(6),
    ];

    let groups: Vec<EventGroup> = (0..7)
        .map(|i| {
            let mut seed_ev = 100u64 + i * 37;
            let n_events = 8 + (i as usize) * 2;
            let center = (i as f64) * 15.0 + 10.0;
            let mut spikes: Vec<f64> = (0..n_events)
                .map(|_| center + lcg(&mut seed_ev) * 20.0 - 10.0)
                .collect();
            spikes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            EventGroup::new(format!("Ch {}", i + 1), spikes).color(event_colors[i as usize])
        })
        .collect();

    let event_plot = EventPlot::new()
        .groups(groups)
        .title("Event Plot")
        .x_axis(Axis::new().label("Time").grid(true));

    // ---- MultiPanel mosaic layout: ABC / DE. ----
    let panel = MultiPanel::from_mosaic("ABC\nDE.")
        .gap(1)
        .suptitle("Statistical Plots Showcase (q to quit)")
        .mosaic_panel('A', move |area: Rect, buf: &mut Buffer| {
            (&box_plot).render(area, buf);
        })
        .mosaic_panel('B', move |area: Rect, buf: &mut Buffer| {
            (&violin_plot).render(area, buf);
        })
        .mosaic_panel('C', move |area: Rect, buf: &mut Buffer| {
            (&error_bar_plot).render(area, buf);
        })
        .mosaic_panel('D', move |area: Rect, buf: &mut Buffer| {
            (&ecdf_plot).render(area, buf);
        })
        .mosaic_panel('E', move |area: Rect, buf: &mut Buffer| {
            (&event_plot).render(area, buf);
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
