//! Beeswarm (swarm) plot widget with collision avoidance.

use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBackend, Z_CHROME, Z_MARKER, create_backend};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::ticker::NullLocator;

/// A single group (category) of data in a swarm plot.
#[derive(Clone, Debug)]
pub struct SwarmGroup {
    /// Category label.
    pub name: String,
    /// Raw data values.
    pub data: Vec<f64>,
    /// Point color.
    pub color: Color,
}

impl SwarmGroup {
    /// Create a new swarm group.
    pub fn new(name: impl Into<String>, data: Vec<f64>, color: Color) -> Self {
        Self {
            name: name.into(),
            data,
            color,
        }
    }
}

/// A beeswarm plot widget with collision avoidance.
///
/// Points are displaced horizontally to avoid overlap, producing a
/// distribution-revealing "swarm" pattern. Uses a simple greedy algorithm:
/// for each point (sorted by value), try center x first, then shift
/// left/right until a free cell is found.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::widgets::swarm::{SwarmPlot, SwarmGroup};
///
/// let plot = SwarmPlot::new()
///     .group(SwarmGroup::new("A", vec![1.0, 1.1, 1.2, 2.0, 3.0], Color::Cyan))
///     .group(SwarmGroup::new("B", vec![2.0, 2.5, 3.0, 3.5, 4.0], Color::Yellow))
///     .title("Swarm Plot");
/// ```
pub struct SwarmPlot {
    groups: Vec<SwarmGroup>,
    title: Option<String>,
    y_axis: Axis,
    point_size: u16,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for SwarmPlot {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            title: None,
            y_axis: Axis::new(),
            point_size: 1,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl SwarmPlot {
    /// Create a new empty swarm plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a group (category) of data.
    pub fn group(mut self, g: SwarmGroup) -> Self {
        self.groups.push(g);
        self
    }

    /// Set the plot title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Set the y-axis configuration.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the point size (1 = single character).
    pub fn point_size(mut self, size: u16) -> Self {
        self.point_size = size.max(1);
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
impl crate::plotters_render::PlottersRenderable for SwarmPlot {
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

impl Widget for &SwarmPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        #[cfg(feature = "plotters-render")]
        {
            if crate::plotters_render::should_use_plotters() {
                use crate::plotters_render::PlottersRenderable;
                self.render_plotters(area, buf, &self.theme);
                return;
            }
        }
        if area.width < 4 || area.height < 4 || self.groups.is_empty() {
            return;
        }

        // Compute global y range
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
        if !y_min.is_finite() || !y_max.is_finite() {
            return;
        }
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Use NullLocator for x-axis (categories drawn manually)
        let x_axis = Axis::new().locator(NullLocator);
        let n = self.groups.len();
        let x_lo = 0.0;
        let x_hi = n as f64;

        let mut pb = create_backend(area);

        // Create and render the plot frame
        let frame = PlotFrame::new(&x_axis, &self.y_axis, &self.theme)
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

        // Track occupied screen cells for collision avoidance
        let mut occupied: HashSet<(u16, u16)> = HashSet::new();

        let marker = if self.point_size > 1 {
            self.theme.chars.marker.default_point
        } else {
            self.theme.chars.marker.small_point
        };

        // Max horizontal displacement (half the column width)
        let col_width = pa.width / n.max(1) as u16;
        let max_displacement = (col_width / 2).saturating_sub(1).max(1) as i32;

        for (i, group) in self.groups.iter().enumerate() {
            let center_x = pa.x + (i as u16 * pa.width / n as u16) + pa.width / n as u16 / 2;

            // Sort data by value for better packing
            let mut sorted_data: Vec<f64> = group
                .data
                .iter()
                .copied()
                .filter(|v| v.is_finite())
                .collect();
            sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            for &val in &sorted_data {
                let sy = pa.screen_y(val).round() as u16;

                // Skip if outside plot area vertically
                if sy < pa.y || sy >= pa.y + pa.height {
                    continue;
                }

                // Try placing at center, then shift outward
                let mut placed = false;
                for offset in 0..=max_displacement {
                    let candidates: Vec<u16> = if offset == 0 {
                        vec![center_x]
                    } else {
                        let mut c = Vec::new();
                        let right = center_x.saturating_add(offset as u16);
                        let left = center_x.saturating_sub(offset as u16);
                        c.push(right);
                        if left != right {
                            c.push(left);
                        }
                        c
                    };

                    for &cx in &candidates {
                        if cx >= pa.x && cx < pa.x + pa.width && !occupied.contains(&(cx, sy)) {
                            pb.set_char(cx, sy, marker, group.color, Z_MARKER);
                            occupied.insert((cx, sy));
                            placed = true;
                            break;
                        }
                    }
                    if placed {
                        break;
                    }
                }
            }

            // Category label
            let label = &group.name;
            let label_start = center_x.saturating_sub(label.len() as u16 / 2);
            let label_y = pa.y + pa.height;
            if label_y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < area.x + area.width {
                        pb.set_char(lx, label_y, ch, self.theme.axis_color, Z_CHROME);
                    }
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
