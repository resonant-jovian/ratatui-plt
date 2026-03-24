//! Showcase triangulation: 4 triangulated-data plots in a 2x2 MultiPanel.
//!
//! Replicates matplotlib reference plots:
//!   A: TriPlot (mesh edges)          B: TriContour (unfilled contour lines)
//!   C: TriContour (denser levels)    D: TriColor (filled faces)
//!
//! Run: cargo run --example showcase_tri

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

type Vertices = Vec<(f64, f64)>;
type Triangles = Vec<(usize, usize, usize)>;

/// Build a grid triangulation on [-2, 2] x [-2, 2] with scattered vertex
/// perturbations for a more organic look.  Returns (vertices, triangles).
fn make_triangulation(n: usize, seed: &mut u64) -> (Vertices, Triangles) {
    let lo = -2.0_f64;
    let hi = 2.0_f64;
    let step = (hi - lo) / (n - 1) as f64;

    let mut vertices = Vec::with_capacity(n * n);
    for row in 0..n {
        for col in 0..n {
            let base_x = lo + col as f64 * step;
            let base_y = lo + row as f64 * step;
            // Perturb interior vertices for a more natural mesh
            let (dx, dy) = if row > 0 && row < n - 1 && col > 0 && col < n - 1 {
                let jx = (lcg(seed) - 0.5) * step * 0.35;
                let jy = (lcg(seed) - 0.5) * step * 0.35;
                (jx, jy)
            } else {
                (0.0, 0.0)
            };
            vertices.push((base_x + dx, base_y + dy));
        }
    }

    let mut triangles = Vec::with_capacity(2 * (n - 1) * (n - 1));
    for row in 0..(n - 1) {
        for col in 0..(n - 1) {
            let bl = row * n + col;
            let br = bl + 1;
            let tl = bl + n;
            let tr = tl + 1;
            triangles.push((bl, br, tl));
            triangles.push((br, tr, tl));
        }
    }

    (vertices, triangles)
}

/// Scalar field evaluated at a vertex: peaks function.
fn scalar_fn(x: f64, y: f64) -> f64 {
    let g1 = 3.0 * (-(x - 0.5).powi(2) - (y - 0.5).powi(2)).exp();
    let g2 = -((-(x + 1.0).powi(2)) - (y + 1.0).powi(2)).exp();
    let g3 = -0.5 * (-(x.powi(2) + y.powi(2)) / 4.0).exp();
    g1 + g2 + g3
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(Theme::light());
    let theme = Theme::get_default();
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let grid_n = 12_usize;

    // ── A: TriPlot (mesh edges) ─────────────────────────────────────────
    let mut seed_a = 100_u64;
    let (verts_a, tris_a) = make_triangulation(grid_n, &mut seed_a);
    let triplot = TriPlot::new(Triangulation::from_explicit(verts_a, tris_a))
        .title("TriPlot (mesh)")
        .edge_color(theme.primary)
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    // ── B: TriContour (unfilled) ────────────────────────────────────────
    let mut seed_b = 100_u64; // same seed for identical mesh
    let (verts_b, tris_b) = make_triangulation(grid_n, &mut seed_b);
    let vals_b: Vec<f64> = verts_b.iter().map(|&(x, y)| scalar_fn(x, y)).collect();
    let tricontour_unfilled = TriContour::new(Triangulation::from_explicit(verts_b, tris_b))
        .vertex_values(vals_b)
        .levels_auto(10)
        .colormap(Viridis)
        .title("TriContour (unfilled)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    // ── C: TriContour (denser levels) ───────────────────────────────────
    let mut seed_c = 100_u64;
    let (verts_c, tris_c) = make_triangulation(grid_n, &mut seed_c);
    let vals_c: Vec<f64> = verts_c.iter().map(|&(x, y)| scalar_fn(x, y)).collect();
    let tricontour_filled = TriContour::new(Triangulation::from_explicit(verts_c, tris_c))
        .vertex_values(vals_c)
        .levels_auto(20)
        .colormap(Plasma)
        .title("TriContour (dense)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    // ── D: TriColor (filled faces) ──────────────────────────────────────
    let mut seed_d = 100_u64;
    let (verts_d, tris_d) = make_triangulation(grid_n, &mut seed_d);
    let face_vals: Vec<f64> = tris_d
        .iter()
        .map(|&(a, b, c)| {
            let (ax, ay) = verts_d[a];
            let (bx, by) = verts_d[b];
            let (cx, cy) = verts_d[c];
            let mx = (ax + bx + cx) / 3.0;
            let my = (ay + by + cy) / 3.0;
            scalar_fn(mx, my)
        })
        .collect();
    let tricolor = TriColor::new(Triangulation::from_explicit(verts_d, tris_d))
        .face_values(face_vals)
        .colormap(Viridis)
        .title("TriColor (faces)")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true));

    // ── Assemble 2x2 panel ──────────────────────────────────────────────
    let panel = MultiPanel::new(2, 2)
        .width_ratios(vec![1.0, 1.0])
        .height_ratios(vec![1.0, 1.0])
        .gap(1)
        .suptitle("ratatui-plt Triangulation Showcase  (q to quit)")
        .panel(0, 0, move |area: Rect, buf: &mut Buffer| {
            (&triplot).render(area, buf);
        })
        .panel(0, 1, move |area: Rect, buf: &mut Buffer| {
            (&tricontour_unfilled).render(area, buf);
        })
        .panel(1, 0, move |area: Rect, buf: &mut Buffer| {
            (&tricontour_filled).render(area, buf);
        })
        .panel(1, 1, move |area: Rect, buf: &mut Buffer| {
            (&tricolor).render(area, buf);
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
