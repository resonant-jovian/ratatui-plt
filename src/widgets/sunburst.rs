//! Sunburst chart widget for hierarchical data.
//!
//! Renders hierarchical data as concentric rings, where each ring level shows
//! children of the previous level and angular extent is proportional to value.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::sunburst::{Sunburst, SunburstNode};
//! use ratatui::style::Color;
//!
//! let root = SunburstNode::new("root", 0.0)
//!     .child(SunburstNode::new("A", 5.0).color(Color::Cyan)
//!         .child(SunburstNode::new("A1", 3.0).color(Color::Blue))
//!         .child(SunburstNode::new("A2", 2.0).color(Color::LightBlue)))
//!     .child(SunburstNode::new("B", 4.0).color(Color::Yellow));
//! let chart = Sunburst::new(root).title("Hierarchy");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::color_cycle::ColorCycle;
use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA};
use crate::theme::Theme;

/// A node in the sunburst hierarchy.
#[derive(Clone, Debug)]
pub struct SunburstNode {
    /// Display label.
    pub label: String,
    /// Value (leaf weight). For branch nodes, the effective value is the sum of children.
    pub value: f64,
    /// Optional fill color.
    pub color: Option<Color>,
    /// Child nodes.
    pub children: Vec<SunburstNode>,
}

impl SunburstNode {
    /// Create a new node with the given label and value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
            children: Vec::new(),
        }
    }

    /// Set the node color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Add a child node.
    pub fn child(mut self, child: SunburstNode) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children.
    pub fn children(mut self, children: Vec<SunburstNode>) -> Self {
        self.children.extend(children);
        self
    }

    /// Effective value: if the node has children, sum of children's effective values;
    /// otherwise, the node's own value.
    pub fn effective_value(&self) -> f64 {
        if self.children.is_empty() {
            self.value.max(0.0)
        } else {
            self.children.iter().map(|c| c.effective_value()).sum()
        }
    }

    /// Maximum depth of the tree.
    pub fn max_depth(&self) -> usize {
        if self.children.is_empty() {
            1
        } else {
            1 + self
                .children
                .iter()
                .map(|c| c.max_depth())
                .max()
                .unwrap_or(0)
        }
    }
}

/// A sunburst chart widget.
///
/// Renders hierarchical data as concentric rings. The center represents
/// the root node, and each ring outward shows one level of the hierarchy.
pub struct Sunburst {
    root: SunburstNode,
    title: Option<String>,
    show_labels: bool,
    theme: Theme,
}

impl Sunburst {
    /// Create a new sunburst chart from a root node.
    pub fn new(root: SunburstNode) -> Self {
        Self {
            root,
            title: None,
            show_labels: true,
            theme: Theme::get_default(),
        }
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Whether to show labels on segments.
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

/// A flattened segment for rendering.
struct Segment {
    /// Ring level (0 = innermost/root children).
    level: usize,
    /// Start angle in radians.
    angle_start: f64,
    /// End angle in radians.
    angle_end: f64,
    /// Fill color.
    color: Color,
    /// Label text.
    label: String,
}

/// Recursively flatten the tree into renderable segments.
fn flatten_segments(
    node: &SunburstNode,
    level: usize,
    angle_start: f64,
    angle_end: f64,
    cycle: &mut ColorCycle,
    out: &mut Vec<Segment>,
) {
    if node.children.is_empty() {
        return;
    }

    let total = node.effective_value();
    if total <= 0.0 {
        return;
    }

    let mut cursor = angle_start;
    for child in &node.children {
        let child_val = child.effective_value();
        if child_val <= 0.0 {
            continue;
        }
        let fraction = child_val / total;
        let sweep = fraction * (angle_end - angle_start);
        let child_start = cursor;
        let child_end = cursor + sweep;

        let color = child.color.unwrap_or_else(|| cycle.next_color());

        out.push(Segment {
            level,
            angle_start: child_start,
            angle_end: child_end,
            color,
            label: child.label.clone(),
        });

        // Recurse into children
        flatten_segments(child, level + 1, child_start, child_end, cycle, out);

        cursor = child_end;
    }
}

impl Widget for &Sunburst {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 8 || area.height < 6 {
            return;
        }

        let root_val = self.root.effective_value();
        if root_val <= 0.0 {
            return;
        }

        // Reserve space for title
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let py = area.y + title_height;
        let ph = area.height.saturating_sub(title_height);

        if ph < 4 {
            return;
        }

        let mut pb = PlotBuffer::new(area);

        // Draw title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    pb.set_char(x, area.y, ch, self.theme.foreground, Z_CHROME);
                }
            }
        }

        // Flatten tree into segments
        let mut cycle = ColorCycle::default();
        let mut segments = Vec::new();
        let two_pi = 2.0 * std::f64::consts::PI;
        let start_angle = -std::f64::consts::FRAC_PI_2; // 12 o'clock
        flatten_segments(
            &self.root,
            0,
            start_angle,
            start_angle + two_pi,
            &mut cycle,
            &mut segments,
        );

        if segments.is_empty() {
            return;
        }

        // Determine number of ring levels
        let max_level = segments.iter().map(|s| s.level).max().unwrap_or(0);
        let num_rings = max_level + 1;

        // Center of the chart
        let cx = area.x as f64 + area.width as f64 / 2.0;
        let cy = py as f64 + ph as f64 / 2.0;

        // Radius calculation, accounting for terminal cell aspect ratio
        let cell_aspect = 0.5;
        let avail_x = (area.width / 2).saturating_sub(1) as f64;
        let avail_y = (ph / 2).saturating_sub(1) as f64;
        let max_r_data = avail_y.min(avail_x * cell_aspect);
        let r_screen_x = max_r_data / cell_aspect;
        let r_screen_y = max_r_data;

        // Inner radius for the center circle (root label area)
        let inner_frac = 0.2;
        let ring_width = (1.0 - inner_frac) / num_rings as f64;

        // Render segments by filling pixels
        let pw = area.width;
        for screen_y in py..py + ph {
            for screen_x in area.x..area.x + pw {
                let nx = (screen_x as f64 - cx) / r_screen_x;
                let ny = (screen_y as f64 - cy) / r_screen_y;
                let r = (nx * nx + ny * ny).sqrt();

                if r > 1.0 || r < inner_frac {
                    continue;
                }

                // Determine which ring level this radius falls in
                let ring_frac = (r - inner_frac) / (1.0 - inner_frac);
                let level = (ring_frac * num_rings as f64).floor() as usize;
                let level = level.min(num_rings - 1);

                // Compute angle
                let mut angle = ny.atan2(nx);
                // Normalize angle to [start_angle, start_angle + 2*PI)
                while angle < start_angle {
                    angle += two_pi;
                }
                while angle >= start_angle + two_pi {
                    angle -= two_pi;
                }

                // Find matching segment
                for seg in &segments {
                    if seg.level == level && angle >= seg.angle_start && angle < seg.angle_end {
                        pb.set_cell(screen_x, screen_y, '█', seg.color, seg.color, Z_DATA);
                        break;
                    }
                }
            }
        }

        // Draw root label in center
        if self.show_labels {
            let label = &self.root.label;
            let max_len = (inner_frac * r_screen_x * 2.0).floor() as usize;
            let truncated: String = label.chars().take(max_len).collect();
            let lx = cx - truncated.len() as f64 / 2.0;
            let ly = cy;
            for (j, ch) in truncated.chars().enumerate() {
                let x = lx.round() as u16 + j as u16;
                let y = ly.round() as u16;
                if x >= area.x && x < area.x + area.width && y >= py && y < py + ph {
                    pb.set_char(x, y, ch, self.theme.foreground, Z_CHROME);
                }
            }
        }

        // Draw segment labels on the mid-angle/mid-radius of each segment
        if self.show_labels {
            for seg in &segments {
                let mid_angle = (seg.angle_start + seg.angle_end) / 2.0;
                let r_inner = inner_frac + seg.level as f64 * ring_width;
                let r_mid = r_inner + ring_width / 2.0;

                let lx = cx + r_mid * r_screen_x * mid_angle.cos();
                let ly = cy + r_mid * r_screen_y * mid_angle.sin();

                // Only draw label if the segment is wide enough
                let arc_len = (seg.angle_end - seg.angle_start) * r_mid * r_screen_x;
                if arc_len < seg.label.len() as f64 * 1.2 {
                    continue;
                }

                let label_start_x = lx - seg.label.len() as f64 / 2.0;
                let label_y = ly.round() as u16;
                if label_y >= py && label_y < py + ph {
                    for (j, ch) in seg.label.chars().enumerate() {
                        let x = (label_start_x + j as f64).round() as u16;
                        if x >= area.x && x < area.x + area.width {
                            pb.set_char(x, label_y, ch, self.theme.foreground, Z_CHROME);
                        }
                    }
                }
            }
        }

        pb.composite(buf);
    }
}
