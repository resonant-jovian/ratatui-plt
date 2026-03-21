//! Strip plot widget for jittered categorical scatter plots.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::strip::{StripPlot, StripGroup};
//!
//! let plot = StripPlot::new()
//!     .group(StripGroup::new("Group A", vec![1.0, 2.0, 3.0, 4.0], Color::Cyan))
//!     .group(StripGroup::new("Group B", vec![2.0, 3.0, 5.0, 7.0], Color::Yellow))
//!     .title("Strip Plot");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::ticker::NullLocator;
use crate::transform::data_to_screen;

/// A single group in a strip plot.
#[derive(Clone, Debug)]
pub struct StripGroup {
    /// Group/category name.
    pub name: String,
    /// Data values for this group.
    pub data: Vec<f64>,
    /// Point color.
    pub color: Color,
}

impl StripGroup {
    /// Create a new strip group.
    pub fn new(name: impl Into<String>, data: Vec<f64>, color: Color) -> Self {
        Self {
            name: name.into(),
            data,
            color,
        }
    }
}

/// A strip plot widget showing jittered categorical scatter points.
///
/// For each group, data points are scattered horizontally with deterministic
/// jitter (hash-based, not random) for reproducibility. Uses a category-based
/// x-axis similar to `BoxPlot`.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::strip::{StripPlot, StripGroup};
/// use ratatui::style::Color;
///
/// let plot = StripPlot::new()
///     .group(StripGroup::new("A", vec![1.0, 2.0, 3.0], Color::Red))
///     .jitter(0.3)
///     .title("Strip Plot");
/// ```
pub struct StripPlot {
    groups: Vec<StripGroup>,
    title: Option<String>,
    y_axis: Axis,
    jitter: f64,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for StripPlot {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            title: None,
            y_axis: Axis::new(),
            jitter: 0.2,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl StripPlot {
    /// Create an empty strip plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a group.
    pub fn group(mut self, g: StripGroup) -> Self {
        self.groups.push(g);
        self
    }

    /// Set the plot title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the Y axis.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the amount of horizontal jitter (0.0 to 0.4, default 0.2).
    ///
    /// Higher values spread points more within each category.
    pub fn jitter(mut self, j: f64) -> Self {
        self.jitter = j.clamp(0.0, 0.4);
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

    /// Deterministic hash-based jitter for a value.
    ///
    /// Returns a value in [-jitter, +jitter] based on the bit pattern
    /// of the input, the point index, and the group index.
    fn compute_jitter(value: f64, point_index: usize, group_index: usize, jitter: f64) -> f64 {
        if jitter == 0.0 {
            return 0.0;
        }
        // Simple deterministic hash: mix the value bits with the indices
        let bits = value.to_bits();
        let hash = bits
            .wrapping_mul(0x517cc1b727220a95)
            .wrapping_add(point_index as u64 * 0x6c62272e07bb0142)
            .wrapping_add(group_index as u64 * 0x9e3779b97f4a7c15);
        // Map to [-1, 1] range
        let normalized = (hash as i64 as f64) / (i64::MAX as f64);
        normalized * jitter
    }
}

impl Widget for &StripPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.groups.is_empty() {
            return;
        }

        // Compute global y range from all groups
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for g in &self.groups {
            for &v in &g.data {
                if v.is_finite() {
                    y_min = y_min.min(v);
                    y_max = y_max.max(v);
                }
            }
        }
        if y_min.is_infinite() {
            y_min = 0.0;
            y_max = 1.0;
        }

        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Use NullLocator for x-axis (categories drawn manually like BoxPlot)
        let x_axis = Axis::new().locator(NullLocator);
        let n = self.groups.len();
        let x_lo = 0.0;
        let x_hi = n as f64;

        // Create and render the plot frame
        let frame = PlotFrame::new(&x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, DataBounds { x_lo, x_hi, y_lo, y_hi }) else {
            return;
        };

        // Draw each group's points with jitter
        for (gi, g) in self.groups.iter().enumerate() {
            let center_x_data = gi as f64 + 0.5;
            let center_sx = data_to_screen(
                center_x_data,
                x_lo,
                x_hi,
                pa.x as f64,
                (pa.x + pa.width - 1) as f64,
            );

            // Width of one category slot in screen pixels
            let slot_width = pa.width as f64 / n as f64;

            for (pi, &v) in g.data.iter().enumerate() {
                if !v.is_finite() {
                    continue;
                }

                let jitter_offset = StripPlot::compute_jitter(v, pi, gi, self.jitter) * slot_width;
                let sx = (center_sx + jitter_offset).round() as u16;
                let sy = data_to_screen(v, y_lo, y_hi, (pa.y + pa.height - 1) as f64, pa.y as f64)
                    .round() as u16;

                if pa.contains(sx, sy) {
                    buf[(sx, sy)].set_char('●').set_fg(g.color);
                }
            }

            // Draw category label
            let label = &g.name;
            let center_xi = center_sx.round() as u16;
            let label_start = center_xi.saturating_sub(label.len() as u16 / 2);
            let label_y = pa.y + pa.height;
            if label_y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        buf[(lx, label_y)]
                            .set_char(ch)
                            .set_fg(self.theme.axis_color);
                    }
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations(&pa, &self.annotations, buf);
    }
}
