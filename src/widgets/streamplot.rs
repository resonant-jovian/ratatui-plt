//! Streamline visualization for 2D vector fields, inspired by matplotlib's `streamplot`.
//!
//! Traces streamlines through a vector field using 4th-order Runge-Kutta
//! integration and renders them as connected characters with directional arrows.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::streamplot::StreamPlot;
//!
//! let field = VectorFieldData::from_fn(
//!     (-2.0, 2.0), (-2.0, 2.0), 20, 20,
//!     |x, y| (-y, x), // Circular flow
//! );
//! let plot = StreamPlot::new(field)
//!     .title("Stream Lines")
//!     .density(2)
//!     .color_by_magnitude(true);
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::colormap::{Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::series::VectorFieldData;
use crate::theme::Theme;
use crate::transform::data_to_screen;

/// A streamline plot widget for visualising vector fields.
///
/// Computes streamlines via 4th-order Runge-Kutta integration from seed
/// points placed on a regular grid. Streamlines are rendered as connected
/// characters with arrow heads showing flow direction.
pub struct StreamPlot {
    /// The vector field data (positions and vectors).
    field: VectorFieldData,
    /// X-axis configuration.
    x_axis: Axis,
    /// Y-axis configuration.
    y_axis: Axis,
    /// Chart title.
    title: Option<String>,
    /// Seed grid density multiplier (1 = default spacing).
    density: usize,
    /// Base colour for streamlines (when not colouring by magnitude).
    color: Color,
    /// Whether to colour streamlines by local velocity magnitude.
    color_by_magnitude: bool,
    /// Colormap used when `color_by_magnitude` is true.
    colormap: Box<dyn Colormap>,
    /// Arrow rendering scale (controls visual weight of arrow heads).
    arrow_scale: f64,
    /// Visual theme.
    theme: Theme,
}

impl StreamPlot {
    /// Create a new stream plot from vector field data.
    pub fn new(field: VectorFieldData) -> Self {
        Self {
            field,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            density: 1,
            color: Color::Cyan,
            color_by_magnitude: false,
            colormap: Box::new(Viridis),
            arrow_scale: 1.0,
            theme: Theme::get_default(),
        }
    }

    /// Set the X-axis configuration.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y-axis configuration.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the chart title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Set the seed point density (higher = more streamlines).
    pub fn density(mut self, d: usize) -> Self {
        self.density = d.max(1);
        self
    }

    /// Set the base streamline colour.
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }

    /// Enable or disable colouring by local velocity magnitude.
    pub fn color_by_magnitude(mut self, enable: bool) -> Self {
        self.color_by_magnitude = enable;
        self
    }

    /// Set the colormap for magnitude-based colouring.
    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    /// Set the arrow head visual scale factor.
    pub fn arrow_scale(mut self, scale: f64) -> Self {
        self.arrow_scale = scale;
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

/// Interpolate the vector field at an arbitrary (x, y) position using
/// inverse-distance weighted interpolation of nearby grid vectors.
fn interpolate_field(field: &VectorFieldData, x: f64, y: f64) -> (f64, f64) {
    if field.vectors.is_empty() {
        return (0.0, 0.0);
    }

    let mut weight_sum = 0.0f64;
    let mut dx_sum = 0.0f64;
    let mut dy_sum = 0.0f64;

    // Find the closest vectors and do inverse-distance weighting.
    // For performance, we limit to vectors within a reasonable neighbourhood.
    // First pass: find a rough distance scale from the grid spacing.
    let mut min_dist_sq = f64::INFINITY;
    let mut closest_idx = 0;
    for (i, &(vx, vy, _, _)) in field.vectors.iter().enumerate() {
        let dsq = (vx - x) * (vx - x) + (vy - y) * (vy - y);
        if dsq < min_dist_sq {
            min_dist_sq = dsq;
            closest_idx = i;
        }
    }

    // If we are very close to a grid point, just return its value
    if min_dist_sq < 1e-12 {
        let (_, _, dx, dy) = field.vectors[closest_idx];
        return (dx, dy);
    }

    // Use inverse-distance weighting with the 4 nearest neighbours
    let mut dists: Vec<(f64, usize)> = field
        .vectors
        .iter()
        .enumerate()
        .map(|(i, &(vx, vy, _, _))| {
            let dsq = (vx - x) * (vx - x) + (vy - y) * (vy - y);
            (dsq, i)
        })
        .collect();
    dists.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let n_neighbours = dists.len().min(4);
    for &(dsq, idx) in &dists[..n_neighbours] {
        let w = 1.0 / (dsq + 1e-10);
        let (_, _, fdx, fdy) = field.vectors[idx];
        dx_sum += w * fdx;
        dy_sum += w * fdy;
        weight_sum += w;
    }

    if weight_sum > 0.0 {
        (dx_sum / weight_sum, dy_sum / weight_sum)
    } else {
        (0.0, 0.0)
    }
}

/// Integrate a single streamline using 4th-order Runge-Kutta.
/// Returns a list of (x, y) points along the streamline.
#[allow(clippy::too_many_arguments)]
fn trace_streamline(
    field: &VectorFieldData,
    x0: f64,
    y0: f64,
    x_lo: f64,
    x_hi: f64,
    y_lo: f64,
    y_hi: f64,
    max_steps: usize,
    dt: f64,
) -> Vec<(f64, f64)> {
    let mut points = Vec::with_capacity(max_steps);
    let mut x = x0;
    let mut y = y0;

    points.push((x, y));

    for _ in 0..max_steps {
        // RK4 integration
        let (k1x, k1y) = interpolate_field(field, x, y);
        let (k2x, k2y) = interpolate_field(field, x + 0.5 * dt * k1x, y + 0.5 * dt * k1y);
        let (k3x, k3y) = interpolate_field(field, x + 0.5 * dt * k2x, y + 0.5 * dt * k2y);
        let (k4x, k4y) = interpolate_field(field, x + dt * k3x, y + dt * k3y);

        let dx = dt * (k1x + 2.0 * k2x + 2.0 * k3x + k4x) / 6.0;
        let dy = dt * (k1y + 2.0 * k2y + 2.0 * k3y + k4y) / 6.0;

        // Stop if velocity is essentially zero
        if dx * dx + dy * dy < 1e-20 {
            break;
        }

        x += dx;
        y += dy;

        // Stop if out of bounds
        if x < x_lo || x > x_hi || y < y_lo || y > y_hi {
            break;
        }

        points.push((x, y));
    }

    points
}

struct ClipRect {
    x_min: u16,
    y_min: u16,
    x_max: u16,
    y_max: u16,
}

const BRAILLE_BITS: [[u8; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40],
    [0x08, 0x10, 0x20, 0x80],
];
const BRAILLE_BASE: u32 = 0x2800;

fn write_braille(buf: &mut Buffer, x: u16, y: u16, bits: u8, color: Color) {
    let existing = {
        let ch = buf[(x, y)].symbol().chars().next().unwrap_or(' ');
        let code = ch as u32;
        if (BRAILLE_BASE..=0x28FF).contains(&code) {
            (code - BRAILLE_BASE) as u8
        } else {
            0
        }
    };
    let combined = existing | bits;
    if let Some(ch) = char::from_u32(BRAILLE_BASE + combined as u32) {
        buf[(x, y)].set_char(ch).set_fg(color);
    }
}

fn draw_braille_line(
    buf: &mut Buffer,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
    clip: &ClipRect,
) {
    let mut ix0 = (x0 * 2.0).round() as i32;
    let mut iy0 = (y0 * 4.0).round() as i32;
    let ix1 = (x1 * 2.0).round() as i32;
    let iy1 = (y1 * 4.0).round() as i32;

    let dx = (ix1 - ix0).abs();
    let dy = -(iy1 - iy0).abs();
    let sx = if ix0 < ix1 { 1 } else { -1 };
    let sy = if iy0 < iy1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if ix0 >= 0 && iy0 >= 0 {
            let cell_x = (ix0 / 2) as u16;
            let cell_y = (iy0 / 4) as u16;
            if cell_x >= clip.x_min
                && cell_x < clip.x_max
                && cell_y >= clip.y_min
                && cell_y < clip.y_max
            {
                let dot_col = (ix0 % 2) as usize;
                let dot_row = (iy0 % 4) as usize;
                let bit = BRAILLE_BITS[dot_col][dot_row];
                write_braille(buf, cell_x, cell_y, bit, color);
            }
        }

        if ix0 == ix1 && iy0 == iy1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            ix0 += sx;
        }
        if e2 <= dx {
            err += dx;
            iy0 += sy;
        }
    }
}

/// Choose an arrow character for the direction.
fn arrow_char(dx: f64, dy: f64) -> char {
    if dx.abs() < 1e-10 && dy.abs() < 1e-10 {
        return '·';
    }
    let angle = dy.atan2(dx);
    let octant = ((angle + std::f64::consts::PI) / (std::f64::consts::PI / 4.0)).round() as i32 % 8;
    match octant {
        0 => '←',
        1 => '↙',
        2 => '↓',
        3 => '↘',
        4 => '→',
        5 => '↗',
        6 => '↑',
        7 => '↖',
        _ => '→',
    }
}

impl Widget for &StreamPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.field.vectors.is_empty() {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let y_label_width: u16 = 8;
        let tick_height: u16 = 1;

        let px = area.x + y_label_width;
        let py = area.y + title_height;
        let pw = area.width.saturating_sub(y_label_width + 1);
        let ph = area.height.saturating_sub(title_height + tick_height);

        if pw < 2 || ph < 2 {
            return;
        }

        // Draw title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        // Compute data bounds
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for &(x, y, _, _) in &self.field.vectors {
            x_min = x_min.min(x);
            x_max = x_max.max(x);
            y_min = y_min.min(y);
            y_max = y_max.max(y);
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Compute magnitude range for colour mapping
        let max_mag = self.field.max_magnitude();
        let norm = LinearNorm::new(0.0, if max_mag == 0.0 { 1.0 } else { max_mag });

        // Draw axes
        for x in px..px + pw {
            if x < area.x + area.width {
                buf[(x, py + ph)]
                    .set_char('─')
                    .set_fg(self.theme.axis_color);
            }
        }
        for y in py..py + ph {
            buf[(px.saturating_sub(1), y)]
                .set_char('│')
                .set_fg(self.theme.axis_color);
        }

        // Draw grid
        let x_grid = self.x_axis.grid || self.theme.grid_visible;
        let y_grid = self.y_axis.grid || self.theme.grid_visible;
        if x_grid {
            let gx_ticks = self.x_axis.tick_positions(x_lo, x_hi);
            for &tv in &gx_ticks {
                let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
                let xi = sx.round() as u16;
                if xi >= px && xi < px + pw {
                    for y in py..py + ph {
                        buf[(xi, y)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }
        if y_grid {
            let gy_ticks = self.y_axis.tick_positions(y_lo, y_hi);
            for &tv in &gy_ticks {
                let sy = data_to_screen(tv, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
                let yi = sy.round() as u16;
                if yi >= py && yi < py + ph {
                    for x in px..px + pw {
                        buf[(x, yi)].set_char('·').set_fg(self.theme.grid_color);
                    }
                }
            }
        }

        // Generate seed points on a grid
        let x_range = x_hi - x_lo;
        let y_range = y_hi - y_lo;
        let n_seeds_x = (3 * self.density).max(2);
        let n_seeds_y = (3 * self.density).max(2);

        // Step size for integration: proportional to the data scale
        let diag = (x_range * x_range + y_range * y_range).sqrt();
        let dt = diag / (50.0 * self.density as f64).max(1.0);
        let max_steps = 200 * self.density;

        // Arrow head interval (every N points along the streamline)
        let arrow_interval = (15.0 / self.arrow_scale).max(3.0) as usize;

        // Trace streamlines from each seed
        for si in 0..n_seeds_y {
            for sj in 0..n_seeds_x {
                let sx = x_lo + x_range * (sj as f64 + 0.5) / n_seeds_x as f64;
                let sy = y_lo + y_range * (si as f64 + 0.5) / n_seeds_y as f64;

                // Trace forward
                let forward =
                    trace_streamline(&self.field, sx, sy, x_lo, x_hi, y_lo, y_hi, max_steps, dt);

                // Trace backward
                let backward =
                    trace_streamline(&self.field, sx, sy, x_lo, x_hi, y_lo, y_hi, max_steps, -dt);

                // Combine: reverse of backward (excluding seed) + forward
                let mut points: Vec<(f64, f64)> = Vec::new();
                for i in (1..backward.len()).rev() {
                    points.push(backward[i]);
                }
                points.extend_from_slice(&forward);

                if points.len() < 2 {
                    continue;
                }

                // Render the streamline using braille sub-pixel lines
                let clip = ClipRect { x_min: px, y_min: py, x_max: px + pw, y_max: py + ph };
                let mut prev_screen: Option<(f64, f64)> = None;
                for (idx, &(ptx, pty)) in points.iter().enumerate() {
                    let scr_x = data_to_screen(ptx, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
                    let scr_y = data_to_screen(pty, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
                    let xi = scr_x.round() as u16;
                    let yi = scr_y.round() as u16;

                    if xi < px || xi >= px + pw || yi < py || yi >= py + ph {
                        prev_screen = None;
                        continue;
                    }

                    // Determine colour
                    let color = if self.color_by_magnitude {
                        let (fdx, fdy) = interpolate_field(&self.field, ptx, pty);
                        let mag = (fdx * fdx + fdy * fdy).sqrt();
                        let t = norm.normalize(mag);
                        self.colormap.color_at(t)
                    } else {
                        self.color
                    };

                    // Draw braille line from previous point
                    if let Some((prev_x, prev_y)) = prev_screen {
                        draw_braille_line(buf, prev_x, prev_y, scr_x, scr_y, color, &clip);
                    }

                    // Arrow head at intervals (cell resolution, drawn on top)
                    if idx % arrow_interval == arrow_interval / 2 && idx > 0 {
                        let (fdx, fdy) = interpolate_field(&self.field, ptx, pty);
                        let ch = arrow_char(fdx, -fdy);
                        buf[(xi, yi)].set_char(ch).set_fg(color);
                    }

                    prev_screen = Some((scr_x, scr_y));
                }
            }
        }

        // Tick labels on X axis
        let x_ticks = self.x_axis.tick_positions(x_lo, x_hi);
        for &tv in &x_ticks {
            let sx = data_to_screen(tv, x_lo, x_hi, px as f64, (px + pw - 1) as f64);
            let label = self.x_axis.format_tick(tv);
            let xi = sx.round() as u16;
            let start = xi.saturating_sub(label.len() as u16 / 2);
            let y = py + ph;
            if y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, y)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }

        // Tick labels on Y axis
        let y_ticks = self.y_axis.tick_positions(y_lo, y_hi);
        for &tv in &y_ticks {
            let sy = data_to_screen(tv, y_lo, y_hi, (py + ph - 1) as f64, py as f64);
            let label = self.y_axis.format_tick(tv);
            let yi = sy.round() as u16;
            if yi >= py && yi < py + ph {
                let label_start = if label.len() < y_label_width as usize {
                    px.saturating_sub(y_label_width) + (y_label_width - label.len() as u16)
                } else {
                    px.saturating_sub(y_label_width)
                };
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < px.saturating_sub(1) {
                        buf[(lx, yi)].set_char(ch).set_fg(self.theme.axis_color);
                    }
                }
            }
        }
    }
}
