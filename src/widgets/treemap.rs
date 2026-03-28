//! Treemap widget for hierarchical data visualization.
//!
//! Uses the squarified treemap algorithm (Bruls et al.) to divide a rectangle
//! into nested sub-rectangles proportional to node values, with aspect ratios
//! optimized to be as square as possible.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::treemap::{Treemap, TreemapNode};
//! use ratatui::style::Color;
//!
//! let root = TreemapNode::new("root", 0.0)
//!     .child(TreemapNode::new("A", 6.0).color(Color::Cyan))
//!     .child(TreemapNode::new("B", 4.0).color(Color::Yellow))
//!     .child(TreemapNode::new("C", 3.0).color(Color::Green));
//! let chart = Treemap::new(root).title("Disk Usage");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::color_cycle::ColorCycle;
use crate::plot_buffer::{PlotBackend, Z_CHROME, Z_DATA, create_backend};
use crate::theme::Theme;

/// A node in the treemap hierarchy.
#[derive(Clone, Debug)]
pub struct TreemapNode {
    /// Display label.
    pub label: String,
    /// Value (leaf weight). For branch nodes, the effective value is the sum of children.
    pub value: f64,
    /// Optional fill color. If `None`, a color from the theme cycle is assigned.
    pub color: Option<Color>,
    /// Child nodes.
    pub children: Vec<TreemapNode>,
}

impl TreemapNode {
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
    pub fn child(mut self, child: TreemapNode) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children.
    pub fn children(mut self, children: Vec<TreemapNode>) -> Self {
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
}

/// A treemap chart widget.
///
/// Renders hierarchical data as nested rectangles with areas proportional to
/// values. Uses the squarified treemap algorithm for optimal aspect ratios.
pub struct Treemap {
    root: TreemapNode,
    title: Option<String>,
    show_labels: bool,
    theme: Theme,
}

impl Treemap {
    /// Create a new treemap from a root node.
    pub fn new(root: TreemapNode) -> Self {
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

/// A laid-out rectangle for rendering.
struct LayoutRect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    label: String,
    color: Color,
}

/// Squarified treemap layout: partition `rect` among `items` sorted by descending value.
fn squarify_layout(
    items: &[(f64, String, Color)],
    rect_x: u16,
    rect_y: u16,
    rect_w: u16,
    rect_h: u16,
    out: &mut Vec<LayoutRect>,
) {
    if items.is_empty() || rect_w == 0 || rect_h == 0 {
        return;
    }

    if items.len() == 1 {
        out.push(LayoutRect {
            x: rect_x,
            y: rect_y,
            w: rect_w,
            h: rect_h,
            label: items[0].1.clone(),
            color: items[0].2,
        });
        return;
    }

    // Sort items by value descending
    let mut sorted: Vec<(usize, f64)> = items
        .iter()
        .enumerate()
        .map(|(i, &(v, _, _))| (i, v))
        .collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let total: f64 = sorted.iter().map(|(_, v)| v).sum();
    if total <= 0.0 {
        return;
    }

    // Lay out along the shorter side
    let lay_horizontal = rect_w >= rect_h;
    let short_side = if lay_horizontal { rect_h } else { rect_w } as f64;

    // Greedily add items to the current row/column, optimizing aspect ratio
    let mut row: Vec<usize> = Vec::new();
    let mut row_sum = 0.0;
    let remaining = total;
    let mut best_worst_ratio = f64::MAX;

    let mut split_at = sorted.len(); // default: everything in one strip

    for (idx, &(orig_idx, val)) in sorted.iter().enumerate() {
        row.push(orig_idx);
        row_sum += val;

        // Compute worst aspect ratio if we commit this row
        let row_frac = row_sum / total;
        let strip_size = row_frac * short_side;
        if strip_size <= 0.0 {
            continue;
        }

        let mut worst_ratio = 0.0f64;
        for &ri in &row {
            let item_frac = items[ri].0 / row_sum;
            let item_long = item_frac
                * (if lay_horizontal { rect_w } else { rect_h }) as f64
                * (row_sum / total);
            // Correct: item gets proportional share of the strip
            let item_size = items[ri].0 / total
                * (short_side * (if lay_horizontal { rect_w } else { rect_h }) as f64);
            let item_w_f;
            let item_h_f;
            if lay_horizontal {
                item_h_f = strip_size;
                item_w_f = if strip_size > 0.0 {
                    (items[ri].0 / row_sum) * rect_w as f64
                } else {
                    0.0
                };
            } else {
                item_w_f = strip_size;
                item_h_f = if strip_size > 0.0 {
                    (items[ri].0 / row_sum) * rect_h as f64
                } else {
                    0.0
                };
            }
            if item_w_f > 0.0 && item_h_f > 0.0 {
                let ratio = (item_w_f / item_h_f).max(item_h_f / item_w_f);
                worst_ratio = worst_ratio.max(ratio);
            }
            let _ = item_long;
            let _ = item_size;
        }

        if worst_ratio <= best_worst_ratio {
            best_worst_ratio = worst_ratio;
        } else {
            // Adding this item made it worse; split before it
            row.pop();
            let _ = val;
            split_at = idx;
            break;
        }

        let _ = remaining;
    }

    if row.is_empty() {
        return;
    }

    // Lay out the row
    let row_total: f64 = row.iter().map(|&i| items[i].0).sum();
    let row_frac = row_total / total;

    let (strip_x, strip_y, strip_w, strip_h, rem_x, rem_y, rem_w, rem_h);
    if lay_horizontal {
        let strip_h_px = (row_frac * rect_h as f64).round().max(1.0) as u16;
        let strip_h_px = strip_h_px.min(rect_h);
        strip_x = rect_x;
        strip_y = rect_y;
        strip_w = rect_w;
        strip_h = strip_h_px;
        rem_x = rect_x;
        rem_y = rect_y + strip_h_px;
        rem_w = rect_w;
        rem_h = rect_h.saturating_sub(strip_h_px);
    } else {
        let strip_w_px = (row_frac * rect_w as f64).round().max(1.0) as u16;
        let strip_w_px = strip_w_px.min(rect_w);
        strip_x = rect_x;
        strip_y = rect_y;
        strip_w = strip_w_px;
        strip_h = rect_h;
        rem_x = rect_x + strip_w_px;
        rem_y = rect_y;
        rem_w = rect_w.saturating_sub(strip_w_px);
        rem_h = rect_h;
    }

    // Place items within the strip
    let mut cursor = 0u16;
    for (idx, &ri) in row.iter().enumerate() {
        let item_frac = items[ri].0 / row_total;
        let is_last = idx == row.len() - 1;

        let (ix, iy, iw, ih);
        if lay_horizontal {
            ix = strip_x + cursor;
            iy = strip_y;
            iw = if is_last {
                strip_w.saturating_sub(cursor)
            } else {
                (item_frac * strip_w as f64).round().max(1.0) as u16
            };
            ih = strip_h;
            cursor += iw;
        } else {
            ix = strip_x;
            iy = strip_y + cursor;
            iw = strip_w;
            ih = if is_last {
                strip_h.saturating_sub(cursor)
            } else {
                (item_frac * strip_h as f64).round().max(1.0) as u16
            };
            cursor += ih;
        }

        if iw > 0 && ih > 0 {
            out.push(LayoutRect {
                x: ix,
                y: iy,
                w: iw,
                h: ih,
                label: items[ri].1.clone(),
                color: items[ri].2,
            });
        }
    }

    // Recurse for remaining items
    if split_at < sorted.len() && rem_w > 0 && rem_h > 0 {
        let remaining_items: Vec<(f64, String, Color)> = sorted[split_at..]
            .iter()
            .map(|&(orig_idx, _)| items[orig_idx].clone())
            .collect();
        squarify_layout(&remaining_items, rem_x, rem_y, rem_w, rem_h, out);
    }
}

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for Treemap {
    fn render_plotters(
        &self,
        area: ratatui::layout::Rect,
        buf: &mut ratatui::buffer::Buffer,
        theme: &crate::theme::Theme,
    ) {
        use crate::plotters_render::{bridge, theme_bridge};
        bridge::render_plotters_to_buf(
            area, buf, theme_bridge::theme_bg_rgb(theme),
            |_root| { /* Minimal stub - full plotters rendering TBD */ },
        );
    }
}

impl Widget for &Treemap {
    fn render(self, area: Rect, buf: &mut Buffer) {
        #[cfg(feature = "plotters-render")]
        {
            if crate::plotters_render::should_use_plotters() {
                use crate::plotters_render::PlottersRenderable;
                self.render_plotters(area, buf, &self.theme);
                return;
            }
        }
        if area.width < 4 || area.height < 3 {
            return;
        }

        // Reserve space for title
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let py = area.y + title_height;
        let ph = area.height.saturating_sub(title_height);

        if ph < 2 {
            return;
        }

        let mut pb = create_backend(area);

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

        // Flatten children (or use root as single item) with color assignment
        let mut cycle = ColorCycle::default();
        let items: Vec<(f64, String, Color)> = if self.root.children.is_empty() {
            let c = self.root.color.unwrap_or_else(|| cycle.next_color());
            vec![(self.root.effective_value(), self.root.label.clone(), c)]
        } else {
            self.root
                .children
                .iter()
                .map(|child| {
                    let c = child.color.unwrap_or_else(|| cycle.next_color());
                    (child.effective_value(), child.label.clone(), c)
                })
                .filter(|(v, _, _)| *v > 0.0)
                .collect()
        };

        if items.is_empty() {
            return;
        }

        let mut rects = Vec::new();
        squarify_layout(&items, area.x, py, area.width, ph, &mut rects);

        // Render each rectangle
        for r in &rects {
            // Draw border using box-drawing characters
            for dx in 0..r.w {
                for dy in 0..r.h {
                    let x = r.x + dx;
                    let y = r.y + dy;
                    if x >= area.x + area.width || y >= py + ph {
                        continue;
                    }

                    let bc = &self.theme.chars.border;
                    let ch = if dx == 0 && dy == 0 {
                        bc.top_left
                    } else if dx == r.w - 1 && dy == 0 {
                        bc.top_right
                    } else if dx == 0 && dy == r.h - 1 {
                        bc.bottom_left
                    } else if dx == r.w - 1 && dy == r.h - 1 {
                        bc.bottom_right
                    } else if dy == 0 || dy == r.h - 1 {
                        bc.horizontal
                    } else if dx == 0 || dx == r.w - 1 {
                        bc.vertical
                    } else {
                        self.theme.chars.fill.solid
                    };

                    let color = if dx == 0 || dx == r.w - 1 || dy == 0 || dy == r.h - 1 {
                        self.theme.axis_color
                    } else {
                        r.color
                    };

                    let z = if dx == 0 || dx == r.w - 1 || dy == 0 || dy == r.h - 1 {
                        Z_CHROME
                    } else {
                        Z_DATA
                    };
                    if ch == self.theme.chars.fill.solid {
                        pb.set_cell(x, y, ch, color, color, z);
                    } else {
                        pb.set_char(x, y, ch, color, z);
                    }
                }
            }

            // Draw centered label
            if self.show_labels && r.w >= 3 && r.h >= 2 {
                let label = &r.label;
                let max_label_len = (r.w as usize).saturating_sub(2);
                let truncated: String = label.chars().take(max_label_len).collect();
                let label_x = r.x + 1 + (max_label_len.saturating_sub(truncated.len()) as u16) / 2;
                let label_y = r.y + r.h / 2;

                if label_y >= py && label_y < py + ph {
                    for (j, ch) in truncated.chars().enumerate() {
                        let x = label_x + j as u16;
                        if x < r.x + r.w && x < area.x + area.width {
                            pb.set_char(x, label_y, ch, self.theme.foreground, Z_CHROME);
                        }
                    }
                }
            }
        }

        pb.composite(buf);
    }
}
