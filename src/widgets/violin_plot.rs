//! Violin plot widget for distribution comparison using kernel density estimation.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA, Z_MARKER};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::ticker::NullLocator;
use crate::transform::data_to_screen;

/// Controls how violin widths are normalized across groups.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DensityNorm {
    /// All violins have the same area (default behavior).
    #[default]
    Area,
    /// Violin area is proportional to the number of observations.
    Count,
    /// All violins have the same maximum width.
    Width,
}

/// Controls what is rendered inside each violin body.
#[derive(Clone, Debug, Default)]
pub enum ViolinInner {
    /// Show a mini box plot (IQR box + median line). This is the default.
    #[default]
    Box,
    /// Show horizontal lines at Q1, median, and Q3.
    Quartile,
    /// Show individual data points as dots along the center.
    Point,
    /// Show thin vertical sticks for each data value.
    Stick,
}

/// A single violin dataset.
#[derive(Clone, Debug)]
pub struct ViolinData {
    /// Category label.
    pub label: String,
    /// Raw data values.
    pub values: Vec<f64>,
    /// Violin color.
    pub color: Color,
}

impl ViolinData {
    pub fn new(label: impl Into<String>, values: Vec<f64>, color: Color) -> Self {
        Self {
            label: label.into(),
            values,
            color,
        }
    }

    /// Return a copy of values with NaN/Inf filtered out and sorted.
    fn finite_sorted(&self) -> Vec<f64> {
        let mut v: Vec<f64> = self
            .values
            .iter()
            .copied()
            .filter(|x| x.is_finite())
            .collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v
    }

    /// Compute quartiles (Q1, median, Q3) from sorted finite values.
    fn quartiles(sorted: &[f64]) -> (f64, f64, f64) {
        if sorted.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        (
            percentile(sorted, 25.0),
            percentile(sorted, 50.0),
            percentile(sorted, 75.0),
        )
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let k = (p / 100.0) * (sorted.len() - 1) as f64;
    let f = k.floor() as usize;
    let c = f.min(sorted.len() - 1);
    let d = k - f as f64;
    if c + 1 < sorted.len() {
        sorted[c] + d * (sorted[c + 1] - sorted[c])
    } else {
        sorted[c]
    }
}

/// Gaussian kernel density estimation.
///
/// For each evaluation point `y`, computes:
///   KDE(y) = (1 / (n * h)) * sum_i  exp(-0.5 * ((y - x_i) / h)^2) / sqrt(2 * pi)
///
/// Uses Silverman's rule of thumb for bandwidth: h = 0.9 * min(std, IQR/1.34) * n^(-1/5).
fn gaussian_kde(sorted_values: &[f64], eval_points: &[f64]) -> Vec<f64> {
    let n = sorted_values.len();
    if n == 0 {
        return vec![0.0; eval_points.len()];
    }
    let nf = n as f64;

    // Compute mean
    let mean = sorted_values.iter().sum::<f64>() / nf;

    // Compute standard deviation
    let variance = sorted_values
        .iter()
        .map(|&x| (x - mean) * (x - mean))
        .sum::<f64>()
        / nf;
    let std_dev = variance.sqrt();

    // Compute IQR
    let q1 = percentile(sorted_values, 25.0);
    let q3 = percentile(sorted_values, 75.0);
    let iqr = q3 - q1;

    // Silverman's rule of thumb
    let spread = if iqr > 0.0 {
        std_dev.min(iqr / 1.34)
    } else if std_dev > 0.0 {
        std_dev
    } else {
        1.0
    };
    let h = 0.9 * spread * nf.powf(-0.2);
    let h = if h <= 0.0 { 1.0 } else { h };

    let inv_h = 1.0 / h;
    let norm_factor = 1.0 / (nf * h * (2.0 * std::f64::consts::PI).sqrt());

    eval_points
        .iter()
        .map(|&y| {
            let sum: f64 = sorted_values
                .iter()
                .map(|&xi| {
                    let z = (y - xi) * inv_h;
                    (-0.5 * z * z).exp()
                })
                .sum();
            sum * norm_factor
        })
        .collect()
}

/// A violin plot widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::violin_plot::{ViolinPlot, ViolinData};
/// use ratatui::style::Color;
///
/// let plot = ViolinPlot::new()
///     .dataset(ViolinData::new("Group A", vec![1.0, 2.0, 3.0, 4.0, 5.0], Color::Cyan))
///     .dataset(ViolinData::new("Group B", vec![2.0, 3.0, 4.0, 5.0, 8.0], Color::Yellow))
///     .title("Violin Comparison")
///     .show_box(true);
/// ```
pub struct ViolinPlot {
    datasets: Vec<ViolinData>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_box: bool,
    /// What to render inside each violin body.
    show_inner: ViolinInner,
    /// When true with exactly 2 datasets, draw left-half for the first and
    /// right-half for the second at each position (split violin).
    split: bool,
    /// How violin widths are normalized across groups.
    density_norm: DensityNorm,
    /// Visual theme.
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for ViolinPlot {
    fn default() -> Self {
        Self {
            datasets: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_box: true,
            show_inner: ViolinInner::default(),
            split: false,
            density_norm: DensityNorm::default(),
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl ViolinPlot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dataset(mut self, d: ViolinData) -> Self {
        self.datasets.push(d);
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

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn show_box(mut self, show: bool) -> Self {
        self.show_box = show;
        self
    }

    /// Set what to render inside each violin body.
    pub fn show_inner(mut self, inner: ViolinInner) -> Self {
        self.show_inner = inner;
        self
    }

    /// Enable split violin mode.
    ///
    /// When `true` and there are exactly 2 datasets, the first dataset is
    /// drawn as the left half and the second as the right half at each
    /// position, enabling direct side-by-side comparison.
    pub fn split(mut self, split: bool) -> Self {
        self.split = split;
        self
    }

    /// Set how violin widths are normalized across groups.
    ///
    /// - `Area` (default): all violins have the same total area.
    /// - `Count`: violin area is proportional to the number of observations.
    /// - `Width`: all violins have the same maximum width.
    pub fn density_norm(mut self, norm: DensityNorm) -> Self {
        self.density_norm = norm;
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

    /// Add an annotation.
    pub fn annotation(mut self, ann: Annotation) -> Self {
        self.annotations.push(ann);
        self
    }
}

/// Side selector for split violin drawing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViolinSide {
    Both,
    Left,
    Right,
}

impl Widget for &ViolinPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.datasets.is_empty() {
            return;
        }

        // Compute global y range from all datasets (finite values only)
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for d in &self.datasets {
            for &v in &d.values {
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

        // Use NullLocator for x-axis to suppress x tick labels (categories drawn manually)
        let x_axis = Axis::new().locator(NullLocator);

        // In split mode with 2 datasets, we render them at 1 position instead of 2.
        let use_split = self.split && self.datasets.len() == 2;
        let n_slots = if use_split { 1 } else { self.datasets.len() };
        let x_lo = 0.0;
        let x_hi = n_slots as f64;

        let mut pb = PlotBuffer::new(area);

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

        let slot_width = pa.width / n_slots as u16;

        // Number of KDE evaluation points (one per screen row, doubled for half-block resolution)
        let n_eval = (pa.height as usize * 2).max(10);

        // Build the list of draw jobs: (dataset_index, slot_index, side)
        let jobs: Vec<(usize, usize, ViolinSide)> = if use_split {
            vec![(0, 0, ViolinSide::Left), (1, 0, ViolinSide::Right)]
        } else {
            self.datasets
                .iter()
                .enumerate()
                .map(|(i, _)| (i, i, ViolinSide::Both))
                .collect()
        };

        // Build evaluation grid spanning the y range (shared across all violins)
        let eval_points: Vec<f64> = (0..n_eval)
            .map(|j| y_lo + (y_hi - y_lo) * j as f64 / (n_eval - 1).max(1) as f64)
            .collect();

        // Pre-compute KDE values and sorted data for each dataset
        let mut precomputed: Vec<(Vec<f64>, Vec<f64>)> = Vec::with_capacity(self.datasets.len());
        for d in &self.datasets {
            let sorted = d.finite_sorted();
            let kde_values = if sorted.is_empty() {
                vec![0.0; n_eval]
            } else {
                gaussian_kde(&sorted, &eval_points)
            };
            precomputed.push((sorted, kde_values));
        }

        // Compute per-dataset normalization divisor based on DensityNorm.
        //
        // The divisor transforms raw KDE values so that:
        //   normalized_width = raw_kde / divisor * max_half_width
        //
        // - Width: divisor = per-violin kde_max  (current default behavior)
        // - Area:  divisor = per-violin kde_sum, scaled so largest sum maps to same
        //          width as Width mode (all violins get the same visual area)
        // - Count: divisor = global kde_max, then scale by (n_i / n_max) so wider
        //          violins represent larger groups
        let norm_divisors: Vec<f64> = match self.density_norm {
            DensityNorm::Width => {
                // Each violin independently normalized to the same max width
                precomputed
                    .iter()
                    .map(|(_, kde)| kde.iter().cloned().fold(0.0f64, f64::max))
                    .collect()
            }
            DensityNorm::Area => {
                // Normalize by total area (sum of KDE values) so all violins
                // have the same visual area.  We scale so that the violin with
                // the largest area sum still fills the available width.
                let sums: Vec<f64> = precomputed
                    .iter()
                    .map(|(_, kde)| kde.iter().sum::<f64>())
                    .collect();
                let max_sum = sums.iter().cloned().fold(0.0f64, f64::max);
                if max_sum <= 0.0 {
                    vec![1.0; precomputed.len()]
                } else {
                    // For each violin: divisor = kde_max * (sum / max_sum)
                    // This means a violin with half the area will be twice as wide
                    // per-density-unit, keeping total visual area constant.
                    precomputed
                        .iter()
                        .zip(sums.iter())
                        .map(|((_, kde), &sum)| {
                            let kde_max = kde.iter().cloned().fold(0.0f64, f64::max);
                            if sum > 0.0 {
                                kde_max * (sum / max_sum)
                            } else {
                                1.0
                            }
                        })
                        .collect()
                }
            }
            DensityNorm::Count => {
                // Scale width proportionally to observation count.
                // All violins share a global KDE max, then each is scaled by n_i/n_max.
                let global_kde_max = precomputed
                    .iter()
                    .flat_map(|(_, kde)| kde.iter().cloned())
                    .fold(0.0f64, f64::max);
                let max_n = precomputed
                    .iter()
                    .map(|(sorted, _)| sorted.len())
                    .max()
                    .unwrap_or(1)
                    .max(1) as f64;
                if global_kde_max <= 0.0 {
                    vec![1.0; precomputed.len()]
                } else {
                    precomputed
                        .iter()
                        .map(|(sorted, _)| {
                            let count_scale = sorted.len() as f64 / max_n;
                            if count_scale > 0.0 {
                                global_kde_max / count_scale
                            } else {
                                1.0
                            }
                        })
                        .collect()
                }
            }
        };

        for &(di, slot, side) in &jobs {
            let d = &self.datasets[di];
            let (ref sorted, ref kde_values) = precomputed[di];
            if sorted.is_empty() {
                continue;
            }

            let center_x = pa.x + (slot as u16 * slot_width) + slot_width / 2;

            // Normalization divisor for this violin
            let kde_divisor = norm_divisors[di];
            if kde_divisor <= 0.0 {
                continue;
            }

            // Max half-width in characters for one side of the violin
            let max_half_width = (slot_width / 2).saturating_sub(1).max(1) as f64;

            // Render the violin shape using half-block characters.
            for row in 0..pa.height {
                let screen_y = pa.y + row;
                if screen_y >= area.y + area.height {
                    break;
                }

                // Map screen row to eval index. Top of screen = high y, bottom = low y.
                let top_eval_idx_f = (1.0 - (row as f64 * 2.0) / (n_eval as f64 - 1.0).max(1.0))
                    * (n_eval - 1) as f64;
                let top_idx = (top_eval_idx_f.round() as usize).min(n_eval - 1);
                let bot_eval_idx_f = (1.0
                    - (row as f64 * 2.0 + 1.0) / (n_eval as f64 - 1.0).max(1.0))
                    * (n_eval - 1) as f64;
                let bot_idx = (bot_eval_idx_f.round() as usize).min(n_eval - 1);

                let top_width =
                    (kde_values[top_idx] / kde_divisor * max_half_width).round() as u16;
                let bot_width =
                    (kde_values[bot_idx] / kde_divisor * max_half_width).round() as u16;

                let max_w = top_width.max(bot_width);
                for dx in 0..=max_w {
                    let top_filled = dx <= top_width && top_width > 0;
                    let bot_filled = dx <= bot_width && bot_width > 0;

                    let ch = match (top_filled, bot_filled) {
                        (true, true) => self.theme.chars.fill.solid,
                        (true, false) => self.theme.chars.fill.half_upper,
                        (false, true) => self.theme.chars.fill.half_lower,
                        (false, false) => continue,
                    };

                    // Collect screen x positions depending on side
                    let positions = match side {
                        ViolinSide::Both => {
                            if dx == 0 {
                                vec![center_x]
                            } else {
                                let mut p = Vec::new();
                                if center_x + dx < pa.x + pa.width {
                                    p.push(center_x + dx);
                                }
                                if center_x >= dx + pa.x {
                                    p.push(center_x - dx);
                                }
                                p
                            }
                        }
                        ViolinSide::Left => {
                            if center_x >= dx + pa.x {
                                vec![center_x - dx]
                            } else {
                                vec![]
                            }
                        }
                        ViolinSide::Right => {
                            if center_x + dx < pa.x + pa.width {
                                vec![center_x + dx]
                            } else {
                                vec![]
                            }
                        }
                    };

                    for &sx in &positions {
                        if pa.contains(sx, screen_y) {
                            pb.set_char(sx, screen_y, ch, d.color, Z_DATA);
                        }
                    }
                }
            }

            // Draw inner decoration based on show_inner (respecting show_box for compat)
            let draw_inner = self.show_box;
            if draw_inner {
                let (q1, median, q3) = ViolinData::quartiles(sorted);
                let sy_q1 =
                    data_to_screen(q1, y_lo, y_hi, (pa.y + pa.height - 1) as f64, pa.y as f64)
                        .round() as u16;
                let sy_median = data_to_screen(
                    median,
                    y_lo,
                    y_hi,
                    (pa.y + pa.height - 1) as f64,
                    pa.y as f64,
                )
                .round() as u16;
                let sy_q3 =
                    data_to_screen(q3, y_lo, y_hi, (pa.y + pa.height - 1) as f64, pa.y as f64)
                        .round() as u16;

                match &self.show_inner {
                    ViolinInner::Box => {
                        // Draw thin box from Q1 to Q3
                        let box_half = 1u16;
                        for y in sy_q3..=sy_q1 {
                            if y >= pa.y && y < pa.y + pa.height {
                                for dx in 0..=box_half {
                                    let positions = inner_positions(center_x, dx, side, &pa);
                                    for sx in positions {
                                        pb.set_char(
                                            sx,
                                            y,
                                            self.theme.chars.border.vertical,
                                            self.theme.foreground,
                                            Z_MARKER,
                                        );
                                    }
                                }
                            }
                        }
                        // Median line
                        if sy_median >= pa.y && sy_median < pa.y + pa.height {
                            for dx in 0..=box_half {
                                let positions = inner_positions(center_x, dx, side, &pa);
                                for sx in positions {
                                    pb.set_char(
                                        sx,
                                        sy_median,
                                        self.theme.chars.dash.bold_h,
                                        self.theme.foreground,
                                        Z_MARKER,
                                    );
                                }
                            }
                        }
                    }
                    ViolinInner::Quartile => {
                        // Draw horizontal lines at Q1, median, and Q3
                        let half = 2u16.min((slot_width / 4).max(1));
                        for &sy in &[sy_q1, sy_median, sy_q3] {
                            if sy >= pa.y && sy < pa.y + pa.height {
                                for dx in 0..=half {
                                    let positions = inner_positions(center_x, dx, side, &pa);
                                    for sx in positions {
                                        let ch = if sy == sy_median {
                                            self.theme.chars.dash.bold_h
                                        } else {
                                            self.theme.chars.border.horizontal
                                        };
                                        pb.set_char(sx, sy, ch, self.theme.foreground, Z_MARKER);
                                    }
                                }
                            }
                        }
                    }
                    ViolinInner::Point => {
                        for &v in sorted {
                            let sy = data_to_screen(
                                v,
                                y_lo,
                                y_hi,
                                (pa.y + pa.height - 1) as f64,
                                pa.y as f64,
                            )
                            .round() as u16;
                            if pa.contains(center_x, sy) {
                                pb.set_char(
                                    center_x,
                                    sy,
                                    self.theme.chars.marker.small_point,
                                    self.theme.foreground,
                                    Z_MARKER,
                                );
                            }
                        }
                    }
                    ViolinInner::Stick => {
                        for &v in sorted {
                            let sy = data_to_screen(
                                v,
                                y_lo,
                                y_hi,
                                (pa.y + pa.height - 1) as f64,
                                pa.y as f64,
                            )
                            .round() as u16;
                            if pa.contains(center_x, sy) {
                                pb.set_char(
                                    center_x,
                                    sy,
                                    self.theme.chars.border.vertical,
                                    self.theme.foreground,
                                    Z_MARKER,
                                );
                            }
                        }
                    }
                }
            }

            // Category label
            let label = &d.label;
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

/// Helper: compute screen x positions for inner decoration, respecting split side.
fn inner_positions(
    center_x: u16,
    dx: u16,
    side: ViolinSide,
    pa: &crate::frame::PlotArea,
) -> Vec<u16> {
    match side {
        ViolinSide::Both => {
            if dx == 0 {
                vec![center_x]
            } else {
                let mut p = Vec::new();
                if center_x + dx < pa.x + pa.width {
                    p.push(center_x + dx);
                }
                if center_x >= dx + pa.x {
                    p.push(center_x - dx);
                }
                p
            }
        }
        ViolinSide::Left => {
            if center_x >= dx + pa.x {
                vec![center_x - dx]
            } else {
                vec![]
            }
        }
        ViolinSide::Right => {
            if center_x + dx < pa.x + pa.width {
                vec![center_x + dx]
            } else {
                vec![]
            }
        }
    }
}
