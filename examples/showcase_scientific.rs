//! Showcase: Scientific visualization widgets.
//!
//! 6 plots in a 2x3 mosaic grid (ABC / DEF):
//! A) HorizonGraph, B) CarpetPlot, C) SmithChart,
//! D) ChoroplethMap, E) PSD (fft feature), F) Spectrogram (fft feature).
//!
//! Panels E and F require the `fft` feature. Without it, a placeholder
//! text label is rendered instead.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::carpet::CarpetPlot;
use ratatui_plt::widgets::choropleth::{ChoroplethMap, ChoroplethRegion, MapType};
use ratatui_plt::widgets::smith_chart::{SmithChart, SmithChartPoint};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(Theme::light());
    let theme = Theme::get_default();
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let c0 = theme.color_cycle.at(0);
    let c1 = theme.color_cycle.at(1);
    let c2 = theme.color_cycle.at(2);

    // ---- Panel A: HorizonGraph — 3 sensor time series ----
    let n_pts = 200;
    let sensor1: Vec<(f64, f64)> = (0..n_pts)
        .map(|i| {
            let x = i as f64 * 0.05;
            (x, (x * 0.7).sin() * 3.0 + 0.5 * x.sqrt())
        })
        .collect();
    let sensor2: Vec<(f64, f64)> = (0..n_pts)
        .map(|i| {
            let x = i as f64 * 0.05;
            (x, (x * 1.2).cos() * 2.5 - 0.3 * x.sqrt())
        })
        .collect();
    let sensor3: Vec<(f64, f64)> = (0..n_pts)
        .map(|i| {
            let x = i as f64 * 0.05;
            (x, (x * 0.4).sin() * 2.0 + (x * 1.8).cos() * 1.5)
        })
        .collect();

    let horizon = HorizonGraph::new()
        .series(Series::new("Sensor A").data(sensor1).color(c0))
        .series(Series::new("Sensor B").data(sensor2).color(c1))
        .series(Series::new("Sensor C").data(sensor3).color(c2))
        .n_bands(3)
        .x_axis(Axis::new().label("Time").grid(true))
        .title("Horizon Graph");

    // ---- Panel B: CarpetPlot — parametric grid with scalar field ----
    let na = 10;
    let nb = 8;
    let a_vals: Vec<f64> = (0..na).map(|i| i as f64).collect();
    let b_vals: Vec<f64> = (0..nb).map(|i| i as f64).collect();
    let mut x_grid = vec![vec![0.0; na]; nb];
    let mut y_grid = vec![vec![0.0; na]; nb];
    let mut val_grid = vec![vec![0.0; na]; nb];

    for (bi, &bv) in b_vals.iter().enumerate() {
        for (ai, &av) in a_vals.iter().enumerate() {
            // Curvilinear mapping with some warping
            x_grid[bi][ai] = av + 0.2 * bv * (av * 0.3).sin();
            y_grid[bi][ai] = bv + 0.15 * av * (bv * 0.4).cos();
            // Scalar field: distance-based
            let cx = av - (na as f64) / 2.0;
            let cy = bv - (nb as f64) / 2.0;
            val_grid[bi][ai] = (-(cx * cx + cy * cy) / 10.0).exp();
        }
    }

    let carpet = CarpetPlot::new(a_vals, b_vals, x_grid, y_grid)
        .values(val_grid)
        .colormap(Viridis)
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .title("Carpet Plot");

    // ---- Panel C: SmithChart — 6 RF impedance points ----
    let smith = SmithChart::new()
        .point(SmithChartPoint::new(1.0, 0.0).label("Z0"))
        .point(SmithChartPoint::new(0.5, 0.5))
        .point(SmithChartPoint::new(2.0, 1.0))
        .point(SmithChartPoint::new(0.3, -0.8))
        .point(SmithChartPoint::new(1.5, -0.5))
        .point(SmithChartPoint::new(0.8, 1.2).label("Load"))
        .show_trace(true)
        .title("Smith Chart");

    // ---- Panel D: ChoroplethMap — world population by continent ----
    let choropleth = ChoroplethMap::new()
        .region(ChoroplethRegion::new("North America", 580.0))
        .region(ChoroplethRegion::new("South America", 423.0))
        .region(ChoroplethRegion::new("Europe", 447.0))
        .region(ChoroplethRegion::new("Africa", 1400.0))
        .region(ChoroplethRegion::new("Asia", 4700.0))
        .region(ChoroplethRegion::new("Oceania", 44.0))
        .map_type(MapType::World)
        .colormap(Plasma)
        .title("World Population (M)");

    // ---- Panel E & F: PSD and Spectrogram (conditional on fft feature) ----
    // Build the composite signal (440 Hz + 1000 Hz) regardless of feature
    // so the code structure is clear, even if unused without fft.
    #[cfg(feature = "fft")]
    let sample_rate = 8000.0_f64;
    #[cfg(feature = "fft")]
    let n_samples = 4096_usize;
    #[cfg(feature = "fft")]
    let signal: Vec<f64> = (0..n_samples)
        .map(|i| {
            let t = i as f64 / sample_rate;
            let pi2 = 2.0 * std::f64::consts::PI;
            (pi2 * 440.0 * t).sin() + 0.6 * (pi2 * 1000.0 * t).sin()
        })
        .collect();

    #[cfg(feature = "fft")]
    let psd_series = ratatui_plt::fft::psd(&signal, sample_rate);
    #[cfg(feature = "fft")]
    let psd_plot = ratatui_plt::widgets::psd::PsdPlot::new()
        .series(psd_series)
        .title("PSD (440 Hz + 1 kHz)")
        .x_axis(Axis::new().label("Frequency (Hz)").grid(true))
        .y_axis(Axis::new().label("Power (dB)").grid(true))
        .show_legend(false);

    #[cfg(feature = "fft")]
    let stft_grid = ratatui_plt::fft::stft(&signal, 256, 128);
    #[cfg(feature = "fft")]
    let spectrogram = ratatui_plt::widgets::spectrogram::Spectrogram::new(stft_grid)
        .colormap(Viridis)
        .title("Spectrogram")
        .x_axis(Axis::new().label("Time").grid(true))
        .y_axis(Axis::new().label("Freq").grid(true))
        .show_colorbar(false);

    // ---- Assemble 2x3 mosaic: ABC / DEF ----
    let panel = MultiPanel::from_mosaic("ABC\nDEF")
        .gap(1)
        .suptitle("Scientific Plots Showcase (q to quit)")
        .mosaic_panel('A', move |area: Rect, buf: &mut Buffer| {
            (&horizon).render(area, buf);
        })
        .mosaic_panel('B', move |area: Rect, buf: &mut Buffer| {
            (&carpet).render(area, buf);
        })
        .mosaic_panel('C', move |area: Rect, buf: &mut Buffer| {
            (&smith).render(area, buf);
        })
        .mosaic_panel('D', move |area: Rect, buf: &mut Buffer| {
            (&choropleth).render(area, buf);
        })
        .mosaic_panel('E', move |area: Rect, buf: &mut Buffer| {
            #[cfg(feature = "fft")]
            {
                (&psd_plot).render(area, buf);
            }
            #[cfg(not(feature = "fft"))]
            {
                let msg = "PSD (enable fft feature)";
                let x = area.x + area.width.saturating_sub(msg.len() as u16) / 2;
                let y = area.y + area.height / 2;
                for (i, ch) in msg.chars().enumerate() {
                    let px = x + i as u16;
                    if px < area.x + area.width && y < area.y + area.height {
                        buf[(px, y)].set_char(ch);
                    }
                }
            }
        })
        .mosaic_panel('F', move |area: Rect, buf: &mut Buffer| {
            #[cfg(feature = "fft")]
            {
                (&spectrogram).render(area, buf);
            }
            #[cfg(not(feature = "fft"))]
            {
                let msg = "Spectrogram (enable fft feature)";
                let x = area.x + area.width.saturating_sub(msg.len() as u16) / 2;
                let y = area.y + area.height / 2;
                for (i, ch) in msg.chars().enumerate() {
                    let px = x + i as u16;
                    if px < area.x + area.width && y < area.y + area.height {
                        buf[(px, y)].set_char(ch);
                    }
                }
            }
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
