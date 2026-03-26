//! Icicle chart widget for hierarchical data visualization.
//!
//! Renders hierarchical data as stacked rectangles where the root spans the full
//! width and children subdivide their parent's width proportionally. Like a
//! sunburst chart but in Cartesian coordinates.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::treemap::TreemapNode;
//! use ratatui_plt::widgets::icicle::{IcicleChart, IcicleOrientation};
//! use ratatui::style::Color;
//!
//! let root = TreemapNode::new("root", 0.0)
//!     .child(TreemapNode::new("A", 6.0).color(Color::Cyan)
//!         .child(TreemapNode::new("A1", 4.0).color(Color::Blue))
//!         .child(TreemapNode::new("A2", 2.0).color(Color::LightBlue)))
//!     .child(TreemapNode::new("B", 4.0).color(Color::Yellow));
//! let chart = IcicleChart::new(root)
//!     .orientation(IcicleOrientation::TopDown)
//!     .max_depth(3)
//!     .title("File Sizes");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::color_cycle::ColorCycle;
use crate::colormap::{Colormap, Viridis};
use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA};
use crate::theme::Theme;
use crate::widgets::treemap::TreemapNode;

/// Orientation of the icicle chart.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum IcicleOrientation {
    /// Root at top, children expand downward (default).
    #[default]
    TopDown,
    /// Root at bottom, children expand upward.
    BottomUp,
    /// Root at left, children expand rightward.
    LeftRight,
    /// Root at right, children expand leftward.
    RightLeft,
}

/// An icicle chart widget for hierarchical data.
///
/// Renders a tree hierarchy as nested rectangles stacked by depth level.
/// The root node spans the full width (or height, depending on orientation),
/// and each child subdivides its parent's extent proportionally.
pub struct IcicleChart {
    root: TreemapNode,
    title: Option<String>,
    max_depth: Option<usize>,
    orientation: IcicleOrientation,
    colormap: Box<dyn Colormap>,
    show_labels: bool,
    theme: Theme,
}

impl IcicleChart {
    /// Create a new icicle chart from a root node.
    pub fn new(root: TreemapNode) -> Self {
        Self {
            root,
            title: None,
            max_depth: None,
            orientation: IcicleOrientation::default(),
            colormap: Box::new(Viridis),
            show_labels: true,
            theme: Theme::get_default(),
        }
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Limit the maximum rendering depth.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set the chart orientation.
    pub fn orientation(mut self, orientation: IcicleOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set the colormap used for depth-based coloring.
    pub fn colormap(mut self, cmap: Box<dyn Colormap>) -> Self {
        self.colormap = cmap;
        self
    }

    /// Whether to show labels inside rectangles.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

/// Compute the maximum depth of a `TreemapNode` tree.
fn tree_max_depth(node: &TreemapNode) -> usize {
    if node.children.is_empty() {
        1
    } else {
        1 + node
            .children
            .iter()
            .map(tree_max_depth)
            .max()
            .unwrap_or(0)
    }
}

/// A flattened rectangle ready for rendering.
struct IcicleRect {
    /// Depth level (0 = root).
    level: usize,
    /// Start position along the primary axis (fraction 0.0..1.0).
    start_frac: f64,
    /// End position along the primary axis (fraction 0.0..1.0).
    end_frac: f64,
    /// Fill color.
    color: Color,
    /// Label text.
    label: String,
}

/// Context for the recursive flattening of the tree into renderable rectangles.
struct FlattenCtx<'a> {
    max_depth: Option<usize>,
    cycle: &'a mut ColorCycle,
    colormap: &'a dyn Colormap,
    total_depth: usize,
    out: &'a mut Vec<IcicleRect>,
}

/// Recursively flatten the tree into renderable rectangles.
fn flatten_rects(
    node: &TreemapNode,
    level: usize,
    start_frac: f64,
    end_frac: f64,
    ctx: &mut FlattenCtx<'_>,
) {
    // Check depth limit
    if let Some(max_d) = ctx.max_depth
        && level >= max_d
    {
        return;
    }

    // Push this node as a rectangle
    let color = node.color.unwrap_or_else(|| {
        if ctx.total_depth > 1 {
            ctx.colormap
                .color_at(level as f64 / (ctx.total_depth - 1) as f64)
        } else {
            ctx.cycle.next_color()
        }
    });

    ctx.out.push(IcicleRect {
        level,
        start_frac,
        end_frac,
        color,
        label: node.label.clone(),
    });

    // Recurse into children
    let total_val = node.effective_value();
    if total_val <= 0.0 || node.children.is_empty() {
        return;
    }

    let span = end_frac - start_frac;
    let mut cursor = start_frac;
    for child in &node.children {
        let child_val = child.effective_value();
        if child_val <= 0.0 {
            continue;
        }
        let child_frac = child_val / total_val;
        let child_start = cursor;
        let child_end = cursor + child_frac * span;

        flatten_rects(child, level + 1, child_start, child_end, ctx);

        cursor = child_end;
    }
}

impl Widget for &IcicleChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 3 {
            return;
        }

        let root_val = self.root.effective_value();
        if root_val <= 0.0 {
            return;
        }

        // Reserve space for title
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let plot_y = area.y + title_height;
        let plot_h = area.height.saturating_sub(title_height);

        if plot_h < 2 {
            return;
        }

        let mut pb = PlotBuffer::new(area);

        // Draw title (centered)
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    pb.set_char(x, area.y, ch, self.theme.foreground, Z_CHROME);
                }
            }
        }

        // Compute total tree depth and effective depth limit
        let total_depth = tree_max_depth(&self.root);
        let effective_depth = match self.max_depth {
            Some(d) => d.min(total_depth),
            None => total_depth,
        };

        if effective_depth == 0 {
            return;
        }

        // Flatten tree into rectangles
        let mut cycle = ColorCycle::default();
        let mut rects = Vec::new();
        let mut ctx = FlattenCtx {
            max_depth: self.max_depth,
            cycle: &mut cycle,
            colormap: &*self.colormap,
            total_depth: effective_depth,
            out: &mut rects,
        };
        flatten_rects(&self.root, 0, 0.0, 1.0, &mut ctx);

        if rects.is_empty() {
            pb.composite(buf);
            return;
        }

        // Determine whether we lay out along width (horizontal orientations)
        // or along height (vertical orientations).
        let horizontal = matches!(
            self.orientation,
            IcicleOrientation::TopDown | IcicleOrientation::BottomUp
        );

        // For each rectangle, map level to one axis and start/end to the other
        for r in &rects {
            let (rx, ry, rw, rh) = if horizontal {
                // Primary axis = width, depth axis = height
                let level_size = plot_h as f64 / effective_depth as f64;
                let level_pos = match self.orientation {
                    IcicleOrientation::TopDown => {
                        plot_y as f64 + r.level as f64 * level_size
                    }
                    IcicleOrientation::BottomUp => {
                        plot_y as f64
                            + (effective_depth - 1 - r.level) as f64 * level_size
                    }
                    _ => plot_y as f64,
                };

                let x_start = area.x as f64 + r.start_frac * area.width as f64;
                let x_end = area.x as f64 + r.end_frac * area.width as f64;

                let rx = x_start.round() as u16;
                let rw = (x_end.round() as u16).saturating_sub(rx).max(1);
                let ry = level_pos.round() as u16;
                let rh = (level_size.round() as u16).max(1);

                (rx, ry, rw, rh)
            } else {
                // Primary axis = height, depth axis = width
                let level_size = area.width as f64 / effective_depth as f64;
                let level_pos = match self.orientation {
                    IcicleOrientation::LeftRight => {
                        area.x as f64 + r.level as f64 * level_size
                    }
                    IcicleOrientation::RightLeft => {
                        area.x as f64
                            + (effective_depth - 1 - r.level) as f64 * level_size
                    }
                    _ => area.x as f64,
                };

                let y_start = plot_y as f64 + r.start_frac * plot_h as f64;
                let y_end = plot_y as f64 + r.end_frac * plot_h as f64;

                let rx = level_pos.round() as u16;
                let rw = (level_size.round() as u16).max(1);
                let ry = y_start.round() as u16;
                let rh = (y_end.round() as u16).saturating_sub(ry).max(1);

                (rx, ry, rw, rh)
            };

            // Draw the filled rectangle with borders
            for dy in 0..rh {
                for dx in 0..rw {
                    let x = rx + dx;
                    let y = ry + dy;

                    // Clip to area
                    if x >= area.x + area.width || y >= plot_y + plot_h {
                        continue;
                    }
                    if x < area.x || y < plot_y {
                        continue;
                    }

                    let is_border =
                        dx == 0 || dx == rw - 1 || dy == 0 || dy == rh - 1;

                    if is_border {
                        let bc = &self.theme.chars.border;
                        let ch = if dx == 0 && dy == 0 {
                            bc.top_left
                        } else if dx == rw - 1 && dy == 0 {
                            bc.top_right
                        } else if dx == 0 && dy == rh - 1 {
                            bc.bottom_left
                        } else if dx == rw - 1 && dy == rh - 1 {
                            bc.bottom_right
                        } else if dy == 0 || dy == rh - 1 {
                            bc.horizontal
                        } else {
                            bc.vertical
                        };
                        pb.set_char(x, y, ch, self.theme.axis_color, Z_CHROME);
                    } else {
                        pb.set_cell(
                            x,
                            y,
                            self.theme.chars.fill.solid,
                            r.color,
                            r.color,
                            Z_DATA,
                        );
                    }
                }
            }

            // Draw centered label if it fits
            if self.show_labels && rw >= 3 && rh >= 2 {
                let max_label_len = (rw as usize).saturating_sub(2);
                if max_label_len > 0 {
                    let truncated: String =
                        r.label.chars().take(max_label_len).collect();
                    let label_x =
                        rx + 1 + (max_label_len.saturating_sub(truncated.len()) as u16) / 2;
                    let label_y = ry + rh / 2;

                    if label_y >= plot_y && label_y < plot_y + plot_h {
                        for (j, ch) in truncated.chars().enumerate() {
                            let x = label_x + j as u16;
                            if x < rx + rw && x < area.x + area.width {
                                pb.set_char(
                                    x,
                                    label_y,
                                    ch,
                                    self.theme.foreground,
                                    Z_CHROME,
                                );
                            }
                        }
                    }
                }
            }
        }

        pb.composite(buf);
    }
}
