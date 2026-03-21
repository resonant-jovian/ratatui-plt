//! Multi-panel triangulation demo: TriPlot, TriColor, and TriContour side by side.

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::prelude::*;
use ratatui_plt::ticker::MaxNLocator;
use ratatui_plt::triangulation::Triangulation;

fn parse_theme() -> Theme {
    match std::env::args().nth(1).as_deref() {
        Some("light") => Theme::light(),
        Some("minimal") => Theme::minimal(),
        Some("publication") => Theme::publication(),
        Some("solarized") => Theme::solarized(),
        Some("dark") | None => Theme::dark(),
        Some(other) => {
            eprintln!(
                "Unknown theme '{other}'. Available: dark, light, minimal, publication, solarized"
            );
            std::process::exit(1);
        }
    }
}

type Vertices = Vec<(f64, f64)>;
type Triangles = Vec<(usize, usize, usize)>;

/// Build an 8x8 regular grid on [-2, 2] x [-2, 2] with 64 vertices and 98 triangles.
///
/// Each grid cell is split into two triangles along the diagonal.
fn make_grid_triangulation() -> (Vertices, Triangles) {
    let n = 8;
    let lo = -2.0_f64;
    let hi = 2.0_f64;
    let step = (hi - lo) / (n - 1) as f64;

    let mut vertices = Vec::with_capacity(n * n);
    for row in 0..n {
        for col in 0..n {
            vertices.push((lo + col as f64 * step, lo + row as f64 * step));
        }
    }

    let mut triangles = Vec::with_capacity(2 * (n - 1) * (n - 1));
    for row in 0..(n - 1) {
        for col in 0..(n - 1) {
            let bl = row * n + col;
            let br = bl + 1;
            let tl = bl + n;
            let tr = tl + 1;
            // Lower-left triangle
            triangles.push((bl, br, tl));
            // Upper-right triangle
            triangles.push((br, tr, tl));
        }
    }

    (vertices, triangles)
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    // --- Left panel: TriPlot (mesh edges) ---
    let (verts, tris) = make_grid_triangulation();
    let tri_plot = TriPlot::new(Triangulation::from_explicit(verts, tris))
        .title("Mesh Edges")
        .edge_color(Color::Cyan)
        .x_axis(Axis::new().locator(MaxNLocator::new(4)))
        .y_axis(Axis::new().locator(MaxNLocator::new(4)));

    // --- Middle panel: TriColor (colored faces) ---
    let (verts, tris) = make_grid_triangulation();
    // Face values: distance from the origin of each triangle's centroid
    let face_values: Vec<f64> = tris
        .iter()
        .map(|&(a, b, c)| {
            let (ax, ay) = verts[a];
            let (bx, by) = verts[b];
            let (cx, cy) = verts[c];
            let cx_mid = (ax + bx + cx) / 3.0;
            let cy_mid = (ay + by + cy) / 3.0;
            (cx_mid * cx_mid + cy_mid * cy_mid).sqrt()
        })
        .collect();
    let tri_color = TriColor::new(Triangulation::from_explicit(verts, tris))
        .face_values(face_values)
        .colormap(Viridis)
        .title("Colored Faces")
        .x_axis(Axis::new().locator(MaxNLocator::new(4)))
        .y_axis(Axis::new().locator(MaxNLocator::new(4)));

    // --- Right panel: TriContour (contour lines on vertex values) ---
    let (verts, tris) = make_grid_triangulation();
    // Vertex values: distance from the origin
    let vertex_values: Vec<f64> = verts.iter().map(|&(x, y)| (x * x + y * y).sqrt()).collect();
    let tri_contour = TriContour::new(Triangulation::from_explicit(verts, tris))
        .vertex_values(vertex_values)
        .levels_auto(10)
        .title("Contour Lines")
        .x_axis(Axis::new().locator(MaxNLocator::new(4)))
        .y_axis(Axis::new().locator(MaxNLocator::new(4)));

    // Multi-panel layout: 1 row, 3 columns
    let panel = MultiPanel::new(1, 3)
        .width_ratios(vec![1.0, 1.0, 1.0])
        .gap(1)
        .suptitle("Triangulation Plots (q to quit)")
        .panel(0, 0, move |area: Rect, buf: &mut Buffer| {
            (&tri_plot).render(area, buf);
        })
        .panel(0, 1, move |area: Rect, buf: &mut Buffer| {
            (&tri_color).render(area, buf);
        })
        .panel(0, 2, move |area: Rect, buf: &mut Buffer| {
            (&tri_contour).render(area, buf);
        });

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(&panel, area);
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
