//! Vector field (quiver) plot widget.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis};
use crate::colormap::{Colormap, Viridis};
use crate::frame::{PlotFrame, ReferenceLine};
use crate::norm::{LinearNorm, Normalize};
use crate::series::VectorFieldData;
use crate::spines::Spines;
use crate::theme::Theme;

/// A 2D vector field (quiver) plot widget.
///
/// Renders arrows at grid points showing vector direction and magnitude.
/// Useful for velocity fields, gravitational fields, etc.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let field = VectorFieldData::from_fn(
///     (-2.0, 2.0), (-2.0, 2.0), 10, 10,
///     |x, y| (-y, x), // Circular flow
/// );
/// let plot = VectorField::new(field).title("Circular Flow");
/// ```
pub struct VectorField {
    data: VectorFieldData,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    aspect_ratio: AspectRatio,
    color: Color,
    color_by_magnitude: bool,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    arrow_scale: f64,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl VectorField {
    pub fn new(data: VectorFieldData) -> Self {
        let max_mag = data.max_magnitude();
        Self {
            data,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            aspect_ratio: AspectRatio::Auto,
            color: Color::Cyan,
            color_by_magnitude: false,
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(
                0.0,
                if max_mag == 0.0 { 1.0 } else { max_mag },
            )),
            arrow_scale: 1.0,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }

    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }

    /// Color arrows by their magnitude using the colormap.
    pub fn color_by_magnitude(mut self, enable: bool) -> Self {
        self.color_by_magnitude = enable;
        self
    }

    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    /// Scale factor for arrow length.
    pub fn arrow_scale(mut self, scale: f64) -> Self {
        self.arrow_scale = scale;
        self
    }

    /// Set the theme.
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

/// Choose an arrow character based on the direction angle.
fn arrow_char(dx: f64, dy: f64) -> char {
    if dx == 0.0 && dy == 0.0 {
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

impl Widget for &VectorField {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.data.vectors.is_empty() {
            return;
        }

        // Compute bounds
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for &(x, y, _, _) in &self.data.vectors {
            x_min = x_min.min(x);
            x_max = x_max.max(x);
            y_min = y_min.min(y);
            y_max = y_max.max(y);
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Create and render the plot frame (title, axes, grid, ticks, labels, spines, ref lines)
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .aspect_ratio(self.aspect_ratio.clone())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, x_lo, x_hi, y_lo, y_hi) else {
            return;
        };

        // Draw arrows
        let max_mag = self.data.max_magnitude();
        for &(x, y, dx, dy) in &self.data.vectors {
            let sx = pa.screen_x(x);
            let sy = pa.screen_y(y);
            let xi = sx.round() as u16;
            let yi = sy.round() as u16;

            if pa.contains(xi, yi) {
                let mag = (dx * dx + dy * dy).sqrt();
                let color = if self.color_by_magnitude && max_mag > 0.0 {
                    let t = self.norm.normalize(mag);
                    self.colormap.color_at(t)
                } else {
                    self.color
                };

                let ch = arrow_char(dx, -dy); // Negate dy because screen y is inverted
                buf[(xi, yi)].set_char(ch).set_fg(color);
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations(&pa, &self.annotations, buf);
    }
}
