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

use crate::plot_buffer::{PlotBackend, Z_CHROME, Z_DATA, create_backend};
use crate::theme::Theme;

/// Flow direction for the Sankey diagram.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SankeyOrientation {
    /// Nodes arranged in columns, flows go left to right.
    #[default]
    Horizontal,
    /// Nodes arranged in rows, flows go top to bottom.
    Vertical,
}

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
    orientation: SankeyOrientation,
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
            orientation: SankeyOrientation::default(),
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

    /// Set the flow direction (horizontal or vertical).
    pub fn orientation(mut self, o: SankeyOrientation) -> Self {
        self.orientation = o;
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

impl SankeyDiagram {
    /// Derive a flow color from a source node color at ~65% brightness.
    fn flow_color_from_source(source_color: Color) -> Color {
        match source_color {
            Color::Rgb(r, g, b) => Color::Rgb(
                (r as f64 * 0.65) as u8,
                (g as f64 * 0.65) as u8,
                (b as f64 * 0.65) as u8,
            ),
            Color::Yellow => Color::Rgb(200, 200, 50),
            Color::Cyan => Color::Rgb(50, 200, 200),
            Color::Red | Color::LightRed => Color::Rgb(200, 60, 60),
            Color::Green | Color::LightGreen => Color::Rgb(60, 200, 60),
            Color::Blue | Color::LightBlue => Color::Rgb(60, 60, 200),
            Color::Magenta | Color::LightMagenta => Color::Rgb(200, 60, 200),
            other => other,
        }
    }

    /// Render in horizontal (left-to-right) orientation.
    #[allow(clippy::too_many_arguments)]
    fn render_horizontal(
        &self,
        area: Rect,
        py: u16,
        ph: u16,
        pb: &mut dyn PlotBackend,
        columns: &[usize],
        node_totals: &[f64],
        num_columns: usize,
        col_nodes: &[Vec<usize>],
    ) {
        // Compute label margins
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

        // Evenly space columns across the width
        let usable_width = area.width.saturating_sub(left_margin + right_margin);
        let col_spacing = if num_columns > 1 {
            (usable_width.saturating_sub(self.node_width * num_columns as u16))
                / (num_columns as u16 - 1).max(1)
        } else {
            0
        };
        let col_step = self.node_width + col_spacing;

        let col_x: Vec<u16> = (0..num_columns)
            .map(|c| area.x + left_margin + c as u16 * col_step)
            .collect();

        // Vertical layout for each column
        let mut node_y: Vec<f64> = vec![0.0; self.nodes.len()];
        let mut node_h: Vec<f64> = vec![0.0; self.nodes.len()];

        for nodes_in_col in col_nodes.iter().take(num_columns) {
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

        // Draw flows (behind nodes)
        let mut source_offsets = vec![0.0f64; self.nodes.len()];
        let mut target_offsets = vec![0.0f64; self.nodes.len()];

        for flow in &self.flows {
            if flow.source >= self.nodes.len() || flow.target >= self.nodes.len() {
                continue;
            }

            let source_color = self.nodes[flow.source]
                .color
                .unwrap_or_else(|| self.theme.color_cycle.at(flow.source));
            let flow_color = flow
                .color
                .unwrap_or_else(|| Self::flow_color_from_source(source_color));

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

            if tx > sx {
                let band_width = tx - sx;
                for dx in 0..band_width {
                    let raw_frac = dx as f64 / band_width as f64;
                    let frac = 3.0 * raw_frac * raw_frac - 2.0 * raw_frac * raw_frac * raw_frac;
                    let x = sx + dx;

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

            // Label placement
            let label = &node.label;
            let label_x = if col == num_columns - 1 {
                nx + self.node_width + 1
            } else if col == 0 && nx > area.x + label.len() as u16 {
                nx.saturating_sub(label.len() as u16 + 1)
            } else {
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
    }

    /// Render in vertical (top-to-bottom) orientation.
    ///
    /// Nodes are horizontal bars arranged in rows. Flows curve
    /// downward between source bottom-edge and target top-edge.
    #[allow(clippy::too_many_arguments)]
    fn render_vertical(
        &self,
        area: Rect,
        py: u16,
        ph: u16,
        pb: &mut dyn PlotBackend,
        columns: &[usize],
        node_totals: &[f64],
        num_rows: usize,
        row_nodes: &[Vec<usize>],
    ) {
        // Reserve 1 row above/below for labels
        let label_margin: u16 = 1;
        let top_margin = label_margin;
        let bot_margin = label_margin;
        let usable_h = ph.saturating_sub(top_margin + bot_margin);
        if usable_h < num_rows as u16 {
            return;
        }

        // Evenly space rows vertically
        let row_spacing = if num_rows > 1 {
            (usable_h.saturating_sub(self.node_width * num_rows as u16))
                / (num_rows as u16 - 1).max(1)
        } else {
            0
        };
        let row_step = self.node_width + row_spacing;

        let row_y: Vec<u16> = (0..num_rows)
            .map(|r| py + top_margin + r as u16 * row_step)
            .collect();

        // Horizontal layout for each row: stack nodes proportional
        // to total flow across the available width.
        let mut node_x: Vec<f64> = vec![0.0; self.nodes.len()];
        let mut node_w: Vec<f64> = vec![0.0; self.nodes.len()];

        for nodes_in_row in row_nodes.iter().take(num_rows) {
            if nodes_in_row.is_empty() {
                continue;
            }

            let total_flow: f64 = nodes_in_row.iter().map(|&i| node_totals[i]).sum();
            let total_padding =
                self.node_padding as f64 * (nodes_in_row.len() as f64 - 1.0).max(0.0);
            let available_w = (area.width as f64 - total_padding).max(1.0);

            let mut x_cursor = area.x as f64;
            for &ni in nodes_in_row {
                let fraction = node_totals[ni] / total_flow;
                let w = (fraction * available_w).max(1.0);
                node_x[ni] = x_cursor;
                node_w[ni] = w;
                x_cursor += w + self.node_padding as f64;
            }
        }

        // Draw flows (behind nodes)
        let mut source_offsets = vec![0.0f64; self.nodes.len()];
        let mut target_offsets = vec![0.0f64; self.nodes.len()];

        for flow in &self.flows {
            if flow.source >= self.nodes.len() || flow.target >= self.nodes.len() {
                continue;
            }

            let source_color = self.nodes[flow.source]
                .color
                .unwrap_or_else(|| self.theme.color_cycle.at(flow.source));
            let flow_color = flow
                .color
                .unwrap_or_else(|| Self::flow_color_from_source(source_color));

            // Source bottom-edge y, target top-edge y
            let sy = row_y[columns[flow.source]] + self.node_width;
            let ty = row_y[columns[flow.target]];

            // Source horizontal band
            let s_frac = flow.value / node_totals[flow.source];
            let s_w = s_frac * node_w[flow.source];
            let s_left = node_x[flow.source] + source_offsets[flow.source];
            source_offsets[flow.source] += s_w;

            // Target horizontal band
            let t_frac = flow.value / node_totals[flow.target];
            let t_w = t_frac * node_w[flow.target];
            let t_left = node_x[flow.target] + target_offsets[flow.target];
            target_offsets[flow.target] += t_w;

            // Draw vertical flow band with cubic easing
            if ty > sy {
                let band_height = ty - sy;
                for dy in 0..band_height {
                    let raw_frac = dy as f64 / band_height as f64;
                    let frac = 3.0 * raw_frac * raw_frac - 2.0 * raw_frac * raw_frac * raw_frac;
                    let y = sy + dy;

                    // Interpolate left and right edges
                    let left = s_left + (t_left - s_left) * frac;
                    let right = (s_left + s_w) + ((t_left + t_w) - (s_left + s_w)) * frac;

                    let x_first = left.floor().max(area.x as f64) as u16;
                    let x_last = right.floor().min((area.x + area.width) as f64 - 1.0) as u16;

                    if y < py || y >= py + ph {
                        continue;
                    }

                    for x in x_first..=x_last {
                        if x >= area.x + area.width {
                            continue;
                        }
                        // Use half-block characters for sub-cell
                        // horizontal edge smoothing: at left and
                        // right boundaries use partial fills.
                        if x == x_first && x == x_last {
                            pb.set_char(x, y, self.theme.chars.fill.solid, flow_color, Z_DATA);
                        } else if x == x_first {
                            let left_frac = left - x as f64;
                            if left_frac > 0.5 {
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
                        } else if x == x_last {
                            let right_frac = right - x as f64;
                            if right_frac < 0.5 {
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

        // Draw node boxes (horizontal bars)
        for (i, node) in self.nodes.iter().enumerate() {
            let row = columns[i];
            let ny = row_y[row];
            let nx = node_x[i].round() as u16;
            let nw = node_w[i].round().max(1.0) as u16;
            let node_color = node.color.unwrap_or_else(|| self.theme.color_cycle.at(i));

            // Draw filled horizontal bar
            for dy in 0..self.node_width {
                for dx in 0..nw {
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

            // Label above first row, below last row, else above
            let label = &node.label;
            let label_len = label.len() as u16;
            let label_x = nx + nw / 2 - label_len.min(nw) / 2;
            let label_y = if row == 0 {
                ny.saturating_sub(1)
            } else if row == num_rows - 1 {
                ny + self.node_width
            } else {
                ny.saturating_sub(1)
            };

            if label_y >= area.y && label_y < area.y + area.height {
                for (j, ch) in label.chars().enumerate() {
                    let x = label_x + j as u16;
                    if x < area.x + area.width {
                        pb.set_char(x, label_y, ch, self.theme.foreground, Z_CHROME);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for SankeyDiagram {
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

impl Widget for &SankeyDiagram {
    fn render(self, area: Rect, buf: &mut Buffer) {
        #[cfg(feature = "plotters-render")]
        {
            if crate::plotters_render::should_use_plotters() {
                use crate::plotters_render::PlottersRenderable;
                self.render_plotters(area, buf, &self.theme);
                return;
            }
        }
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

        let columns = self.assign_columns();
        let node_totals = self.node_totals();

        let num_layers = columns.iter().copied().max().unwrap_or(0) + 1;
        if num_layers == 0 {
            return;
        }

        // Gather nodes per layer (column for horiz, row for vert)
        let mut layer_nodes: Vec<Vec<usize>> = vec![Vec::new(); num_layers];
        for (i, &col) in columns.iter().enumerate() {
            layer_nodes[col].push(i);
        }

        match self.orientation {
            SankeyOrientation::Horizontal => {
                self.render_horizontal(
                    area,
                    py,
                    ph,
                    &mut *pb,
                    &columns,
                    &node_totals,
                    num_layers,
                    &layer_nodes,
                );
            }
            SankeyOrientation::Vertical => {
                self.render_vertical(
                    area,
                    py,
                    ph,
                    &mut *pb,
                    &columns,
                    &node_totals,
                    num_layers,
                    &layer_nodes,
                );
            }
        }

        pb.composite(buf);
    }
}
