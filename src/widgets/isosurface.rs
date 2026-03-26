//! 3D isosurface widget using simplified marching cubes.
//!
//! Extracts and renders a surface of constant value from a volumetric
//! scalar field, using triangle rasterisation with half-block shading
//! and painter's algorithm depth sorting.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{StatefulWidget, Widget};

use crate::colormap::{Colormap, Viridis};
use crate::drawing::draw_braille_line;
use crate::frame::PlotArea;
use crate::theme::Theme;
use crate::transform::{Camera3D, Camera3DState, data_to_screen};

/// A 3D isosurface widget.
///
/// Extracts a surface of constant value (iso-level) from a volumetric
/// scalar field using a simplified marching cubes algorithm. The resulting
/// triangles are depth-sorted and rendered with half-block shading.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// // Create a spherical scalar field
/// let n = 10;
/// let mut field = vec![vec![vec![0.0; n]; n]; n];
/// for iz in 0..n {
///     for iy in 0..n {
///         for ix in 0..n {
///             let x = ix as f64 / n as f64 * 2.0 - 1.0;
///             let y = iy as f64 / n as f64 * 2.0 - 1.0;
///             let z = iz as f64 / n as f64 * 2.0 - 1.0;
///             field[iz][iy][ix] = x * x + y * y + z * z;
///         }
///     }
/// }
/// let plot = Isosurface::new(field, 0.5)
///     .x_range(-1.0, 1.0)
///     .y_range(-1.0, 1.0)
///     .z_range(-1.0, 1.0)
///     .title("Sphere");
/// ```
pub struct Isosurface {
    /// 3D scalar field values[z][y][x].
    values: Vec<Vec<Vec<f64>>>,
    /// Iso-level threshold.
    level: f64,
    /// Grid bounds.
    x_range: (f64, f64),
    y_range: (f64, f64),
    z_range: (f64, f64),
    camera: Camera3D,
    colormap: Box<dyn Colormap>,
    title: Option<String>,
    theme: Theme,
}

impl Isosurface {
    /// Create a new isosurface from a 3D scalar field at the given iso-level.
    pub fn new(values: Vec<Vec<Vec<f64>>>, level: f64) -> Self {
        Self {
            values,
            level,
            x_range: (-1.0, 1.0),
            y_range: (-1.0, 1.0),
            z_range: (-1.0, 1.0),
            camera: Camera3D::default(),
            colormap: Box::new(Viridis),
            title: None,
            theme: Theme::get_default(),
        }
    }

    /// Set the x-axis range of the grid.
    pub fn x_range(mut self, lo: f64, hi: f64) -> Self {
        self.x_range = (lo, hi);
        self
    }

    /// Set the y-axis range of the grid.
    pub fn y_range(mut self, lo: f64, hi: f64) -> Self {
        self.y_range = (lo, hi);
        self
    }

    /// Set the z-axis range of the grid.
    pub fn z_range(mut self, lo: f64, hi: f64) -> Self {
        self.z_range = (lo, hi);
        self
    }

    /// Set the camera configuration.
    pub fn camera(mut self, cam: Camera3D) -> Self {
        self.camera = cam;
        self
    }

    /// Set the colormap used for triangle shading.
    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    /// Set the plot title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    fn render_with_camera(&self, camera: &Camera3D, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let px = area.x + 1;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(2);
        let ph = area.height.saturating_sub(title_height + 1);

        if pw < 2 || ph < 2 {
            return;
        }

        // Title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        let nz = self.values.len();
        if nz < 2 {
            return;
        }
        let ny = self.values[0].len();
        if ny < 2 {
            return;
        }
        let nx = self.values[0][0].len();
        if nx < 2 {
            return;
        }

        // Generate triangles via marching cubes
        let triangles = self.march_cubes(nx, ny, nz);

        if triangles.is_empty() {
            return;
        }

        // Normalize triangles to [-1, 1] and project
        let x_lo = self.x_range.0;
        let x_hi = self.x_range.1;
        let y_lo = self.y_range.0;
        let y_hi = self.y_range.1;
        let z_lo = self.z_range.0;
        let z_hi = self.z_range.1;
        let x_range = if x_hi == x_lo { 1.0 } else { x_hi - x_lo };
        let y_range = if y_hi == y_lo { 1.0 } else { y_hi - y_lo };
        let z_range = if z_hi == z_lo { 1.0 } else { z_hi - z_lo };

        // Project and collect screen-space triangles with depth
        struct ProjTri {
            screen: [(f64, f64); 3],
            avg_depth: f64,
            norm_z: f64, // average normalized z for coloring
        }

        let mut proj_tris: Vec<ProjTri> = Vec::new();

        for tri in &triangles {
            let mut screen = [(0.0, 0.0); 3];
            let mut total_depth = 0.0;
            let mut total_z = 0.0;
            for (i, &(tx, ty, tz)) in tri.iter().enumerate() {
                let nx_v = 2.0 * (tx - x_lo) / x_range - 1.0;
                let ny_v = 2.0 * (ty - y_lo) / y_range - 1.0;
                let nz_v = (2.0 * (tz - z_lo) / z_range - 1.0) * 0.8;
                let (sx, sy, depth) = camera.project(nx_v, ny_v, nz_v);
                screen[i] = (sx, sy);
                total_depth += depth;
                total_z += (tz - z_lo) / z_range;
            }
            proj_tris.push(ProjTri {
                screen,
                avg_depth: total_depth / 3.0,
                norm_z: (total_z / 3.0).clamp(0.0, 1.0),
            });
        }

        // Screen bounds
        let mut sx_min = f64::INFINITY;
        let mut sx_max = f64::NEG_INFINITY;
        let mut sy_min = f64::INFINITY;
        let mut sy_max = f64::NEG_INFINITY;

        for pt in &proj_tris {
            for &(sx, sy) in &pt.screen {
                sx_min = sx_min.min(sx);
                sx_max = sx_max.max(sx);
                sy_min = sy_min.min(sy);
                sy_max = sy_max.max(sy);
            }
        }

        // Apply zoom
        let zoom = 5.0 / camera.distance;
        {
            let cx = (sx_min + sx_max) / 2.0;
            let cy = (sy_min + sy_max) / 2.0;
            let hx = (sx_max - sx_min) / 2.0 / zoom;
            let hy = (sy_max - sy_min) / 2.0 / zoom;
            sx_min = cx - hx;
            sx_max = cx + hx;
            sy_min = cy - hy;
            sy_max = cy + hy;
        }

        // Sort back-to-front
        proj_tris.sort_by(|a, b| {
            b.avg_depth
                .partial_cmp(&a.avg_depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Render triangles
        for pt in &proj_tris {
            let face_color = self.colormap.color_at(pt.norm_z);

            let tri: [(i32, i32); 3] = [
                (
                    data_to_screen(pt.screen[0].0, sx_min, sx_max, px as f64, (px + pw - 1) as f64)
                        .round() as i32,
                    data_to_screen(pt.screen[0].1, sy_min, sy_max, py as f64, (py + ph - 1) as f64)
                        .round() as i32,
                ),
                (
                    data_to_screen(pt.screen[1].0, sx_min, sx_max, px as f64, (px + pw - 1) as f64)
                        .round() as i32,
                    data_to_screen(pt.screen[1].1, sy_min, sy_max, py as f64, (py + ph - 1) as f64)
                        .round() as i32,
                ),
                (
                    data_to_screen(pt.screen[2].0, sx_min, sx_max, px as f64, (px + pw - 1) as f64)
                        .round() as i32,
                    data_to_screen(pt.screen[2].1, sy_min, sy_max, py as f64, (py + ph - 1) as f64)
                        .round() as i32,
                ),
            ];

            // Bounding box
            let Some(bb_min_x) = tri.iter().map(|v| v.0).min() else {
                continue;
            };
            let Some(bb_max_x) = tri.iter().map(|v| v.0).max() else {
                continue;
            };
            let Some(bb_min_y) = tri.iter().map(|v| v.1).min() else {
                continue;
            };
            let Some(bb_max_y) = tri.iter().map(|v| v.1).max() else {
                continue;
            };

            // Fill using half-block characters
            let mut row = bb_min_y;
            while row <= bb_max_y {
                let upper_row = row;
                let lower_row = row + 1;
                for sx in bb_min_x..=bb_max_x {
                    let ux = sx as u16;
                    let uy = upper_row as u16;

                    let upper_in = ux >= px
                        && ux < px + pw
                        && uy >= py
                        && uy < py + ph
                        && point_in_triangle(sx, upper_row, &tri);

                    let lower_in = lower_row <= bb_max_y
                        && ux >= px
                        && ux < px + pw
                        && (lower_row as u16) >= py
                        && (lower_row as u16) < py + ph
                        && point_in_triangle(sx, lower_row, &tri);

                    if upper_in && lower_in {
                        let uf = vert_frac(upper_row, bb_min_y, bb_max_y);
                        let lf = vert_frac(lower_row, bb_min_y, bb_max_y);
                        let uc = shade_color(face_color, 1.0 - uf * 0.2);
                        let lc = shade_color(face_color, 1.0 - lf * 0.2);
                        buf[(ux, uy)]
                            .set_char(self.theme.chars.fill.half_upper)
                            .set_style(Style::default().fg(uc).bg(lc));
                    } else if upper_in {
                        let uf = vert_frac(upper_row, bb_min_y, bb_max_y);
                        let uc = shade_color(face_color, 1.0 - uf * 0.2);
                        buf[(ux, uy)]
                            .set_char(self.theme.chars.fill.half_upper)
                            .set_style(Style::default().fg(uc));
                    } else if lower_in {
                        let lf = vert_frac(lower_row, bb_min_y, bb_max_y);
                        let lc = shade_color(face_color, 1.0 - lf * 0.2);
                        buf[(ux, uy)]
                            .set_char(self.theme.chars.fill.half_lower)
                            .set_style(Style::default().fg(lc));
                    }
                }
                row += 2;
            }
        }

        // Draw 3D axis lines
        let pa = PlotArea {
            x: px,
            y: py,
            width: pw,
            height: ph,
            x_lo: 0.0,
            x_hi: 0.0,
            y_lo: 0.0,
            y_hi: 0.0,
            area: Rect::new(px, py, pw, ph),
        };
        let sb = ScreenBounds {
            sx_min,
            sx_max,
            sy_min,
            sy_max,
        };
        draw_axis_lines(camera, buf, &pa, &sb, &self.theme);
    }

    /// Simplified marching cubes: for each cell, check 8 corners and generate
    /// triangles at edge intersections via linear interpolation.
    fn march_cubes(
        &self,
        nx: usize,
        ny: usize,
        nz: usize,
    ) -> Vec<[(f64, f64, f64); 3]> {
        let mut triangles: Vec<[(f64, f64, f64); 3]> = Vec::new();

        let x_lo = self.x_range.0;
        let x_hi = self.x_range.1;
        let y_lo = self.y_range.0;
        let y_hi = self.y_range.1;
        let z_lo = self.z_range.0;
        let z_hi = self.z_range.1;

        let dx = (x_hi - x_lo) / (nx - 1) as f64;
        let dy = (y_hi - y_lo) / (ny - 1) as f64;
        let dz = (z_hi - z_lo) / (nz - 1) as f64;

        for iz in 0..nz - 1 {
            for iy in 0..ny - 1 {
                for ix in 0..nx - 1 {
                    // 8 corner values
                    let v = [
                        self.values[iz][iy][ix],         // 0
                        self.values[iz][iy][ix + 1],     // 1
                        self.values[iz][iy + 1][ix + 1], // 2
                        self.values[iz][iy + 1][ix],     // 3
                        self.values[iz + 1][iy][ix],     // 4
                        self.values[iz + 1][iy][ix + 1], // 5
                        self.values[iz + 1][iy + 1][ix + 1], // 6
                        self.values[iz + 1][iy + 1][ix], // 7
                    ];

                    // Compute case index
                    let mut case_idx: u8 = 0;
                    for (i, &val) in v.iter().enumerate() {
                        if val >= self.level {
                            case_idx |= 1 << i;
                        }
                    }

                    // Skip empty or fully inside cells
                    if case_idx == 0 || case_idx == 255 {
                        continue;
                    }

                    // Corner positions
                    let pos = [
                        (x_lo + ix as f64 * dx, y_lo + iy as f64 * dy, z_lo + iz as f64 * dz),
                        (x_lo + (ix + 1) as f64 * dx, y_lo + iy as f64 * dy, z_lo + iz as f64 * dz),
                        (x_lo + (ix + 1) as f64 * dx, y_lo + (iy + 1) as f64 * dy, z_lo + iz as f64 * dz),
                        (x_lo + ix as f64 * dx, y_lo + (iy + 1) as f64 * dy, z_lo + iz as f64 * dz),
                        (x_lo + ix as f64 * dx, y_lo + iy as f64 * dy, z_lo + (iz + 1) as f64 * dz),
                        (x_lo + (ix + 1) as f64 * dx, y_lo + iy as f64 * dy, z_lo + (iz + 1) as f64 * dz),
                        (x_lo + (ix + 1) as f64 * dx, y_lo + (iy + 1) as f64 * dy, z_lo + (iz + 1) as f64 * dz),
                        (x_lo + ix as f64 * dx, y_lo + (iy + 1) as f64 * dy, z_lo + (iz + 1) as f64 * dz),
                    ];

                    // 12 edges of the cube
                    let edges: [(usize, usize); 12] = [
                        (0, 1), (1, 2), (2, 3), (3, 0), // bottom face
                        (4, 5), (5, 6), (6, 7), (7, 4), // top face
                        (0, 4), (1, 5), (2, 6), (3, 7), // vertical edges
                    ];

                    // Find intersection points on edges that cross the iso-level
                    let mut edge_verts: [Option<(f64, f64, f64)>; 12] =
                        [None; 12];
                    for (ei, &(a, b)) in edges.iter().enumerate() {
                        let va = v[a];
                        let vb = v[b];
                        let a_in = va >= self.level;
                        let b_in = vb >= self.level;
                        if a_in != b_in {
                            let denom = vb - va;
                            let t = if denom.abs() < 1e-12 {
                                0.5
                            } else {
                                (self.level - va) / denom
                            };
                            let t = t.clamp(0.0, 1.0);
                            edge_verts[ei] = Some((
                                pos[a].0 + t * (pos[b].0 - pos[a].0),
                                pos[a].1 + t * (pos[b].1 - pos[a].1),
                                pos[a].2 + t * (pos[b].2 - pos[a].2),
                            ));
                        }
                    }

                    // Use the edge table to generate triangles for this case
                    let tri_edges = mc_triangulation(case_idx);
                    let mut i = 0;
                    while i + 2 < tri_edges.len() {
                        let e0 = tri_edges[i] as usize;
                        let e1 = tri_edges[i + 1] as usize;
                        let e2 = tri_edges[i + 2] as usize;
                        if e0 == 255 || e1 == 255 || e2 == 255 {
                            break;
                        }
                        if let (Some(p0), Some(p1), Some(p2)) =
                            (edge_verts[e0], edge_verts[e1], edge_verts[e2])
                        {
                            triangles.push([p0, p1, p2]);
                        }
                        i += 3;
                    }
                }
            }
        }

        triangles
    }
}

impl Widget for &Isosurface {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_with_camera(&self.camera, area, buf);
    }
}

impl StatefulWidget for &Isosurface {
    type State = Camera3DState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let camera = state.to_camera();
        self.render_with_camera(&camera, area, buf);
    }
}

// ── triangle hit-testing ────────────────────────────────────────────────────

fn point_in_triangle(px: i32, py: i32, tri: &[(i32, i32); 3]) -> bool {
    let mut sign = 0i32;
    for i in 0..3 {
        let (x0, y0) = tri[i];
        let (x1, y1) = tri[(i + 1) % 3];
        let cross =
            (x1 - x0) as i64 * (py - y0) as i64 - (y1 - y0) as i64 * (px - x0) as i64;
        if cross != 0 {
            let s = if cross > 0 { 1 } else { -1 };
            if sign == 0 {
                sign = s;
            } else if sign != s {
                return false;
            }
        }
    }
    true
}

// ── helper utilities ────────────────────────────────────────────────────────

fn vert_frac(row: i32, min_y: i32, max_y: i32) -> f64 {
    if max_y == min_y {
        0.5
    } else {
        (row - min_y) as f64 / (max_y - min_y) as f64
    }
}

fn shade_color(color: Color, factor: f64) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f64 * factor).round().clamp(0.0, 255.0) as u8,
            (g as f64 * factor).round().clamp(0.0, 255.0) as u8,
            (b as f64 * factor).round().clamp(0.0, 255.0) as u8,
        ),
        other => other,
    }
}

// ── screen bounds ───────────────────────────────────────────────────────────

struct ScreenBounds {
    sx_min: f64,
    sx_max: f64,
    sy_min: f64,
    sy_max: f64,
}

// ── axis helpers ────────────────────────────────────────────────────────────

fn draw_axis_lines(
    camera: &Camera3D,
    buf: &mut Buffer,
    pa: &PlotArea,
    sb: &ScreenBounds,
    theme: &Theme,
) {
    let (px, pw, py, ph) = (pa.x, pa.width, pa.y, pa.height);
    let (sx_min, sx_max, sy_min, sy_max) = (sb.sx_min, sb.sx_max, sb.sy_min, sb.sy_max);

    let to_sx = |v: f64| data_to_screen(v, sx_min, sx_max, px as f64, (px + pw - 1) as f64);
    let to_sy = |v: f64| data_to_screen(v, sy_min, sy_max, py as f64, (py + ph - 1) as f64);

    let origin = camera.project(-1.0, -1.0, -0.8);
    let x_tip = camera.project(1.0, -1.0, -0.8);
    let y_tip = camera.project(-1.0, 1.0, -0.8);
    let z_tip = camera.project(-1.0, -1.0, 0.8);

    let ox = to_sx(origin.0);
    let oy = to_sy(origin.1);

    let ax = to_sx(x_tip.0);
    let ay = to_sy(x_tip.1);
    draw_braille_line(buf, ox, oy, ax, ay, theme.x_axis_3d_color, pa);
    let axi = ax.round() as u16;
    let ayi = ay.round() as u16;
    if axi >= px && axi < px + pw && ayi >= py && ayi < py + ph {
        buf[(axi, ayi)].set_char('X').set_fg(theme.x_axis_3d_color);
    }

    let bx = to_sx(y_tip.0);
    let by = to_sy(y_tip.1);
    draw_braille_line(buf, ox, oy, bx, by, theme.y_axis_3d_color, pa);
    let bxi = bx.round() as u16;
    let byi = by.round() as u16;
    if bxi >= px && bxi < px + pw && byi >= py && byi < py + ph {
        buf[(bxi, byi)].set_char('Y').set_fg(theme.y_axis_3d_color);
    }

    let cx = to_sx(z_tip.0);
    let cy = to_sy(z_tip.1);
    draw_braille_line(buf, ox, oy, cx, cy, theme.z_axis_3d_color, pa);
    let cxi = cx.round() as u16;
    let cyi = cy.round() as u16;
    if cxi >= px && cxi < px + pw && cyi >= py && cyi < py + ph {
        buf[(cxi, cyi)].set_char('Z').set_fg(theme.z_axis_3d_color);
    }
}

// ── Marching cubes lookup table ─────────────────────────────────────────────

/// Returns the triangle edge indices for a given marching cubes case (0-255).
/// Uses a compressed table covering all 256 cases by symmetry.
/// Returns a slice of edge indices in groups of 3 (triangle vertices).
/// Terminated by 255 sentinel values.
fn mc_triangulation(case: u8) -> &'static [u8] {
    // Full marching cubes triangulation table.
    // Each entry is a list of edge indices forming triangles, terminated by 255.
    const TABLE: [[u8; 16]; 256] = [
        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 0
        [0, 8, 3, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 1
        [0, 1, 9, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 2
        [1, 8, 3, 9, 8, 1, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 3
        [1, 2, 10, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 4
        [0, 8, 3, 1, 2, 10, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 5
        [9, 2, 10, 0, 2, 9, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 6
        [2, 8, 3, 2, 10, 8, 10, 9, 8, 255, 255, 255, 255, 255, 255, 255], // 7
        [3, 11, 2, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 8
        [0, 11, 2, 8, 11, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 9
        [1, 9, 0, 2, 3, 11, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 10
        [1, 11, 2, 1, 9, 11, 9, 8, 11, 255, 255, 255, 255, 255, 255, 255], // 11
        [3, 10, 1, 11, 10, 3, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 12
        [0, 10, 1, 0, 8, 10, 8, 11, 10, 255, 255, 255, 255, 255, 255, 255], // 13
        [3, 9, 0, 3, 11, 9, 11, 10, 9, 255, 255, 255, 255, 255, 255, 255], // 14
        [9, 8, 10, 10, 8, 11, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 15
        [4, 7, 8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 16
        [4, 3, 0, 7, 3, 4, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 17
        [0, 1, 9, 8, 4, 7, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 18
        [4, 1, 9, 4, 7, 1, 7, 3, 1, 255, 255, 255, 255, 255, 255, 255], // 19
        [1, 2, 10, 8, 4, 7, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 20
        [3, 4, 7, 3, 0, 4, 1, 2, 10, 255, 255, 255, 255, 255, 255, 255], // 21
        [9, 2, 10, 9, 0, 2, 8, 4, 7, 255, 255, 255, 255, 255, 255, 255], // 22
        [2, 10, 9, 2, 9, 7, 2, 7, 3, 7, 9, 4, 255, 255, 255, 255], // 23
        [8, 4, 7, 3, 11, 2, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 24
        [11, 4, 7, 11, 2, 4, 2, 0, 4, 255, 255, 255, 255, 255, 255, 255], // 25
        [9, 0, 1, 8, 4, 7, 2, 3, 11, 255, 255, 255, 255, 255, 255, 255], // 26
        [4, 7, 11, 9, 4, 11, 9, 11, 2, 9, 2, 1, 255, 255, 255, 255], // 27
        [3, 10, 1, 3, 11, 10, 7, 8, 4, 255, 255, 255, 255, 255, 255, 255], // 28
        [1, 11, 10, 1, 4, 11, 1, 0, 4, 7, 11, 4, 255, 255, 255, 255], // 29
        [4, 7, 8, 9, 0, 11, 9, 11, 10, 11, 0, 3, 255, 255, 255, 255], // 30
        [4, 7, 11, 4, 11, 9, 9, 11, 10, 255, 255, 255, 255, 255, 255, 255], // 31
        [9, 5, 4, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 32
        [9, 5, 4, 0, 8, 3, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 33
        [0, 5, 4, 1, 5, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 34
        [8, 5, 4, 8, 3, 5, 3, 1, 5, 255, 255, 255, 255, 255, 255, 255], // 35
        [1, 2, 10, 9, 5, 4, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 36
        [3, 0, 8, 1, 2, 10, 4, 9, 5, 255, 255, 255, 255, 255, 255, 255], // 37
        [5, 2, 10, 5, 4, 2, 4, 0, 2, 255, 255, 255, 255, 255, 255, 255], // 38
        [2, 10, 5, 3, 2, 5, 3, 5, 4, 3, 4, 8, 255, 255, 255, 255], // 39
        [9, 5, 4, 2, 3, 11, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 40
        [0, 11, 2, 0, 8, 11, 4, 9, 5, 255, 255, 255, 255, 255, 255, 255], // 41
        [0, 5, 4, 0, 1, 5, 2, 3, 11, 255, 255, 255, 255, 255, 255, 255], // 42
        [2, 1, 5, 2, 5, 8, 2, 8, 11, 4, 8, 5, 255, 255, 255, 255], // 43
        [10, 3, 11, 10, 1, 3, 9, 5, 4, 255, 255, 255, 255, 255, 255, 255], // 44
        [4, 9, 5, 0, 8, 1, 8, 10, 1, 8, 11, 10, 255, 255, 255, 255], // 45
        [5, 4, 0, 5, 0, 11, 5, 11, 10, 11, 0, 3, 255, 255, 255, 255], // 46
        [5, 4, 8, 5, 8, 10, 10, 8, 11, 255, 255, 255, 255, 255, 255, 255], // 47
        [9, 7, 8, 5, 7, 9, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 48
        [9, 3, 0, 9, 5, 3, 5, 7, 3, 255, 255, 255, 255, 255, 255, 255], // 49
        [0, 7, 8, 0, 1, 7, 1, 5, 7, 255, 255, 255, 255, 255, 255, 255], // 50
        [1, 5, 3, 3, 5, 7, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 51
        [9, 7, 8, 9, 5, 7, 10, 1, 2, 255, 255, 255, 255, 255, 255, 255], // 52
        [10, 1, 2, 9, 5, 0, 5, 3, 0, 5, 7, 3, 255, 255, 255, 255], // 53
        [8, 0, 2, 8, 2, 5, 8, 5, 7, 10, 5, 2, 255, 255, 255, 255], // 54
        [2, 10, 5, 2, 5, 3, 3, 5, 7, 255, 255, 255, 255, 255, 255, 255], // 55
        [7, 9, 5, 7, 8, 9, 3, 11, 2, 255, 255, 255, 255, 255, 255, 255], // 56
        [9, 5, 7, 9, 7, 2, 9, 2, 0, 2, 7, 11, 255, 255, 255, 255], // 57
        [2, 3, 11, 0, 1, 8, 1, 7, 8, 1, 5, 7, 255, 255, 255, 255], // 58
        [11, 2, 1, 11, 1, 7, 7, 1, 5, 255, 255, 255, 255, 255, 255, 255], // 59
        [9, 5, 8, 8, 5, 7, 10, 1, 3, 10, 3, 11, 255, 255, 255, 255], // 60
        [5, 7, 0, 5, 0, 9, 7, 11, 0, 1, 0, 10, 11, 10, 0, 255], // 61
        [11, 10, 0, 11, 0, 3, 10, 5, 0, 8, 0, 7, 5, 7, 0, 255], // 62
        [11, 10, 5, 7, 11, 5, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 63
        [10, 6, 5, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 64
        [0, 8, 3, 5, 10, 6, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 65
        [9, 0, 1, 5, 10, 6, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 66
        [1, 8, 3, 1, 9, 8, 5, 10, 6, 255, 255, 255, 255, 255, 255, 255], // 67
        [1, 6, 5, 2, 6, 1, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 68
        [1, 6, 5, 1, 2, 6, 3, 0, 8, 255, 255, 255, 255, 255, 255, 255], // 69
        [9, 6, 5, 9, 0, 6, 0, 2, 6, 255, 255, 255, 255, 255, 255, 255], // 70
        [5, 9, 8, 5, 8, 2, 5, 2, 6, 3, 2, 8, 255, 255, 255, 255], // 71
        [2, 3, 11, 10, 6, 5, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 72
        [11, 0, 8, 11, 2, 0, 10, 6, 5, 255, 255, 255, 255, 255, 255, 255], // 73
        [0, 1, 9, 2, 3, 11, 5, 10, 6, 255, 255, 255, 255, 255, 255, 255], // 74
        [5, 10, 6, 1, 9, 2, 9, 11, 2, 9, 8, 11, 255, 255, 255, 255], // 75
        [6, 3, 11, 6, 5, 3, 5, 1, 3, 255, 255, 255, 255, 255, 255, 255], // 76
        [0, 8, 11, 0, 11, 5, 0, 5, 1, 5, 11, 6, 255, 255, 255, 255], // 77
        [3, 11, 6, 0, 3, 6, 0, 6, 5, 0, 5, 9, 255, 255, 255, 255], // 78
        [6, 5, 9, 6, 9, 11, 11, 9, 8, 255, 255, 255, 255, 255, 255, 255], // 79
        [5, 10, 6, 4, 7, 8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 80
        [4, 3, 0, 4, 7, 3, 6, 5, 10, 255, 255, 255, 255, 255, 255, 255], // 81
        [1, 9, 0, 5, 10, 6, 8, 4, 7, 255, 255, 255, 255, 255, 255, 255], // 82
        [10, 6, 5, 1, 9, 7, 1, 7, 3, 7, 9, 4, 255, 255, 255, 255], // 83
        [6, 1, 2, 6, 5, 1, 4, 7, 8, 255, 255, 255, 255, 255, 255, 255], // 84
        [1, 2, 5, 5, 2, 6, 3, 0, 4, 3, 4, 7, 255, 255, 255, 255], // 85
        [8, 4, 7, 9, 0, 5, 0, 6, 5, 0, 2, 6, 255, 255, 255, 255], // 86
        [7, 3, 9, 7, 9, 4, 3, 2, 9, 5, 9, 6, 2, 6, 9, 255], // 87
        [3, 11, 2, 7, 8, 4, 10, 6, 5, 255, 255, 255, 255, 255, 255, 255], // 88
        [5, 10, 6, 4, 7, 2, 4, 2, 0, 2, 7, 11, 255, 255, 255, 255], // 89
        [0, 1, 9, 4, 7, 8, 2, 3, 11, 5, 10, 6, 255, 255, 255, 255], // 90
        [9, 2, 1, 9, 11, 2, 9, 4, 11, 7, 11, 4, 5, 10, 6, 255], // 91
        [8, 4, 7, 3, 11, 5, 3, 5, 1, 5, 11, 6, 255, 255, 255, 255], // 92
        [5, 1, 11, 5, 11, 6, 1, 0, 11, 7, 11, 4, 0, 4, 11, 255], // 93
        [0, 5, 9, 0, 6, 5, 0, 3, 6, 11, 6, 3, 8, 4, 7, 255], // 94
        [6, 5, 9, 6, 9, 11, 4, 7, 9, 7, 11, 9, 255, 255, 255, 255], // 95
        [10, 4, 9, 6, 4, 10, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 96
        [4, 10, 6, 4, 9, 10, 0, 8, 3, 255, 255, 255, 255, 255, 255, 255], // 97
        [10, 0, 1, 10, 6, 0, 6, 4, 0, 255, 255, 255, 255, 255, 255, 255], // 98
        [8, 3, 1, 8, 1, 6, 8, 6, 4, 6, 1, 10, 255, 255, 255, 255], // 99
        [1, 4, 9, 1, 2, 4, 2, 6, 4, 255, 255, 255, 255, 255, 255, 255], // 100
        [3, 0, 8, 1, 2, 9, 2, 4, 9, 2, 6, 4, 255, 255, 255, 255], // 101
        [0, 2, 4, 4, 2, 6, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 102
        [8, 3, 2, 8, 2, 4, 4, 2, 6, 255, 255, 255, 255, 255, 255, 255], // 103
        [10, 4, 9, 10, 6, 4, 11, 2, 3, 255, 255, 255, 255, 255, 255, 255], // 104
        [0, 8, 2, 2, 8, 11, 4, 9, 10, 4, 10, 6, 255, 255, 255, 255], // 105
        [3, 11, 2, 0, 1, 6, 0, 6, 4, 6, 1, 10, 255, 255, 255, 255], // 106
        [6, 4, 1, 6, 1, 10, 4, 8, 1, 2, 1, 11, 8, 11, 1, 255], // 107
        [9, 6, 4, 9, 3, 6, 9, 1, 3, 11, 6, 3, 255, 255, 255, 255], // 108
        [8, 11, 1, 8, 1, 0, 11, 6, 1, 9, 1, 4, 6, 4, 1, 255], // 109
        [3, 11, 6, 3, 6, 0, 0, 6, 4, 255, 255, 255, 255, 255, 255, 255], // 110
        [6, 4, 8, 11, 6, 8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 111
        [7, 10, 6, 7, 8, 10, 8, 9, 10, 255, 255, 255, 255, 255, 255, 255], // 112
        [0, 7, 3, 0, 10, 7, 0, 9, 10, 6, 7, 10, 255, 255, 255, 255], // 113
        [10, 6, 7, 1, 10, 7, 1, 7, 8, 1, 8, 0, 255, 255, 255, 255], // 114
        [10, 6, 7, 10, 7, 1, 1, 7, 3, 255, 255, 255, 255, 255, 255, 255], // 115
        [1, 2, 6, 1, 6, 8, 1, 8, 9, 8, 6, 7, 255, 255, 255, 255], // 116
        [2, 6, 9, 2, 9, 1, 6, 7, 9, 0, 9, 3, 7, 3, 9, 255], // 117
        [7, 8, 0, 7, 0, 6, 6, 0, 2, 255, 255, 255, 255, 255, 255, 255], // 118
        [7, 3, 2, 6, 7, 2, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 119
        [2, 3, 11, 10, 6, 8, 10, 8, 9, 8, 6, 7, 255, 255, 255, 255], // 120
        [2, 0, 7, 2, 7, 11, 0, 9, 7, 6, 7, 10, 9, 10, 7, 255], // 121
        [1, 8, 0, 1, 7, 8, 1, 10, 7, 6, 7, 10, 2, 3, 11, 255], // 122
        [11, 2, 1, 11, 1, 7, 10, 6, 1, 6, 7, 1, 255, 255, 255, 255], // 123
        [8, 9, 6, 8, 6, 7, 9, 1, 6, 11, 6, 3, 1, 3, 6, 255], // 124
        [0, 9, 1, 11, 6, 7, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 125
        [7, 8, 0, 7, 0, 6, 3, 11, 0, 11, 6, 0, 255, 255, 255, 255], // 126
        [7, 11, 6, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 127
        [7, 6, 11, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 128
        [3, 0, 8, 11, 7, 6, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 129
        [0, 1, 9, 11, 7, 6, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 130
        [8, 1, 9, 8, 3, 1, 11, 7, 6, 255, 255, 255, 255, 255, 255, 255], // 131
        [10, 1, 2, 6, 11, 7, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 132
        [1, 2, 10, 3, 0, 8, 6, 11, 7, 255, 255, 255, 255, 255, 255, 255], // 133
        [2, 9, 0, 2, 10, 9, 6, 11, 7, 255, 255, 255, 255, 255, 255, 255], // 134
        [6, 11, 7, 2, 10, 3, 10, 8, 3, 10, 9, 8, 255, 255, 255, 255], // 135
        [7, 2, 3, 6, 2, 7, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 136
        [7, 0, 8, 7, 6, 0, 6, 2, 0, 255, 255, 255, 255, 255, 255, 255], // 137
        [2, 7, 6, 2, 3, 7, 0, 1, 9, 255, 255, 255, 255, 255, 255, 255], // 138
        [1, 6, 2, 1, 8, 6, 1, 9, 8, 8, 7, 6, 255, 255, 255, 255], // 139
        [10, 7, 6, 10, 1, 7, 1, 3, 7, 255, 255, 255, 255, 255, 255, 255], // 140
        [10, 7, 6, 1, 7, 10, 1, 8, 7, 1, 0, 8, 255, 255, 255, 255], // 141
        [0, 3, 7, 0, 7, 10, 0, 10, 9, 6, 10, 7, 255, 255, 255, 255], // 142
        [7, 6, 10, 7, 10, 8, 8, 10, 9, 255, 255, 255, 255, 255, 255, 255], // 143
        [6, 8, 4, 11, 8, 6, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 144
        [3, 6, 11, 3, 0, 6, 0, 4, 6, 255, 255, 255, 255, 255, 255, 255], // 145
        [8, 6, 11, 8, 4, 6, 9, 0, 1, 255, 255, 255, 255, 255, 255, 255], // 146
        [9, 4, 6, 9, 6, 3, 9, 3, 1, 11, 3, 6, 255, 255, 255, 255], // 147
        [6, 8, 4, 6, 11, 8, 2, 10, 1, 255, 255, 255, 255, 255, 255, 255], // 148
        [1, 2, 10, 3, 0, 11, 0, 6, 11, 0, 4, 6, 255, 255, 255, 255], // 149
        [4, 11, 8, 4, 6, 11, 0, 2, 9, 2, 10, 9, 255, 255, 255, 255], // 150
        [10, 9, 3, 10, 3, 2, 9, 4, 3, 11, 3, 6, 4, 6, 3, 255], // 151
        [8, 2, 3, 8, 4, 2, 4, 6, 2, 255, 255, 255, 255, 255, 255, 255], // 152
        [0, 4, 2, 4, 6, 2, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 153
        [1, 9, 0, 2, 3, 4, 2, 4, 6, 4, 3, 8, 255, 255, 255, 255], // 154
        [1, 9, 4, 1, 4, 2, 2, 4, 6, 255, 255, 255, 255, 255, 255, 255], // 155
        [8, 1, 3, 8, 6, 1, 8, 4, 6, 6, 10, 1, 255, 255, 255, 255], // 156
        [10, 1, 0, 10, 0, 6, 6, 0, 4, 255, 255, 255, 255, 255, 255, 255], // 157
        [4, 6, 3, 4, 3, 8, 6, 10, 3, 0, 3, 9, 10, 9, 3, 255], // 158
        [10, 9, 4, 6, 10, 4, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 159
        [4, 9, 5, 7, 6, 11, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 160
        [0, 8, 3, 4, 9, 5, 11, 7, 6, 255, 255, 255, 255, 255, 255, 255], // 161
        [5, 0, 1, 5, 4, 0, 7, 6, 11, 255, 255, 255, 255, 255, 255, 255], // 162
        [11, 7, 6, 8, 3, 4, 3, 5, 4, 3, 1, 5, 255, 255, 255, 255], // 163
        [9, 5, 4, 10, 1, 2, 7, 6, 11, 255, 255, 255, 255, 255, 255, 255], // 164
        [6, 11, 7, 1, 2, 10, 0, 8, 3, 4, 9, 5, 255, 255, 255, 255], // 165
        [7, 6, 11, 5, 4, 10, 4, 2, 10, 4, 0, 2, 255, 255, 255, 255], // 166
        [3, 4, 8, 3, 5, 4, 3, 2, 5, 10, 5, 2, 11, 7, 6, 255], // 167
        [7, 2, 3, 7, 6, 2, 5, 4, 9, 255, 255, 255, 255, 255, 255, 255], // 168
        [9, 5, 4, 0, 8, 6, 0, 6, 2, 6, 8, 7, 255, 255, 255, 255], // 169
        [3, 6, 2, 3, 7, 6, 1, 5, 0, 5, 4, 0, 255, 255, 255, 255], // 170
        [6, 2, 8, 6, 8, 7, 2, 1, 8, 4, 8, 5, 1, 5, 8, 255], // 171
        [9, 5, 4, 10, 1, 6, 1, 7, 6, 1, 3, 7, 255, 255, 255, 255], // 172
        [1, 6, 10, 1, 7, 6, 1, 0, 7, 8, 7, 0, 9, 5, 4, 255], // 173
        [4, 0, 10, 4, 10, 5, 0, 3, 10, 6, 10, 7, 3, 7, 10, 255], // 174
        [7, 6, 10, 7, 10, 8, 5, 4, 10, 4, 8, 10, 255, 255, 255, 255], // 175
        [6, 9, 5, 6, 11, 9, 11, 8, 9, 255, 255, 255, 255, 255, 255, 255], // 176
        [3, 6, 11, 0, 6, 3, 0, 5, 6, 0, 9, 5, 255, 255, 255, 255], // 177
        [0, 11, 8, 0, 5, 11, 0, 1, 5, 5, 6, 11, 255, 255, 255, 255], // 178
        [6, 11, 3, 6, 3, 5, 5, 3, 1, 255, 255, 255, 255, 255, 255, 255], // 179
        [1, 2, 10, 9, 5, 11, 9, 11, 8, 11, 5, 6, 255, 255, 255, 255], // 180
        [0, 11, 3, 0, 6, 11, 0, 9, 6, 5, 6, 9, 1, 2, 10, 255], // 181
        [11, 8, 5, 11, 5, 6, 8, 0, 5, 10, 5, 2, 0, 2, 5, 255], // 182
        [6, 11, 3, 6, 3, 5, 2, 10, 3, 10, 5, 3, 255, 255, 255, 255], // 183
        [5, 8, 9, 5, 2, 8, 5, 6, 2, 3, 8, 2, 255, 255, 255, 255], // 184
        [9, 5, 6, 9, 6, 0, 0, 6, 2, 255, 255, 255, 255, 255, 255, 255], // 185
        [1, 5, 8, 1, 8, 0, 5, 6, 8, 3, 8, 2, 6, 2, 8, 255], // 186
        [1, 5, 6, 2, 1, 6, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 187
        [1, 3, 6, 1, 6, 10, 3, 8, 6, 5, 6, 9, 8, 9, 6, 255], // 188
        [10, 1, 0, 10, 0, 6, 9, 5, 0, 5, 6, 0, 255, 255, 255, 255], // 189
        [0, 3, 8, 5, 6, 10, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 190
        [10, 5, 6, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 191
        [11, 5, 10, 7, 5, 11, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 192
        [11, 5, 10, 11, 7, 5, 8, 3, 0, 255, 255, 255, 255, 255, 255, 255], // 193
        [5, 11, 7, 5, 10, 11, 1, 9, 0, 255, 255, 255, 255, 255, 255, 255], // 194
        [10, 7, 5, 10, 11, 7, 9, 8, 1, 8, 3, 1, 255, 255, 255, 255], // 195
        [11, 1, 2, 11, 7, 1, 7, 5, 1, 255, 255, 255, 255, 255, 255, 255], // 196
        [0, 8, 3, 1, 2, 7, 1, 7, 5, 7, 2, 11, 255, 255, 255, 255], // 197
        [9, 7, 5, 9, 2, 7, 9, 0, 2, 2, 11, 7, 255, 255, 255, 255], // 198
        [7, 5, 2, 7, 2, 11, 5, 9, 2, 3, 2, 8, 9, 8, 2, 255], // 199
        [2, 5, 10, 2, 3, 5, 3, 7, 5, 255, 255, 255, 255, 255, 255, 255], // 200
        [8, 2, 0, 8, 5, 2, 8, 7, 5, 10, 2, 5, 255, 255, 255, 255], // 201
        [9, 0, 1, 5, 10, 3, 5, 3, 7, 3, 10, 2, 255, 255, 255, 255], // 202
        [9, 8, 2, 9, 2, 1, 8, 7, 2, 10, 2, 5, 7, 5, 2, 255], // 203
        [1, 3, 5, 3, 7, 5, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 204
        [0, 8, 7, 0, 7, 1, 1, 7, 5, 255, 255, 255, 255, 255, 255, 255], // 205
        [9, 0, 3, 9, 3, 5, 5, 3, 7, 255, 255, 255, 255, 255, 255, 255], // 206
        [9, 8, 7, 5, 9, 7, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 207
        [5, 8, 4, 5, 10, 8, 10, 11, 8, 255, 255, 255, 255, 255, 255, 255], // 208
        [5, 0, 4, 5, 11, 0, 5, 10, 11, 11, 3, 0, 255, 255, 255, 255], // 209
        [0, 1, 9, 8, 4, 10, 8, 10, 11, 10, 4, 5, 255, 255, 255, 255], // 210
        [10, 11, 4, 10, 4, 5, 11, 3, 4, 9, 4, 1, 3, 1, 4, 255], // 211
        [2, 5, 1, 2, 8, 5, 2, 11, 8, 4, 5, 8, 255, 255, 255, 255], // 212
        [0, 4, 11, 0, 11, 3, 4, 5, 11, 2, 11, 1, 5, 1, 11, 255], // 213
        [0, 2, 5, 0, 5, 9, 2, 11, 5, 4, 5, 8, 11, 8, 5, 255], // 214
        [9, 4, 5, 2, 11, 3, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 215
        [2, 5, 10, 3, 5, 2, 3, 4, 5, 3, 8, 4, 255, 255, 255, 255], // 216
        [5, 10, 2, 5, 2, 4, 4, 2, 0, 255, 255, 255, 255, 255, 255, 255], // 217
        [3, 10, 2, 3, 5, 10, 3, 8, 5, 4, 5, 8, 0, 1, 9, 255], // 218
        [5, 10, 2, 5, 2, 4, 1, 9, 2, 9, 4, 2, 255, 255, 255, 255], // 219
        [8, 4, 5, 8, 5, 3, 3, 5, 1, 255, 255, 255, 255, 255, 255, 255], // 220
        [0, 4, 5, 1, 0, 5, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 221
        [8, 4, 5, 8, 5, 3, 9, 0, 5, 0, 3, 5, 255, 255, 255, 255], // 222
        [9, 4, 5, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 223
        [4, 11, 7, 4, 9, 11, 9, 10, 11, 255, 255, 255, 255, 255, 255, 255], // 224
        [0, 8, 3, 4, 9, 7, 9, 11, 7, 9, 10, 11, 255, 255, 255, 255], // 225
        [1, 10, 11, 1, 11, 4, 1, 4, 0, 7, 4, 11, 255, 255, 255, 255], // 226
        [3, 1, 4, 3, 4, 8, 1, 10, 4, 7, 4, 11, 10, 11, 4, 255], // 227
        [4, 11, 7, 9, 11, 4, 9, 2, 11, 9, 1, 2, 255, 255, 255, 255], // 228
        [9, 7, 4, 9, 11, 7, 9, 1, 11, 2, 11, 1, 0, 8, 3, 255], // 229
        [11, 7, 4, 11, 4, 2, 2, 4, 0, 255, 255, 255, 255, 255, 255, 255], // 230
        [11, 7, 4, 11, 4, 2, 8, 3, 4, 3, 2, 4, 255, 255, 255, 255], // 231
        [2, 9, 10, 2, 7, 9, 2, 3, 7, 7, 4, 9, 255, 255, 255, 255], // 232
        [9, 10, 7, 9, 7, 4, 10, 2, 7, 8, 7, 0, 2, 0, 7, 255], // 233
        [3, 7, 10, 3, 10, 2, 7, 4, 10, 1, 10, 0, 4, 0, 10, 255], // 234
        [1, 10, 2, 8, 7, 4, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 235
        [4, 9, 1, 4, 1, 7, 7, 1, 3, 255, 255, 255, 255, 255, 255, 255], // 236
        [4, 9, 1, 4, 1, 7, 0, 8, 1, 8, 7, 1, 255, 255, 255, 255], // 237
        [4, 0, 3, 7, 4, 3, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 238
        [4, 8, 7, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 239
        [9, 10, 8, 10, 11, 8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 240
        [3, 0, 9, 3, 9, 11, 11, 9, 10, 255, 255, 255, 255, 255, 255, 255], // 241
        [0, 1, 10, 0, 10, 8, 8, 10, 11, 255, 255, 255, 255, 255, 255, 255], // 242
        [3, 1, 10, 11, 3, 10, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 243
        [1, 2, 11, 1, 11, 9, 9, 11, 8, 255, 255, 255, 255, 255, 255, 255], // 244
        [3, 0, 9, 3, 9, 11, 1, 2, 9, 2, 11, 9, 255, 255, 255, 255], // 245
        [0, 2, 11, 8, 0, 11, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 246
        [3, 2, 11, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 247
        [2, 3, 8, 2, 8, 10, 10, 8, 9, 255, 255, 255, 255, 255, 255, 255], // 248
        [9, 10, 2, 0, 9, 2, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 249
        [2, 3, 8, 2, 8, 10, 0, 1, 8, 1, 10, 8, 255, 255, 255, 255], // 250
        [1, 10, 2, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 251
        [1, 3, 8, 9, 1, 8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 252
        [0, 9, 1, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 253
        [0, 3, 8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 254
        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255], // 255
    ];

    &TABLE[case as usize]
}
