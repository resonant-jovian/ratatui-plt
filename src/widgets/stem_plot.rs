//! Stem plot widget for discrete event visualization.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBackend, Z_CHROME, Z_DATA, Z_MARKER, create_backend};
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
    color: Option<Color>,
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
            color: None,
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
        self.color = Some(c);
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

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for StemPlot {
    fn render_plotters(
        &self,
        area: ratatui::layout::Rect,
        buf: &mut ratatui::buffer::Buffer,
        theme: &crate::theme::Theme,
    ) {
        use crate::plotters_render::{bridge, helpers, theme_bridge};
        use crate::series::is_valid_point;

        if self.data.is_empty() {
            return;
        }

        // Compute bounds
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = self.baseline;
        let mut y_max = self.baseline;
        for &(x, y) in &self.data {
            if is_valid_point(x, y) {
                x_min = x_min.min(x);
                x_max = x_max.max(x);
                y_min = y_min.min(y);
                y_max = y_max.max(y);
            }
        }
        if x_min.is_infinite() { return; }
        let y_padding = (y_max - y_min).abs() * 0.05;
        y_min -= y_padding;
        y_max += y_padding;
        if (y_max - y_min).abs() < 1e-12 { y_max = y_min + 1.0; }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        let x_axis_ref = &self.x_axis;
        let y_axis_ref = &self.y_axis;
        let title_ref = self.title.as_deref();
        let data_ref = &self.data;
        let baseline = self.baseline;
        let stem_color = self.color.unwrap_or(theme.color_cycle.at(0));

        bridge::render_plotters_to_buf(
            area,
            buf,
            theme_bridge::theme_bg_rgb(theme),
            |root| {
                let Ok(mut chart) = helpers::build_cartesian_2d(
                    root, x_axis_ref, y_axis_ref, title_ref, theme,
                    x_lo..x_hi, y_lo..y_hi,
                ) else { return; };

                let pc = theme_bridge::to_plotters_color(stem_color);

                // Draw baseline
                let _ = chart.draw_series(
                    plotters::series::LineSeries::new(
                        vec![(x_lo, baseline), (x_hi, baseline)],
                        plotters::style::ShapeStyle::from(pc).stroke_width(1),
                    ),
                );

                // Draw stems (vertical lines) and markers
                for &(x, y) in data_ref {
                    if !is_valid_point(x, y) { continue; }

                    // Vertical line from baseline to point
                    let _ = chart.draw_series(
                        plotters::series::LineSeries::new(
                            vec![(x, baseline), (x, y)],
                            plotters::style::ShapeStyle::from(pc).stroke_width(1),
                        ),
                    );

                    // Marker at the top
                    let _ = chart.draw_series(std::iter::once(
                        plotters::element::Circle::new(
                            (x, y), 4,
                            plotters::style::ShapeStyle::from(pc).filled(),
                        ),
                    ));
                }
            },
        );
    }
}

impl Widget for &StemPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        #[cfg(feature = "plotters-render")]
        {
            if crate::plotters_render::should_use_plotters() {
                use crate::plotters_render::PlottersRenderable;
                self.render_plotters(area, buf, &self.theme);
                return;
            }
        }
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

        let mut pb = create_backend(area);

        // Create and render the plot frame (title, axes, grid, ticks, labels, spines, ref lines)
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let bounds = DataBounds {
            x_lo,
            x_hi,
            y_lo,
            y_hi,
        };

        let Some(pa) = frame.render_to_pb(&mut pb, area, bounds) else {
            return;
        };

        // Draw baseline
        let base_sy = pa.screen_y(self.baseline);
        let base_yi = base_sy.round() as u16;
        if base_yi >= pa.y && base_yi < pa.y + pa.height {
            for x in pa.x..pa.x + pa.width {
                pb.set_char(
                    x,
                    base_yi,
                    self.theme.chars.border.horizontal,
                    self.theme.axis_color,
                    Z_CHROME,
                );
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

            // Draw stem line using vertical line characters
            let resolved_color = self.color.unwrap_or(self.theme.primary);
            let y_top = yi.min(base_yi);
            let y_bot = yi.max(base_yi);
            for row in y_top..=y_bot {
                if pa.contains(xi, row) {
                    pb.set_char(
                        xi,
                        row,
                        self.theme.chars.border.vertical,
                        resolved_color,
                        Z_DATA,
                    );
                }
            }

            // Draw marker at data point
            if pa.contains(xi, yi) {
                pb.set_char(xi, yi, self.marker.char(), resolved_color, Z_MARKER);
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite to buffer
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);
    }
}
