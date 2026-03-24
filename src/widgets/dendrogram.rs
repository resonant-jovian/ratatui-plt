//! Dendrogram (hierarchical clustering tree) widget.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA};
use crate::spines::Spines;
use crate::theme::Theme;

/// Orientation of the dendrogram.
#[derive(Clone, Debug, Default)]
pub enum DendroOrientation {
    /// Leaves at the bottom, merges grow upward (default).
    #[default]
    Bottom,
    /// Leaves at the top, merges grow downward.
    Top,
    /// Leaves on the left, merges grow rightward.
    Left,
    /// Leaves on the right, merges grow leftward.
    Right,
}

/// A single link in the dendrogram representing a merge of two clusters.
///
/// For `n` leaves there are `n-1` links. Leaf indices are `0..n` and
/// merged cluster indices start at `n` (link `i` creates cluster `n + i`).
#[derive(Clone, Debug)]
pub struct DendroLink {
    /// Index of the left child (leaf index `0..n` or merged cluster `n+i`).
    pub left: usize,
    /// Index of the right child.
    pub right: usize,
    /// Merge distance (height at which the two children are joined).
    pub distance: f64,
}

impl DendroLink {
    /// Create a new dendrogram link.
    pub fn new(left: usize, right: usize, distance: f64) -> Self {
        Self {
            left,
            right,
            distance,
        }
    }
}

/// A dendrogram (hierarchical clustering tree) widget.
///
/// Displays a tree of U-shaped connectors where each merge is shown at its
/// merge distance. Leaves are evenly spaced along the category axis.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::widgets::dendrogram::{Dendrogram, DendroLink};
///
/// let links = vec![
///     DendroLink::new(0, 1, 1.0),
///     DendroLink::new(2, 3, 1.5),
///     DendroLink::new(4, 5, 3.0),
/// ];
/// let labels = vec!["A".into(), "B".into(), "C".into(), "D".into()];
/// let plot = Dendrogram::new(links, labels).title("Clustering");
/// ```
pub struct Dendrogram {
    links: Vec<DendroLink>,
    labels: Vec<String>,
    title: Option<String>,
    orientation: DendroOrientation,
    color_threshold: Option<f64>,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Dendrogram {
    /// Create a dendrogram from merge links and leaf labels.
    ///
    /// `labels` has length `n` (number of leaves), and `links` has length `n-1`.
    /// Each link merges two clusters; leaf indices are `0..n` and merged cluster
    /// `i` has index `n + i`.
    pub fn new(links: Vec<DendroLink>, labels: Vec<String>) -> Self {
        Self {
            links,
            labels,
            title: None,
            orientation: DendroOrientation::default(),
            color_threshold: None,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }

    /// Set the plot title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Set the dendrogram orientation.
    pub fn orientation(mut self, o: DendroOrientation) -> Self {
        self.orientation = o;
        self
    }

    /// Set a color threshold: clusters merging below this height use
    /// distinct colors from a built-in palette.
    pub fn color_threshold(mut self, threshold: f64) -> Self {
        self.color_threshold = Some(threshold);
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

    /// Compute the position (on the category axis) for each node.
    ///
    /// Leaves are placed at 0, 1, 2, ..., n-1. Merged clusters are at the
    /// average of their children's positions.
    fn compute_positions(&self) -> Vec<f64> {
        let n = self.labels.len();
        let total = n + self.links.len();
        let mut positions = vec![0.0; total];

        // Leaf positions
        for (i, pos) in positions.iter_mut().enumerate().take(n) {
            *pos = i as f64;
        }

        // Merged cluster positions
        for (i, link) in self.links.iter().enumerate() {
            let left_pos = if link.left < total {
                positions[link.left]
            } else {
                0.0
            };
            let right_pos = if link.right < total {
                positions[link.right]
            } else {
                0.0
            };
            positions[n + i] = (left_pos + right_pos) / 2.0;
        }

        positions
    }

    /// Compute the height (merge distance) for each node.
    ///
    /// Leaves have height 0, merged clusters have their link distance.
    fn compute_heights(&self) -> Vec<f64> {
        let n = self.labels.len();
        let total = n + self.links.len();
        let mut heights = vec![0.0; total];

        for (i, link) in self.links.iter().enumerate() {
            heights[n + i] = link.distance;
        }

        heights
    }

    /// Assign colors to links based on the color threshold.
    fn compute_colors(&self) -> Vec<Color> {
        let cluster_colors = [
            Color::Cyan,
            Color::Red,
            Color::Green,
            Color::Yellow,
            Color::Magenta,
            Color::Blue,
            Color::LightCyan,
            Color::LightRed,
        ];

        let n = self.links.len();
        let mut colors = vec![self.theme.foreground; n];

        if let Some(threshold) = self.color_threshold {
            let mut color_idx = 0;
            for (i, link) in self.links.iter().enumerate() {
                if link.distance <= threshold {
                    colors[i] = cluster_colors[color_idx % cluster_colors.len()];
                    color_idx += 1;
                }
            }
        }

        colors
    }
}

impl Widget for &Dendrogram {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let n = self.labels.len();
        if n == 0 || self.links.is_empty() {
            return;
        }

        let positions = self.compute_positions();
        let heights = self.compute_heights();
        let link_colors = self.compute_colors();

        // Data bounds
        let cat_lo = -0.5;
        let cat_hi = (n - 1) as f64 + 0.5;
        let max_height = self
            .links
            .iter()
            .map(|l| l.distance)
            .fold(0.0_f64, f64::max);
        let height_lo = 0.0;
        let height_hi = max_height * 1.1;

        // Map orientation to x/y axes
        let is_horizontal = matches!(
            self.orientation,
            DendroOrientation::Bottom | DendroOrientation::Top
        );

        let (x_lo, x_hi, y_lo, y_hi) = if is_horizontal {
            (cat_lo, cat_hi, height_lo, height_hi)
        } else {
            (height_lo, height_hi, cat_lo, cat_hi)
        };

        let x_axis = Axis::new();
        let y_axis = Axis::new();

        let mut pb = PlotBuffer::new(area);

        let frame = PlotFrame::new(&x_axis, &y_axis, &self.theme)
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

        // Draw U-shaped connectors for each link
        for (i, link) in self.links.iter().enumerate() {
            let left_pos = if link.left < positions.len() {
                positions[link.left]
            } else {
                continue;
            };
            let right_pos = if link.right < positions.len() {
                positions[link.right]
            } else {
                continue;
            };
            let left_height = if link.left < heights.len() {
                heights[link.left]
            } else {
                0.0
            };
            let right_height = if link.right < heights.len() {
                heights[link.right]
            } else {
                0.0
            };
            let merge_height = link.distance;
            let color = link_colors[i];

            match self.orientation {
                DendroOrientation::Bottom => {
                    draw_vertical_segment_pb(
                        &mut pb,
                        &pa,
                        left_pos,
                        left_height,
                        merge_height,
                        color,
                    );
                    draw_vertical_segment_pb(
                        &mut pb,
                        &pa,
                        right_pos,
                        right_height,
                        merge_height,
                        color,
                    );
                    draw_horizontal_segment_pb(
                        &mut pb,
                        &pa,
                        left_pos,
                        right_pos,
                        merge_height,
                        color,
                    );
                }
                DendroOrientation::Top => {
                    let flip_h = |h: f64| height_hi - h;
                    draw_vertical_segment_pb(
                        &mut pb,
                        &pa,
                        left_pos,
                        flip_h(left_height),
                        flip_h(merge_height),
                        color,
                    );
                    draw_vertical_segment_pb(
                        &mut pb,
                        &pa,
                        right_pos,
                        flip_h(right_height),
                        flip_h(merge_height),
                        color,
                    );
                    draw_horizontal_segment_pb(
                        &mut pb,
                        &pa,
                        left_pos,
                        right_pos,
                        flip_h(merge_height),
                        color,
                    );
                }
                DendroOrientation::Left => {
                    draw_horizontal_segment_h_pb(
                        &mut pb,
                        &pa,
                        left_pos,
                        left_height,
                        merge_height,
                        color,
                    );
                    draw_horizontal_segment_h_pb(
                        &mut pb,
                        &pa,
                        right_pos,
                        right_height,
                        merge_height,
                        color,
                    );
                    draw_vertical_segment_h_pb(
                        &mut pb,
                        &pa,
                        left_pos,
                        right_pos,
                        merge_height,
                        color,
                    );
                }
                DendroOrientation::Right => {
                    let flip_h = |h: f64| height_hi - h;
                    draw_horizontal_segment_h_pb(
                        &mut pb,
                        &pa,
                        left_pos,
                        flip_h(left_height),
                        flip_h(merge_height),
                        color,
                    );
                    draw_horizontal_segment_h_pb(
                        &mut pb,
                        &pa,
                        right_pos,
                        flip_h(right_height),
                        flip_h(merge_height),
                        color,
                    );
                    draw_vertical_segment_h_pb(
                        &mut pb,
                        &pa,
                        left_pos,
                        right_pos,
                        flip_h(merge_height),
                        color,
                    );
                }
            }
        }

        // Draw leaf labels along the category axis
        if is_horizontal {
            for (i, label) in self.labels.iter().enumerate() {
                let sx = pa.screen_x(i as f64).round() as u16;
                let label_y = pa.y + pa.height; // below the plot area
                if label_y < pa.area.y + pa.area.height {
                    let start = sx.saturating_sub(label.len() as u16 / 2);
                    for (j, ch) in label.chars().enumerate() {
                        let lx = start + j as u16;
                        if lx >= pa.area.x && lx < pa.area.x + pa.area.width {
                            pb.set_char(lx, label_y, ch, self.theme.axis_color, Z_CHROME);
                        }
                    }
                }
            }
        } else {
            for (i, label) in self.labels.iter().enumerate() {
                let sy = pa.screen_y(i as f64).round() as u16;
                if sy >= pa.y && sy < pa.y + pa.height && pa.x > 0 {
                    let label_x = pa.x.saturating_sub(label.len() as u16 + 1);
                    for (j, ch) in label.chars().enumerate() {
                        let lx = label_x + j as u16;
                        if lx >= pa.area.x && lx < pa.x {
                            pb.set_char(lx, sy, ch, self.theme.axis_color, Z_CHROME);
                        }
                    }
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite to buffer
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);
    }
}

use crate::frame::PlotArea;

/// Draw a vertical line segment (for Bottom/Top orientation) into a PlotBuffer.
fn draw_vertical_segment_pb(
    pb: &mut PlotBuffer,
    pa: &PlotArea,
    cat: f64,
    h0: f64,
    h1: f64,
    color: Color,
) {
    let sx = pa.screen_x(cat).round() as u16;
    let sy0 = pa.screen_y(h0).round() as i32;
    let sy1 = pa.screen_y(h1).round() as i32;
    let (top, bot) = if sy0 < sy1 { (sy0, sy1) } else { (sy1, sy0) };

    if sx >= pa.x && sx < pa.x + pa.width {
        for sy in top..=bot {
            let y = sy as u16;
            if y >= pa.y && y < pa.y + pa.height {
                pb.set_char(sx, y, '│', color, Z_DATA);
            }
        }
    }
}

/// Draw a horizontal line segment (for Bottom/Top orientation) into a PlotBuffer.
fn draw_horizontal_segment_pb(
    pb: &mut PlotBuffer,
    pa: &PlotArea,
    cat0: f64,
    cat1: f64,
    h: f64,
    color: Color,
) {
    let sy = pa.screen_y(h).round() as u16;
    let sx0 = pa.screen_x(cat0).round() as i32;
    let sx1 = pa.screen_x(cat1).round() as i32;
    let (left, right) = if sx0 < sx1 { (sx0, sx1) } else { (sx1, sx0) };

    if sy >= pa.y && sy < pa.y + pa.height {
        for sx in left..=right {
            let x = sx as u16;
            if x >= pa.x && x < pa.x + pa.width {
                pb.set_char(x, sy, '─', color, Z_DATA);
            }
        }
        // Draw corner connectors
        let lx = left as u16;
        let rx = right as u16;
        if lx >= pa.x && lx < pa.x + pa.width {
            pb.set_char(lx, sy, '┌', color, Z_DATA);
        }
        if rx >= pa.x && rx < pa.x + pa.width {
            pb.set_char(rx, sy, '┐', color, Z_DATA);
        }
    }
}

/// Draw a horizontal segment in Left/Right orientation into a PlotBuffer.
fn draw_horizontal_segment_h_pb(
    pb: &mut PlotBuffer,
    pa: &PlotArea,
    cat: f64,
    h0: f64,
    h1: f64,
    color: Color,
) {
    let sy = pa.screen_y(cat).round() as u16;
    let sx0 = pa.screen_x(h0).round() as i32;
    let sx1 = pa.screen_x(h1).round() as i32;
    let (left, right) = if sx0 < sx1 { (sx0, sx1) } else { (sx1, sx0) };

    if sy >= pa.y && sy < pa.y + pa.height {
        for sx in left..=right {
            let x = sx as u16;
            if x >= pa.x && x < pa.x + pa.width {
                pb.set_char(x, sy, '─', color, Z_DATA);
            }
        }
    }
}

/// Draw a vertical segment in Left/Right orientation into a PlotBuffer.
fn draw_vertical_segment_h_pb(
    pb: &mut PlotBuffer,
    pa: &PlotArea,
    cat0: f64,
    cat1: f64,
    h: f64,
    color: Color,
) {
    let sx = pa.screen_x(h).round() as u16;
    let sy0 = pa.screen_y(cat0).round() as i32;
    let sy1 = pa.screen_y(cat1).round() as i32;
    let (top, bot) = if sy0 < sy1 { (sy0, sy1) } else { (sy1, sy0) };

    if sx >= pa.x && sx < pa.x + pa.width {
        for sy in top..=bot {
            let y = sy as u16;
            if y >= pa.y && y < pa.y + pa.height {
                pb.set_char(sx, y, '│', color, Z_DATA);
            }
        }
        // Draw corner connectors
        let ty = top as u16;
        let by = bot as u16;
        if ty >= pa.y && ty < pa.y + pa.height {
            pb.set_char(sx, ty, '┌', color, Z_DATA);
        }
        if by >= pa.y && by < pa.y + pa.height {
            pb.set_char(sx, by, '└', color, Z_DATA);
        }
    }
}
