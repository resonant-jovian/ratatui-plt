//! Stem plot widget for discrete event visualization.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::drawing::draw_braille_line;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::spines::Spines;
use crate::style::MarkerShape;
use crate::theme::Theme;

/// A stem plot widget — vertical lines from a baseline to data points.
///
/// Ideal for discrete events (supernovae, collision events, impulse responses).
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::stem_plot::StemPlot;
/// use ratatui::style::Color;
///
/// let plot = StemPlot::new(vec![(1.0, 3.0), (2.0, 5.0), (3.0, 2.0)])
///     .color(Color::Green)
///     .baseline(0.0)
///     .title("Events");
/// ```
pub struct StemPlot {
    data: Vec<(f64, f64)>,
    baseline: f64,
    color: Color,
    marker: MarkerShape,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl StemPlot {
    pub fn new(data: Vec<(f64, f64)>) -> Self {
        Self {
            data,
            baseline: 0.0,
            color: Color::Cyan,
            marker: MarkerShape::FilledCircle,
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }

    pub fn baseline(mut self, b: f64) -> Self {
        self.baseline = b;
        self
    }

    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }

    pub fn marker(mut self, m: MarkerShape) -> Self {
        self.marker = m;
        self
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
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

impl Widget for &StemPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.data.is_empty() {
            return;
        }

        // Compute bounds
        let x_min = self.data.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x_max = self
            .data
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let y_min = self
            .data
            .iter()
            .map(|p| p.1)
            .chain(std::iter::once(self.baseline))
            .fold(f64::INFINITY, f64::min);
        let y_max = self
            .data
            .iter()
            .map(|p| p.1)
            .chain(std::iter::once(self.baseline))
            .fold(f64::NEG_INFINITY, f64::max);

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Create and render the plot frame (title, axes, grid, ticks, labels, spines, ref lines)
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(
            area,
            buf,
            DataBounds {
                x_lo,
                x_hi,
                y_lo,
                y_hi,
            },
        ) else {
            return;
        };

        // Draw baseline
        let base_sy = pa.screen_y(self.baseline);
        let base_yi = base_sy.round() as u16;
        if base_yi >= pa.y && base_yi < pa.y + pa.height {
            for x in pa.x..pa.x + pa.width {
                buf[(x, base_yi)]
                    .set_char('─')
                    .set_fg(self.theme.axis_color);
            }
        }

        // Draw stems and markers
        for &(x, y) in &self.data {
            let sx = pa.screen_x(x);
            let sy = pa.screen_y(y);
            let xi = sx.round() as u16;
            let yi = sy.round() as u16;

            if xi < pa.x || xi >= pa.x + pa.width {
                continue;
            }

            // Draw stem line using Braille sub-pixel rendering
            draw_braille_line(buf, sx, base_sy, sx, sy, self.color, &pa);

            // Draw marker at data point
            if pa.contains(xi, yi) {
                buf[(xi, yi)]
                    .set_char(self.marker.char())
                    .set_fg(self.color);
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations(&pa, &self.annotations, buf);
    }
}
