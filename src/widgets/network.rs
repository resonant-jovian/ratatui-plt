//! Network (graph) plot widget for node-link diagrams.
//!
//! Visualizes graphs with nodes and edges using force-directed, circular,
//! or manual layout algorithms. Uses `PlotFrame` for axis rendering.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::network::{NetworkPlot, GraphNode, GraphEdge, GraphLayout};
//! use ratatui::style::Color;
//!
//! let plot = NetworkPlot::new()
//!     .node(GraphNode::new("A").color(Color::Cyan))
//!     .node(GraphNode::new("B").color(Color::Yellow))
//!     .node(GraphNode::new("C").color(Color::Green))
//!     .edge(GraphEdge::new(0, 1, 1.0))
//!     .edge(GraphEdge::new(1, 2, 1.0))
//!     .edge(GraphEdge::new(2, 0, 1.0))
//!     .layout(GraphLayout::ForceDirected)
//!     .title("Social Network");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::{AspectRatio, Axis};
use crate::drawing::draw_braille_line;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::spines::Spines;
use crate::style::MarkerShape;
use crate::theme::Theme;

/// A node in the graph.
#[derive(Clone, Debug)]
pub struct GraphNode {
    /// Display label.
    pub label: String,
    /// Node color.
    pub color: Color,
    /// Optional fixed position (x, y) in data coordinates.
    pub position: Option<(f64, f64)>,
    /// Marker shape for the node.
    pub marker: MarkerShape,
}

impl GraphNode {
    /// Create a new node with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            color: Color::White,
            position: None,
            marker: MarkerShape::FilledCircle,
        }
    }

    /// Set the node color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set a fixed position for the node.
    pub fn position(mut self, x: f64, y: f64) -> Self {
        self.position = Some((x, y));
        self
    }

    /// Set the marker shape.
    pub fn marker(mut self, marker: MarkerShape) -> Self {
        self.marker = marker;
        self
    }
}

/// An edge between two nodes.
#[derive(Clone, Debug)]
pub struct GraphEdge {
    /// Index of the source node.
    pub source: usize,
    /// Index of the target node.
    pub target: usize,
    /// Edge weight (affects force layout attraction strength).
    pub weight: f64,
    /// Optional color override for the edge.
    pub color: Option<Color>,
}

impl GraphEdge {
    /// Create a new edge from source to target with the given weight.
    pub fn new(source: usize, target: usize, weight: f64) -> Self {
        Self {
            source,
            target,
            weight,
            color: None,
        }
    }

    /// Set the edge color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// Layout algorithm for arranging graph nodes.
#[derive(Clone, Debug, Default)]
pub enum GraphLayout {
    /// Fruchterman-Reingold force-directed layout.
    #[default]
    ForceDirected,
    /// Nodes evenly spaced on a circle.
    Circular,
    /// Use positions specified on each node.
    Manual,
}

/// A network (graph) plot widget.
///
/// Renders nodes as labeled markers and edges as lines between them.
/// Supports force-directed, circular, and manual layout algorithms.
pub struct NetworkPlot {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    layout: GraphLayout,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    aspect_ratio: AspectRatio,
    show_labels: bool,
    iterations: usize,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
}

impl Default for NetworkPlot {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            layout: GraphLayout::default(),
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            aspect_ratio: AspectRatio::Auto,
            show_labels: true,
            iterations: 50,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
        }
    }
}

impl NetworkPlot {
    /// Create a new empty network plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node.
    pub fn node(mut self, node: GraphNode) -> Self {
        self.nodes.push(node);
        self
    }

    /// Add multiple nodes at once.
    pub fn nodes(mut self, nodes: Vec<GraphNode>) -> Self {
        self.nodes.extend(nodes);
        self
    }

    /// Add an edge.
    pub fn edge(mut self, edge: GraphEdge) -> Self {
        self.edges.push(edge);
        self
    }

    /// Add multiple edges at once.
    pub fn edges(mut self, edges: Vec<GraphEdge>) -> Self {
        self.edges.extend(edges);
        self
    }

    /// Set the layout algorithm.
    pub fn layout(mut self, layout: GraphLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the X axis.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y axis.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the aspect ratio.
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
        self
    }

    /// Whether to show node labels.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Number of iterations for force-directed layout.
    pub fn iterations(mut self, n: usize) -> Self {
        self.iterations = n;
        self
    }

    /// Set the visual theme.
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

    /// Compute node positions based on the selected layout algorithm.
    fn compute_positions(&self) -> Vec<(f64, f64)> {
        let n = self.nodes.len();
        if n == 0 {
            return Vec::new();
        }

        match self.layout {
            GraphLayout::Manual => self
                .nodes
                .iter()
                .map(|node| node.position.unwrap_or((0.0, 0.0)))
                .collect(),

            GraphLayout::Circular => {
                let two_pi = 2.0 * std::f64::consts::PI;
                (0..n)
                    .map(|i| {
                        if let Some(pos) = self.nodes[i].position {
                            pos
                        } else {
                            let angle = two_pi * i as f64 / n as f64;
                            (angle.cos(), angle.sin())
                        }
                    })
                    .collect()
            }

            GraphLayout::ForceDirected => self.fruchterman_reingold(),
        }
    }

    /// Fruchterman-Reingold force-directed layout algorithm.
    fn fruchterman_reingold(&self) -> Vec<(f64, f64)> {
        let n = self.nodes.len();
        if n == 0 {
            return Vec::new();
        }
        if n == 1 {
            return vec![(0.0, 0.0)];
        }

        // Area and optimal distance
        let area = (n as f64).sqrt();
        let k = (area / n as f64).sqrt(); // optimal distance

        // Initialize positions: use provided positions or spread in a grid
        let mut pos: Vec<(f64, f64)> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                node.position.unwrap_or_else(|| {
                    let cols = (n as f64).sqrt().ceil() as usize;
                    let row = i / cols.max(1);
                    let col = i % cols.max(1);
                    (col as f64 * 0.5, row as f64 * 0.5)
                })
            })
            .collect();

        let mut temp = area * 0.1; // temperature (maximum displacement)

        for _iter in 0..self.iterations {
            let mut disp = vec![(0.0f64, 0.0f64); n];

            // Repulsive forces between all pairs
            for i in 0..n {
                for j in (i + 1)..n {
                    let dx = pos[i].0 - pos[j].0;
                    let dy = pos[i].1 - pos[j].1;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                    let force = k * k / dist; // repulsive force
                    let fx = force * dx / dist;
                    let fy = force * dy / dist;
                    disp[i].0 += fx;
                    disp[i].1 += fy;
                    disp[j].0 -= fx;
                    disp[j].1 -= fy;
                }
            }

            // Attractive forces along edges
            for edge in &self.edges {
                if edge.source >= n || edge.target >= n {
                    continue;
                }
                let dx = pos[edge.target].0 - pos[edge.source].0;
                let dy = pos[edge.target].1 - pos[edge.source].1;
                let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                let force = dist * dist / k * edge.weight; // attractive force
                let fx = force * dx / dist;
                let fy = force * dy / dist;
                disp[edge.source].0 += fx;
                disp[edge.source].1 += fy;
                disp[edge.target].0 -= fx;
                disp[edge.target].1 -= fy;
            }

            // Apply displacement, limited by temperature
            for i in 0..n {
                // Skip nodes with fixed positions
                if self.nodes[i].position.is_some() {
                    continue;
                }
                let mag = (disp[i].0 * disp[i].0 + disp[i].1 * disp[i].1)
                    .sqrt()
                    .max(0.001);
                let scale = temp.min(mag) / mag;
                pos[i].0 += disp[i].0 * scale;
                pos[i].1 += disp[i].1 * scale;
            }

            // Cool down
            temp *= 0.95;
        }

        pos
    }
}

impl Widget for &NetworkPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 6 || self.nodes.is_empty() {
            return;
        }

        let positions = self.compute_positions();

        // Compute data bounds from positions
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for &(x, y) in &positions {
            x_min = x_min.min(x);
            x_max = x_max.max(x);
            y_min = y_min.min(y);
            y_max = y_max.max(y);
        }

        // Add margin
        let x_margin = (x_max - x_min).max(0.1) * 0.15;
        let y_margin = (y_max - y_min).max(0.1) * 0.15;
        x_min -= x_margin;
        x_max += x_margin;
        y_min -= y_margin;
        y_max += y_margin;

        if x_min >= x_max {
            x_min = -1.0;
            x_max = 1.0;
        }
        if y_min >= y_max {
            y_min = -1.0;
            y_max = 1.0;
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(x_min, x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Create and render the plot frame
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .aspect_ratio(self.aspect_ratio.clone())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(
            area,
            buf,
            DataBounds {
                x_lo,
                x_hi,
                y_lo,
                y_hi,
            },
        ) else {
            return;
        };

        // Draw edges
        for edge in &self.edges {
            if edge.source >= positions.len() || edge.target >= positions.len() {
                continue;
            }

            let (x0, y0) = positions[edge.source];
            let (x1, y1) = positions[edge.target];
            let sx0 = pa.screen_x(x0);
            let sy0 = pa.screen_y(y0);
            let sx1 = pa.screen_x(x1);
            let sy1 = pa.screen_y(y1);

            let edge_color = edge.color.unwrap_or(self.theme.grid_color);

            draw_braille_line(buf, sx0, sy0, sx1, sy1, edge_color, &pa);
        }

        // Draw nodes on top of edges
        for (i, node) in self.nodes.iter().enumerate() {
            if i >= positions.len() {
                continue;
            }
            let (x, y) = positions[i];
            let sx = pa.screen_x(x);
            let sy = pa.screen_y(y);
            let xi = sx.round() as u16;
            let yi = sy.round() as u16;

            if pa.contains(xi, yi) {
                buf[(xi, yi)]
                    .set_char(node.marker.char())
                    .set_fg(node.color);
            }

            // Draw label next to node
            if self.show_labels && !node.label.is_empty() {
                let label_x = xi + 1;
                let label_y = yi;
                if label_y >= pa.y && label_y < pa.y + pa.height {
                    for (j, ch) in node.label.chars().enumerate() {
                        let lx = label_x + j as u16;
                        if lx < pa.x + pa.width {
                            buf[(lx, label_y)]
                                .set_char(ch)
                                .set_fg(self.theme.foreground);
                        }
                    }
                }
            }
        }
    }
}
