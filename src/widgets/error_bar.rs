//! Standalone error bar plot widget.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::plot_buffer::{PlotBackend, Z_DATA, Z_MARKER, create_backend};
use crate::spines::Spines;
use crate::theme::Theme;

/// Error bar direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorDirection {
    Vertical,
    Horizontal,
    Both,
}

/// A standalone error bar plot.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::error_bar::ErrorBarPlot;
///
/// let plot = ErrorBarPlot::new()
///     .data(vec![(1.0, 2.0)], vec![0.3], vec![0.5])
///     .title("Measurement Errors");
/// ```
pub struct ErrorBarPlot {
    points: Vec<(f64, f64)>,
    y_err_low: Vec<f64>,
    y_err_high: Vec<f64>,
    x_err_low: Vec<f64>,
    x_err_high: Vec<f64>,
    direction: ErrorDirection,
    color: Option<Color>,
    name: Option<String>,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
    show_legend: bool,
    legend_position: LegendPosition,
}

impl Default for ErrorBarPlot {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            y_err_low: Vec::new(),
            y_err_high: Vec::new(),
            x_err_low: Vec::new(),
            x_err_high: Vec::new(),
            direction: ErrorDirection::Vertical,
            color: None,
            name: None,
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
            show_legend: true,
            legend_position: LegendPosition::TopRight,
        }
    }
}

impl ErrorBarPlot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set data points with symmetric/asymmetric y-error bars.
    pub fn data(mut self, points: Vec<(f64, f64)>, err_low: Vec<f64>, err_high: Vec<f64>) -> Self {
        self.points = points;
        self.y_err_low = err_low;
        self.y_err_high = err_high;
        self
    }

    /// Set horizontal error bars.
    pub fn x_errors(mut self, err_low: Vec<f64>, err_high: Vec<f64>) -> Self {
        self.x_err_low = err_low;
        self.x_err_high = err_high;
        if self.direction == ErrorDirection::Vertical {
            self.direction = ErrorDirection::Both;
        }
        self
    }

    pub fn direction(mut self, d: ErrorDirection) -> Self {
        self.direction = d;
        self
    }

    pub fn color(mut self, c: Color) -> Self {
        self.color = Some(c);
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

    pub fn theme(mut self, t: Theme) -> Self {
        self.theme = t;
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

    /// Set the series name (used for legend display).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Show or hide the legend.
    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    /// Set the legend position.
    pub fn legend_position(mut self, pos: LegendPosition) -> Self {
        self.legend_position = pos;
        self
    }
}


impl Widget for &ErrorBarPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.points.is_empty() {
            return;
        }

        // Compute bounds
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for (i, &(x, y)) in self.points.iter().enumerate() {
            let xlo = x - self.x_err_low.get(i).copied().unwrap_or(0.0);
            let xhi = x + self.x_err_high.get(i).copied().unwrap_or(0.0);
            let ylo = y - self.y_err_low.get(i).copied().unwrap_or(0.0);
            let yhi = y + self.y_err_high.get(i).copied().unwrap_or(0.0);
            x_min = x_min.min(xlo);
            x_max = x_max.max(xhi);
            y_min = y_min.min(ylo);
            y_max = y_max.max(yhi);
        }

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

        let color = self.color.unwrap_or(self.theme.primary);

        // Draw error bars and points
        for (i, &(x, y)) in self.points.iter().enumerate() {
            let sx = pa.screen_x(x);
            let sy = pa.screen_y(y);
            let xi = sx.round() as u16;
            let yi = sy.round() as u16;

            // Vertical error bars
            if matches!(
                self.direction,
                ErrorDirection::Vertical | ErrorDirection::Both
            ) {
                let elo = self.y_err_low.get(i).copied().unwrap_or(0.0);
                let ehi = self.y_err_high.get(i).copied().unwrap_or(0.0);
                let sy_lo = pa.screen_y(y - elo);
                let sy_hi = pa.screen_y(y + ehi);
                let y_top = sy_hi.round() as u16;
                let y_bot = sy_lo.round() as u16;

                if xi >= pa.x && xi < pa.x + pa.width {
                    for ey in y_top..=y_bot {
                        if ey >= pa.y && ey < pa.y + pa.height {
                            pb.set_char(xi, ey, self.theme.chars.border.vertical, color, Z_DATA);
                        }
                    }
                    if y_top >= pa.y && y_top < pa.y + pa.height {
                        pb.set_char(xi, y_top, self.theme.chars.tick.cap_top, color, Z_DATA);
                    }
                    if y_bot >= pa.y && y_bot < pa.y + pa.height {
                        pb.set_char(xi, y_bot, self.theme.chars.tick.cap_bottom, color, Z_DATA);
                    }
                }
            }

            // Horizontal error bars
            if matches!(
                self.direction,
                ErrorDirection::Horizontal | ErrorDirection::Both
            ) {
                let elo = self.x_err_low.get(i).copied().unwrap_or(0.0);
                let ehi = self.x_err_high.get(i).copied().unwrap_or(0.0);
                let sx_lo = pa.screen_x(x - elo);
                let sx_hi = pa.screen_x(x + ehi);
                let x_left = sx_lo.round() as u16;
                let x_right = sx_hi.round() as u16;

                if yi >= pa.y && yi < pa.y + pa.height {
                    for ex in x_left..=x_right {
                        if ex >= pa.x && ex < pa.x + pa.width {
                            pb.set_char(ex, yi, self.theme.chars.border.horizontal, color, Z_DATA);
                        }
                    }
                    if x_left >= pa.x && x_left < pa.x + pa.width {
                        pb.set_char(x_left, yi, self.theme.chars.tick.cap_left, color, Z_DATA);
                    }
                    if x_right >= pa.x && x_right < pa.x + pa.width {
                        pb.set_char(x_right, yi, self.theme.chars.tick.cap_right, color, Z_DATA);
                    }
                }
            }

            // Draw center point
            if pa.contains(xi, yi) {
                pb.set_char(
                    xi,
                    yi,
                    self.theme.chars.marker.default_point,
                    color,
                    Z_MARKER,
                );
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite to buffer before legend
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Draw legend (directly to buf, after composite)
        if self.show_legend
            && let Some(ref name) = self.name
        {
            let entries = vec![LegendEntry {
                name: name.clone(),
                color,
                marker: Some(self.theme.chars.marker.default_point),
            }];
            let legend = Legend::new(entries)
                .position(self.legend_position.clone())
                .theme(self.theme.clone());
            let legend_area = Rect::new(pa.x, pa.y, pa.width, pa.height);
            (&legend).render(legend_area, buf);
        }
    }
}
