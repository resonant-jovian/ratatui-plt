//! 3D voxel grid widget — colored cubes on a discrete 3D grid.
//!
//! Renders occupied voxels as colored cube faces using the painter's algorithm
//! and half-block characters for doubled vertical resolution.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{StatefulWidget, Widget};

use crate::drawing::draw_braille_line;
use crate::frame::PlotArea;
use crate::theme::Theme;
use crate::transform::{Camera3D, Camera3DState, data_to_screen};

/// A 3D voxel grid widget.
///
/// Renders a discrete 3D grid of colored cubes (like Minecraft blocks).
/// Each occupied voxel is drawn as visible cube faces using half-block
/// shading with painter's algorithm depth sorting.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let mut grid = vec![vec![vec![false; 4]; 4]; 4];
/// grid[0][0][0] = true;
/// grid[1][1][1] = true;
/// grid[2][2][2] = true;
/// let plot = Voxels::new(grid)
///     .camera(Camera3D::new().azimuth(-60.0).elevation(30.0))
///     .title("Voxels");
/// ```
pub struct Voxels {
    /// 3D boolean grid: occupied[z][y][x] = true means occupied.
    occupied: Vec<Vec<Vec<bool>>>,
    /// Optional color per voxel: colors[z][y][x].
    colors: Option<Vec<Vec<Vec<Color>>>>,
    camera: Camera3D,
    title: Option<String>,
    default_color: Option<Color>,
    theme: Theme,
}

impl Voxels {
    /// Create a new voxel grid from a 3D boolean occupancy grid.
    ///
    /// The grid is indexed as `occupied[z][y][x]`.
    pub fn new(occupied: Vec<Vec<Vec<bool>>>) -> Self {
        Self {
            occupied,
            colors: None,
            camera: Camera3D::default(),
            title: None,
            default_color: None,
            theme: Theme::get_default(),
        }
    }

    /// Set per-voxel colors. The grid must match the occupancy grid dimensions.
    pub fn colors(mut self, colors: Vec<Vec<Vec<Color>>>) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Set the camera configuration.
    pub fn camera(mut self, cam: Camera3D) -> Self {
        self.camera = cam;
        self
    }

    /// Set the plot title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Set the default color for occupied voxels without a per-voxel color.
    pub fn default_color(mut self, c: Color) -> Self {
        self.default_color = Some(c);
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

        let nz = self.occupied.len();
        if nz == 0 {
            return;
        }
        let ny = self.occupied[0].len();
        if ny == 0 {
            return;
        }
        let nx = self.occupied[0][0].len();
        if nx == 0 {
            return;
        }

        let base_color = self.default_color.unwrap_or(self.theme.primary);

        // Collect occupied voxels with their center position, depth, and color
        struct VoxelInfo {
            ix: usize,
            iy: usize,
            iz: usize,
            depth: f64,
            color: Color,
        }

        let grid_max = (nx.max(ny).max(nz)) as f64;
        let scale = if grid_max > 0.0 { 2.0 / grid_max } else { 1.0 };

        let mut voxels: Vec<VoxelInfo> = Vec::new();

        for iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    if !self.occupied[iz][iy][ix] {
                        continue;
                    }
                    // Normalize center to [-1, 1]
                    let cx = (ix as f64 + 0.5) * scale - 1.0;
                    let cy = (iy as f64 + 0.5) * scale - 1.0;
                    let cz = ((iz as f64 + 0.5) * scale - 1.0) * 0.8;

                    let (_, _, depth) = camera.project(cx, cy, cz);

                    let color = self
                        .colors
                        .as_ref()
                        .and_then(|c| c.get(iz))
                        .and_then(|c| c.get(iy))
                        .and_then(|c| c.get(ix))
                        .copied()
                        .unwrap_or(base_color);

                    voxels.push(VoxelInfo {
                        ix,
                        iy,
                        iz,
                        depth,
                        color,
                    });
                }
            }
        }

        if voxels.is_empty() {
            return;
        }

        // Sort back-to-front (painter's algorithm)
        voxels.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Compute screen bounds from all cube corners
        let half = scale * 0.5;
        let mut sx_min = f64::INFINITY;
        let mut sx_max = f64::NEG_INFINITY;
        let mut sy_min = f64::INFINITY;
        let mut sy_max = f64::NEG_INFINITY;

        for v in &voxels {
            let cx = (v.ix as f64 + 0.5) * scale - 1.0;
            let cy = (v.iy as f64 + 0.5) * scale - 1.0;
            let cz = ((v.iz as f64 + 0.5) * scale - 1.0) * 0.8;
            // Project all 8 corners
            for &dz in &[-half * 0.8, half * 0.8] {
                for &dy in &[-half, half] {
                    for &dx in &[-half, half] {
                        let (sx, sy, _) = camera.project(cx + dx, cy + dy, cz + dz);
                        sx_min = sx_min.min(sx);
                        sx_max = sx_max.max(sx);
                        sy_min = sy_min.min(sy);
                        sy_max = sy_max.max(sy);
                    }
                }
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

        // Render each voxel as a filled quad (its front-facing faces)
        for v in &voxels {
            let cx = (v.ix as f64 + 0.5) * scale - 1.0;
            let cy = (v.iy as f64 + 0.5) * scale - 1.0;
            let cz = ((v.iz as f64 + 0.5) * scale - 1.0) * 0.8;
            let hz = half * 0.8;

            // Define the 8 corners of the cube
            let corners: [(f64, f64, f64); 8] = [
                (cx - half, cy - half, cz - hz), // 0: left-front-bottom
                (cx + half, cy - half, cz - hz), // 1: right-front-bottom
                (cx + half, cy + half, cz - hz), // 2: right-back-bottom
                (cx - half, cy + half, cz - hz), // 3: left-back-bottom
                (cx - half, cy - half, cz + hz), // 4: left-front-top
                (cx + half, cy - half, cz + hz), // 5: right-front-top
                (cx + half, cy + half, cz + hz), // 6: right-back-top
                (cx - half, cy + half, cz + hz), // 7: left-back-top
            ];

            // Project all corners
            let proj: Vec<(f64, f64, f64)> = corners
                .iter()
                .map(|&(x, y, z)| camera.project(x, y, z))
                .collect();

            // Define 6 faces as quad indices + shading factor
            let faces: [(usize, usize, usize, usize, f64); 6] = [
                (4, 5, 6, 7, 1.0),  // top
                (0, 1, 5, 4, 0.85), // front
                (1, 2, 6, 5, 0.7),  // right
                (3, 2, 1, 0, 0.6),  // bottom
                (3, 0, 4, 7, 0.75), // left
                (2, 3, 7, 6, 0.65), // back
            ];

            // Check which faces are front-facing and draw them
            for &(a, b, c, d, shade) in &faces {
                // Screen-space vertices
                let sa = (proj[a].0, proj[a].1);
                let sb_v = (proj[b].0, proj[b].1);
                let sc = (proj[c].0, proj[c].1);

                // Cross product to determine facing direction
                let e1x = sb_v.0 - sa.0;
                let e1y = sb_v.1 - sa.1;
                let e2x = sc.0 - sa.0;
                let e2y = sc.1 - sa.1;
                let cross = e1x * e2y - e1y * e2x;

                // Only draw front-facing faces (positive cross product)
                if cross <= 0.0 {
                    continue;
                }

                let quad = [
                    (sa.0, sa.1),
                    (sb_v.0, sb_v.1),
                    (sc.0, sc.1),
                    (proj[d].0, proj[d].1),
                ];

                // Map to pixel coordinates
                let screen_quad: Vec<(i32, i32)> = quad
                    .iter()
                    .map(|&(qx, qy)| {
                        let scx =
                            data_to_screen(qx, sx_min, sx_max, px as f64, (px + pw - 1) as f64)
                                .round() as i32;
                        let scy =
                            data_to_screen(qy, sy_min, sy_max, py as f64, (py + ph - 1) as f64)
                                .round() as i32;
                        (scx, scy)
                    })
                    .collect();

                let face_color = shade_color(v.color, shade);

                // Bounding box
                let Some(bb_min_x) = screen_quad.iter().map(|c| c.0).min() else {
                    continue;
                };
                let Some(bb_max_x) = screen_quad.iter().map(|c| c.0).max() else {
                    continue;
                };
                let Some(bb_min_y) = screen_quad.iter().map(|c| c.1).min() else {
                    continue;
                };
                let Some(bb_max_y) = screen_quad.iter().map(|c| c.1).max() else {
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
                            && point_in_quad(sx, upper_row, &screen_quad);

                        let lower_in = lower_row <= bb_max_y
                            && ux >= px
                            && ux < px + pw
                            && (lower_row as u16) >= py
                            && (lower_row as u16) < py + ph
                            && point_in_quad(sx, lower_row, &screen_quad);

                        if upper_in && lower_in {
                            let uf = vert_frac(upper_row, bb_min_y, bb_max_y);
                            let lf = vert_frac(lower_row, bb_min_y, bb_max_y);
                            let uc = shade_color(face_color, 1.0 - uf * 0.15);
                            let lc = shade_color(face_color, 1.0 - lf * 0.15);
                            buf[(ux, uy)]
                                .set_char(self.theme.chars.fill.half_upper)
                                .set_style(Style::default().fg(uc).bg(lc));
                        } else if upper_in {
                            let uf = vert_frac(upper_row, bb_min_y, bb_max_y);
                            let uc = shade_color(face_color, 1.0 - uf * 0.15);
                            buf[(ux, uy)]
                                .set_char(self.theme.chars.fill.half_upper)
                                .set_style(Style::default().fg(uc));
                        } else if lower_in {
                            let lf = vert_frac(lower_row, bb_min_y, bb_max_y);
                            let lc = shade_color(face_color, 1.0 - lf * 0.15);
                            buf[(ux, uy)]
                                .set_char(self.theme.chars.fill.half_lower)
                                .set_style(Style::default().fg(lc));
                        }
                    }
                    row += 2;
                }
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
}


impl Widget for &Voxels {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_with_camera(&self.camera, area, buf);
    }
}

impl StatefulWidget for &Voxels {
    type State = Camera3DState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let camera = state.to_camera();
        self.render_with_camera(&camera, area, buf);
    }
}

// ── helper utilities ────────────────────────────────────────────────────────

/// Test if a point is inside a convex quad using cross-product winding.
fn point_in_quad(px: i32, py: i32, quad: &[(i32, i32)]) -> bool {
    let n = quad.len();
    let mut sign = 0i32;
    for i in 0..n {
        let (x0, y0) = quad[i];
        let (x1, y1) = quad[(i + 1) % n];
        let cross = (x1 - x0) as i64 * (py - y0) as i64 - (y1 - y0) as i64 * (px - x0) as i64;
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

    // X axis
    let ax = to_sx(x_tip.0);
    let ay = to_sy(x_tip.1);
    draw_braille_line(buf, ox, oy, ax, ay, theme.x_axis_3d_color, pa);
    let axi = ax.round() as u16;
    let ayi = ay.round() as u16;
    if axi >= px && axi < px + pw && ayi >= py && ayi < py + ph {
        buf[(axi, ayi)].set_char('X').set_fg(theme.x_axis_3d_color);
    }

    // Y axis
    let bx = to_sx(y_tip.0);
    let by = to_sy(y_tip.1);
    draw_braille_line(buf, ox, oy, bx, by, theme.y_axis_3d_color, pa);
    let bxi = bx.round() as u16;
    let byi = by.round() as u16;
    if bxi >= px && bxi < px + pw && byi >= py && byi < py + ph {
        buf[(bxi, byi)].set_char('Y').set_fg(theme.y_axis_3d_color);
    }

    // Z axis
    let cx = to_sx(z_tip.0);
    let cy = to_sy(z_tip.1);
    draw_braille_line(buf, ox, oy, cx, cy, theme.z_axis_3d_color, pa);
    let cxi = cx.round() as u16;
    let cyi = cy.round() as u16;
    if cxi >= px && cxi < px + pw && cyi >= py && cyi < py + ph {
        buf[(cxi, cyi)].set_char('Z').set_fg(theme.z_axis_3d_color);
    }
}
