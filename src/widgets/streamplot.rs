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

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::colormap::{Colormap, Viridis};
use crate::drawing::BRAILLE_BITS;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::norm::{LinearNorm, Normalize};
use crate::plot_buffer::{PlotBackend, Z_DATA, Z_MARKER, create_backend};
use crate::series::VectorFieldData;
use crate::spines::Spines;
use crate::theme::Theme;

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
    color: Option<Color>,
    /// Whether to colour streamlines by local velocity magnitude.
    color_by_magnitude: bool,
    /// Colormap used when `color_by_magnitude` is true.
    colormap: Box<dyn Colormap>,
    /// Arrow rendering scale (controls visual weight of arrow heads).
    arrow_scale: f64,
    /// Visual theme.
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
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
            color: None,
            color_by_magnitude: false,
            colormap: Box::new(Viridis),
            arrow_scale: 1.0,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
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
        self.color = Some(c);
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

    /// Set spine visibility.
    pub fn spines(mut self, spines: Spines) -> Self {
        self.spines = spines;
        self
    }

    /// Add a reference line.
    pub fn reference_line(mut self, line: ReferenceLine) -> Self {
        self.reference_lines.push(line);
        self
    }

    /// Set all reference lines.
    pub fn reference_lines(mut self, lines: Vec<ReferenceLine>) -> Self {
        self.reference_lines = lines;
        self
    }

    /// Add an annotation.
    pub fn annotation(mut self, ann: Annotation) -> Self {
        self.annotations.push(ann);
        self
    }
}

/// Interpolate the vector field at an arbitrary (x, y) position.
/// Delegates to `VectorFieldData::interpolate` which uses O(1) bilinear
/// lookup for grid-based fields.
fn interpolate_field(field: &VectorFieldData, x: f64, y: f64) -> (f64, f64) {
    field.interpolate(x, y)
}

/// Bounding box for streamline integration.
struct StreamBounds {
    x_lo: f64,
    x_hi: f64,
    y_lo: f64,
    y_hi: f64,
}

/// Integrate a single streamline using 4th-order Runge-Kutta.
/// Returns a list of (x, y) points along the streamline.
fn trace_streamline(
    field: &VectorFieldData,
    x0: f64,
    y0: f64,
    bounds: &StreamBounds,
    max_steps: usize,
    dt: f64,
) -> Vec<(f64, f64)> {
    let StreamBounds {
        x_lo,
        x_hi,
        y_lo,
        y_hi,
    } = *bounds;
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

/// Draw a braille line into a PlotBuffer, clipped to the plot area.
#[allow(clippy::too_many_arguments)]
fn draw_braille_line_stream(
    pb: &mut dyn PlotBackend,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
    pa_x: u16,
    pa_y: u16,
    pa_w: u16,
    pa_h: u16,
    z: u8,
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
            if cell_x >= pa_x && cell_x < pa_x + pa_w && cell_y >= pa_y && cell_y < pa_y + pa_h {
                let dot_col = (ix0 % 2) as usize;
                let dot_row = (iy0 % 4) as usize;
                let bit = BRAILLE_BITS[dot_col][dot_row];
                pb.set_braille(cell_x, cell_y, bit, color, z);
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
fn arrow_char(dx: f64, dy: f64, arrows: &crate::chars::ArrowChars) -> char {
    if dx.abs() < 1e-10 && dy.abs() < 1e-10 {
        return '·';
    }
    let angle = dy.atan2(dx);
    let octant = ((angle + std::f64::consts::PI) / (std::f64::consts::PI / 4.0)).round() as i32 % 8;
    match octant {
        0 => arrows.left,
        1 => arrows.sw,
        2 => arrows.down,
        3 => arrows.se,
        4 => arrows.right,
        5 => arrows.ne,
        6 => arrows.up,
        7 => arrows.nw,
        _ => arrows.right,
    }
}

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for StreamPlot {
    fn render_plotters(
        &self,
        area: ratatui::layout::Rect,
        buf: &mut ratatui::buffer::Buffer,
        theme: &crate::theme::Theme,
    ) {
        use crate::plotters_render::{bridge, theme_bridge};

        bridge::render_plotters_to_buf(
            area,
            buf,
            theme_bridge::theme_bg_rgb(theme),
            |_root| {
                // Minimal implementation - full plotters rendering TBD.
            },
        );
    }
}

impl Widget for &StreamPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        #[cfg(feature = "plotters-render")]
        {
            if crate::plotters_render::should_use_plotters() {
                use crate::plotters_render::PlottersRenderable;
                self.render_plotters(area, buf, &self.theme);
                return;
            }
        }
        if area.width < 4 || area.height < 4 || self.field.vectors.is_empty() {
            return;
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

        let mut pb = create_backend(area);

        // Create and render the plot frame (title, axes, grid, ticks, labels, spines, ref lines)
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render_to_pb(
            &mut pb,
            area,
            DataBounds {
                x_lo,
                x_hi,
                y_lo,
                y_hi,
            },
        ) else {
            return;
        };

        let px = pa.x;
        let py = pa.y;
        let pw = pa.width;
        let ph = pa.height;

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
                let sb = StreamBounds {
                    x_lo,
                    x_hi,
                    y_lo,
                    y_hi,
                };
                let forward = trace_streamline(&self.field, sx, sy, &sb, max_steps, dt);

                // Trace backward
                let backward = trace_streamline(&self.field, sx, sy, &sb, max_steps, -dt);

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
                let mut prev_screen: Option<(f64, f64)> = None;
                for (idx, &(ptx, pty)) in points.iter().enumerate() {
                    let scr_x = pa.screen_x(ptx);
                    let scr_y = pa.screen_y(pty);
                    let xi = scr_x.round() as u16;
                    let yi = scr_y.round() as u16;

                    if !pa.contains(xi, yi) {
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
                        self.color.unwrap_or(self.theme.primary)
                    };

                    // Draw braille line from previous point
                    if let Some((prev_x, prev_y)) = prev_screen {
                        draw_braille_line_stream(
                            &mut pb, prev_x, prev_y, scr_x, scr_y, color, px, py, pw, ph, Z_DATA,
                        );
                    }

                    // Arrow head at intervals (cell resolution, drawn on top)
                    if idx % arrow_interval == arrow_interval / 2 && idx > 0 {
                        let (fdx, fdy) = interpolate_field(&self.field, ptx, pty);
                        let ch = arrow_char(fdx, -fdy, &self.theme.chars.arrow);
                        pb.set_char(xi, yi, ch, color, Z_MARKER);
                    }

                    prev_screen = Some((scr_x, scr_y));
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);
    }
}
