//! Showcase grid: 8 plot types in a 3x3 MultiPanel mosaic layout.
//!
//! Replicates matplotlib reference plots using ratatui-plt:
//!   A: ContourPlot (unfilled)    B: ContourPlot (filled)    C: Heatmap (imshow)
//!   D: Pcolormesh                E: HexbinPlot              F: Hist2D
//!   G: VectorField (quiver)      H: StreamPlot
//!
//! Run: cargo run --example showcase_grid

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;

/// Deterministic LCG pseudo-random number generator.
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*seed >> 33) as f64 / (1u64 << 31) as f64
}

/// Box-Muller transform: produce a standard-normal sample from the LCG.
fn lcg_normal(seed: &mut u64) -> f64 {
    let u1 = lcg(seed).max(1e-10);
    let u2 = lcg(seed);
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(Theme::light());
    // ── A: Contour plot (unfilled) ──────────────────────────────────────
    let contour_data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 120, 120, |x, y| {
        x.sin() * y.cos() + (x * y / 3.0).sin()
    });
    let contour_unfilled = ContourPlot::new(contour_data)
        .levels(12)
        .filled(false)
        .colormap(Viridis)
        .title("Contour (unfilled)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .aspect_ratio(AspectRatio::Equal);

    // ── B: Contour plot (filled) ────────────────────────────────────────
    let contourf_data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 120, 120, |x, y| {
        x.sin() * y.cos() + (x * y / 3.0).sin()
    });
    let contour_filled = ContourPlot::new(contourf_data)
        .levels(12)
        .filled(true)
        .colormap(Viridis)
        .title("Contour (filled)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .aspect_ratio(AspectRatio::Equal);

    // ── C: Heatmap / imshow ─────────────────────────────────────────────
    // Overlapping Gaussians
    let heatmap_data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 200, 200, |x, y| {
        let g1 = (-((x - 1.0).powi(2) + (y - 1.0).powi(2)) / 1.2).exp();
        let g2 = 0.7 * (-((x + 1.0).powi(2) + (y + 0.5).powi(2)) / 1.8).exp();
        let g3 = 0.5 * (-((x - 0.5).powi(2) + (y + 1.5).powi(2)) / 0.8).exp();
        g1 + g2 + g3
    });
    let heatmap = Heatmap::new(heatmap_data)
        .colormap(Viridis)
        .title("Heatmap (imshow)")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"))
        .show_colorbar(false)
        .aspect_ratio(AspectRatio::Equal);

    // ── D: Pcolormesh ───────────────────────────────────────────────────
    // Polar-deformed mesh
    let nr = 15_usize;
    let nt = 20_usize;
    let mut pcm_x = vec![vec![0.0_f64; nt + 1]; nr + 1];
    let mut pcm_y = vec![vec![0.0_f64; nt + 1]; nr + 1];
    let mut pcm_v = vec![vec![0.0_f64; nt]; nr];
    for i in 0..=nr {
        let r = i as f64 / nr as f64 * 2.0;
        for j in 0..=nt {
            let theta = 2.0 * std::f64::consts::PI * j as f64 / nt as f64;
            pcm_x[i][j] = r * theta.cos();
            pcm_y[i][j] = r * theta.sin();
        }
    }
    for (i, row) in pcm_v.iter_mut().enumerate() {
        let r_mid = (i as f64 + 0.5) / nr as f64 * 2.0;
        for (j, val) in row.iter_mut().enumerate() {
            let theta_mid = 2.0 * std::f64::consts::PI * (j as f64 + 0.5) / nt as f64;
            *val = (r_mid * theta_mid.cos()).sin() * (r_mid * theta_mid.sin()).cos();
        }
    }
    let pcolormesh = Pcolormesh::new(pcm_x, pcm_y, pcm_v)
        .colormap(Plasma)
        .show_colorbar(false)
        .title("Pcolormesh (polar)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .aspect_ratio(AspectRatio::Equal);

    // ── E: Hexbin plot ──────────────────────────────────────────────────
    // Correlated diagonal data with noise
    let mut seed = 42_u64;
    let hexbin_data: Vec<(f64, f64)> = (0..500)
        .map(|_| {
            let x = lcg_normal(&mut seed) * 1.5;
            let y = x + lcg_normal(&mut seed) * 0.8;
            (x, y)
        })
        .collect();
    let hexbin = HexbinPlot::new(hexbin_data)
        .gridsize(18)
        .colormap(Viridis)
        .title("Hexbin")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    // ── F: Hist2D ───────────────────────────────────────────────────────
    let mut seed2 = 12345_u64;
    let hist2d_data: Vec<(f64, f64)> = (0..2000)
        .map(|_| {
            let x = lcg_normal(&mut seed2) * 1.5;
            let y = lcg_normal(&mut seed2) * 1.5;
            (x, y)
        })
        .collect();
    let hist2d = Hist2D::new(hist2d_data)
        .bins_x(25)
        .bins_y(25)
        .colormap(Viridis)
        .show_colorbar(false)
        .title("Hist2D")
        .x_axis(Axis::new().label("x"))
        .y_axis(Axis::new().label("y"));

    // ── G: VectorField (quiver) ─────────────────────────────────────────
    // Rotational field: u = -y, v = x
    let quiver_field = VectorFieldData::from_fn((-2.0, 2.0), (-2.0, 2.0), 10, 10, |x, y| (-y, x));
    let vector_field = VectorField::new(quiver_field)
        .title("VectorField (quiver)")
        .color_by_magnitude(true)
        .colormap(Viridis)
        .arrow_scale(2.5)
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .aspect_ratio(AspectRatio::Equal);

    // ── H: StreamPlot ───────────────────────────────────────────────────
    // Same rotational field
    let stream_field = VectorFieldData::from_fn((-2.0, 2.0), (-2.0, 2.0), 15, 15, |x, y| (-y, x));
    let streamplot = StreamPlot::new(stream_field)
        .density(2)
        .arrow_scale(2.0)
        .color_by_magnitude(true)
        .colormap(Viridis)
        .title("StreamPlot")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    // ── Assemble 3x3 mosaic: "ABC\nDEF\nGH." ──────────────────────────
    let panel = MultiPanel::from_mosaic("ABC\nDEF\nGH.")
        .gap(1)
        .suptitle("ratatui-plt Showcase Grid  (q to quit)")
        .mosaic_panel('A', move |area: Rect, buf: &mut Buffer| {
            (&contour_unfilled).render(area, buf);
        })
        .mosaic_panel('B', move |area: Rect, buf: &mut Buffer| {
            (&contour_filled).render(area, buf);
        })
        .mosaic_panel('C', move |area: Rect, buf: &mut Buffer| {
            (&heatmap).render(area, buf);
        })
        .mosaic_panel('D', move |area: Rect, buf: &mut Buffer| {
            (&pcolormesh).render(area, buf);
        })
        .mosaic_panel('E', move |area: Rect, buf: &mut Buffer| {
            (&hexbin).render(area, buf);
        })
        .mosaic_panel('F', move |area: Rect, buf: &mut Buffer| {
            (&hist2d).render(area, buf);
        })
        .mosaic_panel('G', move |area: Rect, buf: &mut Buffer| {
            (&vector_field).render(area, buf);
        })
        .mosaic_panel('H', move |area: Rect, buf: &mut Buffer| {
            (&streamplot).render(area, buf);
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
