//! Sankey diagram widget for flow visualization.
//!
//! Displays flows between nodes arranged in columns, with band widths
//! proportional to flow values. Useful for energy budgets, material flows,
//! and process diagrams.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::sankey::{SankeyDiagram, SankeyNode, SankeyFlow};
//! use ratatui::style::Color;
//!
//! let diagram = SankeyDiagram::new()
//!     .node(SankeyNode::new("A").color(Color::Cyan))
//!     .node(SankeyNode::new("B").color(Color::Yellow))
//!     .node(SankeyNode::new("C").color(Color::Green))
//!     .flow(SankeyFlow::new(0, 1, 5.0))
//!     .flow(SankeyFlow::new(0, 2, 3.0))
//!     .title("Energy Flow");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA};
use crate::theme::Theme;

/// A node in the Sankey diagram.
#[derive(Clone, Debug)]
pub struct SankeyNode {
    /// Display label for the node.
    pub label: String,
    /// Fill color for the node box.
    pub color: Option<Color>,
}

impl SankeyNode {
    /// Create a new node with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            color: None,
        }
    }

    /// Set the node color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A flow (edge) between two nodes in the Sankey diagram.
#[derive(Clone, Debug)]
pub struct SankeyFlow {
    /// Index of the source node.
    pub source: usize,
    /// Index of the target node.
    pub target: usize,
    /// Flow magnitude (determines band width).
    pub value: f64,
    /// Optional color override for the flow band.
    pub color: Option<Color>,
}

impl SankeyFlow {
    /// Create a new flow from source to target with the given value.
    pub fn new(source: usize, target: usize, value: f64) -> Self {
        Self {
            source,
            target,
            value,
            color: None,
        }
    }

    /// Set the flow color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A Sankey diagram widget.
///
/// Renders nodes as filled boxes arranged in columns and flows as density
/// bands between them. Column positions are assigned via topological sort,
/// and nodes within each column are stacked proportional to total flow.
pub struct SankeyDiagram {
    nodes: Vec<SankeyNode>,
    flows: Vec<SankeyFlow>,
    title: Option<String>,
    node_width: u16,
    node_padding: u16,
    theme: Theme,
}

impl Default for SankeyDiagram {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            flows: Vec::new(),
            title: None,
            node_width: 3,
            node_padding: 1,
            theme: Theme::get_default(),
        }
    }
}

impl SankeyDiagram {
    /// Create a new empty Sankey diagram.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node.
    pub fn node(mut self, node: SankeyNode) -> Self {
        self.nodes.push(node);
        self
    }

    /// Add multiple nodes at once.
    pub fn nodes(mut self, nodes: Vec<SankeyNode>) -> Self {
        self.nodes.extend(nodes);
        self
    }

    /// Add a flow.
    pub fn flow(mut self, flow: SankeyFlow) -> Self {
        self.flows.push(flow);
        self
    }

    /// Add multiple flows at once.
    pub fn flows(mut self, flows: Vec<SankeyFlow>) -> Self {
        self.flows.extend(flows);
        self
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the width of node boxes in characters.
    pub fn node_width(mut self, w: u16) -> Self {
        self.node_width = w.max(1);
        self
    }

    /// Set the vertical padding between nodes in characters.
    pub fn node_padding(mut self, p: u16) -> Self {
        self.node_padding = p;
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Topological sort to assign column indices to nodes.
    /// Returns a vec where result[node_index] = column_index.
    fn assign_columns(&self) -> Vec<usize> {
        let n = self.nodes.len();
        if n == 0 {
            return Vec::new();
        }

        // Compute longest path from sources using BFS-like approach
        let mut columns = vec![0usize; n];
        let mut changed = true;
        // Iterate until stable (handles DAG relaxation)
        let mut iters = 0;
        while changed && iters < n {
            changed = false;
            iters += 1;
            for flow in &self.flows {
                if flow.source < n && flow.target < n {
                    let new_col = columns[flow.source] + 1;
                    if new_col > columns[flow.target] {
                        columns[flow.target] = new_col;
                        changed = true;
                    }
                }
            }
        }

        columns
    }

    /// Compute total flow through each node (max of in-flow, out-flow).
    fn node_totals(&self) -> Vec<f64> {
        let n = self.nodes.len();
        let mut in_flow = vec![0.0f64; n];
        let mut out_flow = vec![0.0f64; n];

        for flow in &self.flows {
            if flow.source < n && flow.target < n {
                out_flow[flow.source] += flow.value;
                in_flow[flow.target] += flow.value;
            }
        }

        (0..n)
            .map(|i| in_flow[i].max(out_flow[i]).max(0.001))
            .collect()
    }
}

impl Widget for &SankeyDiagram {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 4 || self.nodes.is_empty() {
            return;
        }

        // Reserve space for title
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let py = area.y + title_height;
        let ph = area.height.saturating_sub(title_height);

        if ph < 3 {
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

        let columns = self.assign_columns();
        let node_totals = self.node_totals();

        let num_columns = columns.iter().copied().max().unwrap_or(0) + 1;
        if num_columns == 0 {
            return;
        }

        // Gather nodes per column
        let mut col_nodes: Vec<Vec<usize>> = vec![Vec::new(); num_columns];
        for (i, &col) in columns.iter().enumerate() {
            col_nodes[col].push(i);
        }

        // Compute label margins: reserve space for left-column and right-column labels
        let left_label_width = col_nodes[0]
            .iter()
            .map(|&i| self.nodes[i].label.len() as u16 + 1)
            .max()
            .unwrap_or(1);
        let right_label_width = col_nodes[num_columns - 1]
            .iter()
            .map(|&i| self.nodes[i].label.len() as u16 + 1)
            .max()
            .unwrap_or(1);
        let left_margin = left_label_width.min(area.width / 4);
        let right_margin = right_label_width.min(area.width / 4);

        // Compute horizontal layout: evenly space columns across the width
        let usable_width = area.width.saturating_sub(left_margin + right_margin);
        let col_spacing = if num_columns > 1 {
            (usable_width.saturating_sub(self.node_width * num_columns as u16))
                / (num_columns as u16 - 1).max(1)
        } else {
            0
        };
        let col_step = self.node_width + col_spacing;

        // Compute column x positions
        let col_x: Vec<u16> = (0..num_columns)
            .map(|c| area.x + left_margin + c as u16 * col_step)
            .collect();

        // Compute vertical layout for each column
        // Total flow in each column determines vertical scaling
        let mut node_y: Vec<f64> = vec![0.0; self.nodes.len()];
        let mut node_h: Vec<f64> = vec![0.0; self.nodes.len()];

        for (_col, nodes_in_col) in col_nodes.iter().enumerate().take(num_columns) {
            if nodes_in_col.is_empty() {
                continue;
            }

            let total_flow: f64 = nodes_in_col.iter().map(|&i| node_totals[i]).sum();
            let total_padding =
                self.node_padding as f64 * (nodes_in_col.len() as f64 - 1.0).max(0.0);
            let available_h = (ph as f64 - total_padding).max(1.0);

            let mut y_cursor = py as f64;
            for &ni in nodes_in_col {
                let fraction = node_totals[ni] / total_flow;
                let h = (fraction * available_h).max(1.0);
                node_y[ni] = y_cursor;
                node_h[ni] = h;
                y_cursor += h + self.node_padding as f64;
            }
        }

        // Draw flows first (behind nodes)
        // Track how much of each node's vertical space has been used for flows
        let mut source_offsets = vec![0.0f64; self.nodes.len()];
        let mut target_offsets = vec![0.0f64; self.nodes.len()];

        for flow in &self.flows {
            if flow.source >= self.nodes.len() || flow.target >= self.nodes.len() {
                continue;
            }

            let source_color = self.nodes[flow.source]
                .color
                .unwrap_or_else(|| self.theme.color_cycle.at(flow.source));
            let flow_color = flow.color.unwrap_or({
                // Derive a saturated flow color from the source node color at ~65% brightness.
                // This avoids near-white flow bands that lack contrast against the background.
                match source_color {
                    Color::Rgb(r, g, b) => {
                        // Scale to ~65% brightness for better visibility
                        Color::Rgb(
                            (r as f64 * 0.65) as u8,
                            (g as f64 * 0.65) as u8,
                            (b as f64 * 0.65) as u8,
                        )
                    }
                    Color::Yellow => Color::Rgb(200, 200, 50),
                    Color::Cyan => Color::Rgb(50, 200, 200),
                    Color::Red | Color::LightRed => Color::Rgb(200, 60, 60),
                    Color::Green | Color::LightGreen => Color::Rgb(60, 200, 60),
                    Color::Blue | Color::LightBlue => Color::Rgb(60, 60, 200),
                    Color::Magenta | Color::LightMagenta => Color::Rgb(200, 60, 200),
                    other => other,
                }
            });

            // Source and target screen coordinates
            let sx = col_x[columns[flow.source]] + self.node_width;
            let tx = col_x[columns[flow.target]];

            let s_frac = flow.value / node_totals[flow.source];
            let s_h = s_frac * node_h[flow.source];
            let s_top = node_y[flow.source] + source_offsets[flow.source];
            source_offsets[flow.source] += s_h;

            let t_frac = flow.value / node_totals[flow.target];
            let t_h = t_frac * node_h[flow.target];
            let t_top = node_y[flow.target] + target_offsets[flow.target];
            target_offsets[flow.target] += t_h;

            // Draw flow band using half-block characters for smooth edges
            if tx > sx {
                let band_width = tx - sx;
                for dx in 0..band_width {
                    // Use smooth cubic interpolation (ease in-out) for the band path
                    let raw_frac = dx as f64 / band_width as f64;
                    let frac = 3.0 * raw_frac * raw_frac - 2.0 * raw_frac * raw_frac * raw_frac;
                    let x = sx + dx;

                    // Interpolate top and bottom edges
                    let top = s_top + (t_top - s_top) * frac;
                    let bot = (s_top + s_h) + ((t_top + t_h) - (s_top + s_h)) * frac;

                    let y_first = top.floor() as u16;
                    let y_last = bot.floor() as u16;

                    for y in y_first..=y_last {
                        if x >= area.x + area.width || y < py || y >= py + ph {
                            continue;
                        }
                        if y == y_first && y == y_last {
                            let top_half = top - y as f64;
                            if top_half > 0.5 {
                                pb.set_char(
                                    x,
                                    y,
                                    self.theme.chars.fill.half_lower,
                                    flow_color,
                                    Z_DATA,
                                );
                            } else {
                                pb.set_char(
                                    x,
                                    y,
                                    self.theme.chars.fill.half_upper,
                                    flow_color,
                                    Z_DATA,
                                );
                            }
                        } else if y == y_first {
                            let top_frac = top - y as f64;
                            if top_frac > 0.5 {
                                pb.set_char(
                                    x,
                                    y,
                                    self.theme.chars.fill.half_lower,
                                    flow_color,
                                    Z_DATA,
                                );
                            } else {
                                pb.set_cell(
                                    x,
                                    y,
                                    self.theme.chars.fill.solid,
                                    flow_color,
                                    flow_color,
                                    Z_DATA,
                                );
                            }
                        } else if y == y_last {
                            let bot_frac = bot - y as f64;
                            if bot_frac < 0.5 {
                                pb.set_char(
                                    x,
                                    y,
                                    self.theme.chars.fill.half_upper,
                                    flow_color,
                                    Z_DATA,
                                );
                            } else {
                                pb.set_cell(
                                    x,
                                    y,
                                    self.theme.chars.fill.solid,
                                    flow_color,
                                    flow_color,
                                    Z_DATA,
                                );
                            }
                        } else {
                            pb.set_cell(
                                x,
                                y,
                                self.theme.chars.fill.solid,
                                flow_color,
                                flow_color,
                                Z_DATA,
                            );
                        }
                    }
                }
            }
        }

        // Draw node boxes
        for (i, node) in self.nodes.iter().enumerate() {
            let col = columns[i];
            let nx = col_x[col];
            let ny = node_y[i].round() as u16;
            let nh = node_h[i].round().max(1.0) as u16;
            let node_color = node.color.unwrap_or_else(|| self.theme.color_cycle.at(i));

            // Draw filled box
            for dy in 0..nh {
                for dx in 0..self.node_width {
                    let x = nx + dx;
                    let y = ny + dy;
                    if x < area.x + area.width && y >= py && y < py + ph {
                        pb.set_cell(
                            x,
                            y,
                            self.theme.chars.fill.solid,
                            node_color,
                            node_color,
                            Z_DATA,
                        );
                    }
                }
            }

            // Draw label to the right of the last column, left of the first, or below
            let label = &node.label;
            let label_x = if col == num_columns - 1 {
                // Right of node
                nx + self.node_width + 1
            } else if col == 0 && nx > area.x + label.len() as u16 {
                // Left of node
                nx.saturating_sub(label.len() as u16 + 1)
            } else {
                // Right of node
                nx + self.node_width + 1
            };

            let label_y = ny + nh / 2;
            if label_y >= py && label_y < py + ph {
                for (j, ch) in label.chars().enumerate() {
                    let x = label_x + j as u16;
                    if x < area.x + area.width {
                        pb.set_char(x, label_y, ch, self.theme.foreground, Z_CHROME);
                    }
                }
            }
        }

        pb.composite(buf);
    }
}
