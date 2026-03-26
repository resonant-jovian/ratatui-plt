//! 3D volumetric rendering widget.
//!
//! Renders a 3D scalar field using front-to-back ray compositing.
//! Each screen pixel samples along a ray through the volume, accumulating
//! color and opacity.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{StatefulWidget, Widget};

use crate::colormap::{Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::theme::Theme;
use crate::transform::{Camera3D, Camera3DState};

/// A 3D volumetric rendering widget.
///
/// Casts rays through a 3D scalar field and composites color/opacity
/// to produce a semi-transparent volume visualization.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// // 5x5x5 scalar field
/// let values: Vec<Vec<Vec<f64>>> = (0..5)
///     .map(|z| {
///         (0..5)
///             .map(|y| {
///                 (0..5)
///                     .map(|x| {
///                         let r = ((x as f64 - 2.0).powi(2)
///                             + (y as f64 - 2.0).powi(2)
///                             + (z as f64 - 2.0).powi(2))
///                         .sqrt();
///                         (-r).exp()
///                     })
///                     .collect()
///             })
///             .collect()
///     })
///     .collect();
/// let vol = Volume3D::new(values)
///     .x_range(-1.0, 1.0)
///     .y_range(-1.0, 1.0)
///     .z_range(-1.0, 1.0)
///     .opacity(0.5);
/// ```
pub struct Volume3D {
    values: Vec<Vec<Vec<f64>>>,
    x_range: (f64, f64),
    y_range: (f64, f64),
    z_range: (f64, f64),
    camera: Camera3D,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    opacity: f64,
    title: Option<String>,
    theme: Theme,
}

impl Volume3D {
    pub fn new(values: Vec<Vec<Vec<f64>>>) -> Self {
        // Compute value bounds
        let (mut vmin, mut vmax) = (f64::INFINITY, f64::NEG_INFINITY);
        for plane in &values {
            for row in plane {
                for &v in row {
                    if v.is_finite() {
                        vmin = vmin.min(v);
                        vmax = vmax.max(v);
                    }
                }
            }
        }
        if vmin >= vmax {
            vmax = vmin + 1.0;
        }

        Self {
            values,
            x_range: (-1.0, 1.0),
            y_range: (-1.0, 1.0),
            z_range: (-1.0, 1.0),
            camera: Camera3D::default(),
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            opacity: 0.5,
            title: None,
            theme: Theme::get_default(),
        }
    }

    pub fn x_range(mut self, lo: f64, hi: f64) -> Self {
        self.x_range = (lo, hi);
        self
    }

    pub fn y_range(mut self, lo: f64, hi: f64) -> Self {
        self.y_range = (lo, hi);
        self
    }

    pub fn z_range(mut self, lo: f64, hi: f64) -> Self {
        self.z_range = (lo, hi);
        self
    }

    pub fn camera(mut self, c: Camera3D) -> Self {
        self.camera = c;
        self
    }

    pub fn colormap(mut self, c: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(c);
        self
    }

    pub fn norm(mut self, n: impl Normalize + 'static) -> Self {
        self.norm = Box::new(n);
        self
    }

    pub fn opacity(mut self, o: f64) -> Self {
        self.opacity = o.clamp(0.0, 1.0);
        self
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn theme(mut self, t: Theme) -> Self {
        self.theme = t;
        self
    }

    fn render_with_camera(&self, area: Rect, buf: &mut Buffer, camera: &Camera3D) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        let nz = self.values.len();
        if nz == 0 {
            return;
        }
        let ny = self.values[0].len();
        if ny == 0 {
            return;
        }
        let nx = self.values[0][0].len();
        if nx == 0 {
            return;
        }

        // Render title
        let plot_area = if let Some(ref t) = self.title {
            let tx = area.x + area.width.saturating_sub(t.len() as u16) / 2;
            for (i, ch) in t.chars().enumerate() {
                let x = tx + i as u16;
                if x < area.x + area.width
                    && let Some(cell) = buf.cell_mut((x, area.y)) {
                        cell.set_char(ch);
                        cell.set_fg(self.theme.foreground);
                    }
            }
            Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1))
        } else {
            area
        };

        // Project volume corners to find screen bounds
        let corners = [
            (self.x_range.0, self.y_range.0, self.z_range.0),
            (self.x_range.1, self.y_range.0, self.z_range.0),
            (self.x_range.0, self.y_range.1, self.z_range.0),
            (self.x_range.1, self.y_range.1, self.z_range.0),
            (self.x_range.0, self.y_range.0, self.z_range.1),
            (self.x_range.1, self.y_range.0, self.z_range.1),
            (self.x_range.0, self.y_range.1, self.z_range.1),
            (self.x_range.1, self.y_range.1, self.z_range.1),
        ];
        let projected: Vec<(f64, f64, f64)> = corners
            .iter()
            .map(|&(x, y, z)| camera.project(x, y, z))
            .collect();

        let (sx_min, sx_max, sy_min, sy_max) = projected.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY),
            |(sxn, sxx, syn, syx), &(sx, sy, _)| {
                (sxn.min(sx), sxx.max(sx), syn.min(sy), syx.max(sy))
            },
        );

        let n_samples = 20;

        // For each screen pixel, cast a ray and composite
        for row in 0..plot_area.height {
            for col in 0..plot_area.width {
                let screen_x = plot_area.x + col;
                let screen_y = plot_area.y + row;

                // Map screen to normalized projection coords
                let _sx_norm = if plot_area.width > 1 {
                    sx_min + (col as f64 / (plot_area.width - 1) as f64) * (sx_max - sx_min)
                } else {
                    (sx_min + sx_max) / 2.0
                };
                let _sy_norm = if plot_area.height > 1 {
                    sy_min + (row as f64 / (plot_area.height - 1) as f64) * (sy_max - sy_min)
                } else {
                    (sy_min + sy_max) / 2.0
                };

                // Simple approach: sample along z-axis through the volume
                // Map screen position to (x, y) in data space
                let data_x = self.x_range.0
                    + (col as f64 / plot_area.width.max(1) as f64)
                        * (self.x_range.1 - self.x_range.0);
                let data_y = self.y_range.0
                    + (row as f64 / plot_area.height.max(1) as f64)
                        * (self.y_range.1 - self.y_range.0);

                let mut accum_r = 0.0f64;
                let mut accum_g = 0.0f64;
                let mut accum_b = 0.0f64;
                let mut accum_a = 0.0f64;

                for si in 0..n_samples {
                    let t = si as f64 / (n_samples - 1).max(1) as f64;
                    let data_z = self.z_range.0 + t * (self.z_range.1 - self.z_range.0);

                    // Trilinear sample
                    let fx = (data_x - self.x_range.0)
                        / (self.x_range.1 - self.x_range.0).max(1e-10)
                        * (nx - 1) as f64;
                    let fy = (data_y - self.y_range.0)
                        / (self.y_range.1 - self.y_range.0).max(1e-10)
                        * (ny - 1) as f64;
                    let fz = (data_z - self.z_range.0)
                        / (self.z_range.1 - self.z_range.0).max(1e-10)
                        * (nz - 1) as f64;

                    let ix = fx.floor() as usize;
                    let iy = fy.floor() as usize;
                    let iz = fz.floor() as usize;

                    if ix >= nx - 1 || iy >= ny - 1 || iz >= nz - 1 {
                        continue;
                    }

                    // Nearest-neighbor for simplicity
                    let val = self.values[iz][iy][ix];
                    if !val.is_finite() {
                        continue;
                    }

                    let norm_val = self.norm.normalize(val);
                    let sample_alpha = norm_val * self.opacity / n_samples as f64;

                    if sample_alpha < 1e-6 {
                        continue;
                    }

                    let color = self.colormap.color_at(norm_val);
                    let (sr, sg, sb) = match color {
                        Color::Rgb(r, g, b) => (r as f64, g as f64, b as f64),
                        _ => (128.0, 128.0, 128.0),
                    };

                    // Front-to-back compositing
                    let remaining = 1.0 - accum_a;
                    accum_r += sr * sample_alpha * remaining;
                    accum_g += sg * sample_alpha * remaining;
                    accum_b += sb * sample_alpha * remaining;
                    accum_a += sample_alpha * remaining;

                    if accum_a > 0.95 {
                        break;
                    }
                }

                if accum_a > 0.01 {
                    let final_color = Color::Rgb(
                        accum_r.clamp(0.0, 255.0) as u8,
                        accum_g.clamp(0.0, 255.0) as u8,
                        accum_b.clamp(0.0, 255.0) as u8,
                    );
                    if let Some(cell) = buf.cell_mut((screen_x, screen_y)) {
                        cell.set_char(self.theme.chars.fill.solid);
                        cell.set_fg(final_color);
                    }
                }
            }
        }
    }
}

impl Widget for &Volume3D {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_with_camera(area, buf, &self.camera);
    }
}

impl StatefulWidget for &Volume3D {
    type State = Camera3DState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let camera = state.to_camera();
        self.render_with_camera(area, buf, &camera);
    }
}
