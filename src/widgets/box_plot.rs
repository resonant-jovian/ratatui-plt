//! Box plot widget for distribution comparison.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBackend, Z_CHROME, Z_DATA, Z_MARKER, create_backend};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::ticker::NullLocator;
use crate::transform::data_to_screen;

/// A single box-and-whisker dataset.
#[derive(Clone, Debug)]
pub struct BoxData {
    /// Category label.
    pub label: String,
    /// Raw data values (will be sorted to compute quartiles).
    pub values: Vec<f64>,
    /// Box color.
    pub color: Color,
}

impl BoxData {
    pub fn new(label: impl Into<String>, values: Vec<f64>, color: Color) -> Self {
        Self {
            label: label.into(),
            values,
            color,
        }
    }

    /// Compute quartiles (Q1, median, Q3).
    pub fn quartiles(&self) -> (f64, f64, f64) {
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();
        if n == 0 {
            return (0.0, 0.0, 0.0);
        }
        let median = percentile(&sorted, 50.0);
        let q1 = percentile(&sorted, 25.0);
        let q3 = percentile(&sorted, 75.0);
        (q1, median, q3)
    }

    /// Compute whisker endpoints (1.5 × IQR rule).
    pub fn whiskers(&self) -> (f64, f64) {
        let (q1, _, q3) = self.quartiles();
        let iqr = q3 - q1;
        let lower_fence = q1 - 1.5 * iqr;
        let upper_fence = q3 + 1.5 * iqr;

        let lower = self
            .values
            .iter()
            .filter(|&&v| v >= lower_fence)
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let upper = self
            .values
            .iter()
            .filter(|&&v| v <= upper_fence)
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        (lower, upper)
    }

    /// Compute the mean value.
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    /// Get outlier values.
    pub fn outliers(&self) -> Vec<f64> {
        let (q1, _, q3) = self.quartiles();
        let iqr = q3 - q1;
        let lower = q1 - 1.5 * iqr;
        let upper = q3 + 1.5 * iqr;
        self.values
            .iter()
            .filter(|&&v| v < lower || v > upper)
            .cloned()
            .collect()
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

/// A box plot widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::box_plot::{BoxPlot, BoxData};
/// use ratatui::style::Color;
///
/// let plot = BoxPlot::new()
///     .box_data(BoxData::new("Group A", vec![1.0, 2.0, 3.0, 4.0, 5.0], Color::Cyan))
///     .box_data(BoxData::new("Group B", vec![2.0, 3.0, 4.0, 5.0, 8.0], Color::Yellow))
///     .title("Distribution Comparison");
/// ```
pub struct BoxPlot {
    data: Vec<BoxData>,
    title: Option<String>,
    y_axis: Axis,
    show_outliers: bool,
    show_means: bool,
    notch: bool,
    /// Enable bootstrap confidence interval for the median.
    bootstrap_ci: bool,
    /// Number of bootstrap resamples (default: 1000).
    bootstrap_n: usize,
    /// Show individual data points alongside each box.
    show_points: bool,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
    /// Whether to fill box interiors with color.
    fill_boxes: bool,
}

impl Default for BoxPlot {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            title: None,
            y_axis: Axis::new(),
            show_outliers: true,
            show_means: false,
            notch: false,
            bootstrap_ci: false,
            bootstrap_n: 1000,
            show_points: false,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
            fill_boxes: true,
        }
    }
}

impl BoxPlot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn box_data(mut self, d: BoxData) -> Self {
        self.data.push(d);
        self
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    pub fn show_outliers(mut self, show: bool) -> Self {
        self.show_outliers = show;
        self
    }

    /// Show or hide mean markers (drawn as a diamond at the mean value).
    pub fn show_means(mut self, show: bool) -> Self {
        self.show_means = show;
        self
    }

    /// Enable or disable filled box interiors (default: true).
    pub fn fill_boxes(mut self, fill: bool) -> Self {
        self.fill_boxes = fill;
        self
    }

    /// Enable or disable notched box display.
    ///
    /// When enabled, boxes are narrower at the median region, with the notch
    /// extending to median +/- 1.57*IQR/sqrt(n). Overlapping notches between
    /// groups suggest no significant difference in medians.
    pub fn notch(mut self, notch: bool) -> Self {
        self.notch = notch;
        self
    }

    /// Show individual data points alongside each box.
    ///
    /// When enabled, raw data values are rendered as scatter markers next to
    /// the box with a small deterministic horizontal jitter for visibility.
    pub fn show_points(mut self, show: bool) -> Self {
        self.show_points = show;
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

    /// Enable bootstrap confidence interval for the median.
    ///
    /// When enabled, the box is rendered with a notch at the median whose
    /// extent is determined by bootstrap resampling rather than the
    /// 1.57*IQR/sqrt(n) approximation.
    pub fn bootstrap_ci(mut self, enabled: bool) -> Self {
        self.bootstrap_ci = enabled;
        self
    }

    /// Set the number of bootstrap resamples (default: 1000).
    pub fn bootstrap_n(mut self, n: usize) -> Self {
        self.bootstrap_n = n.max(10);
        self
    }
}

/// Simple LCG pseudo-random number generator (no external dependency).
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }
    fn next_u64(&mut self) -> u64 {
        // LCG parameters from Numerical Recipes
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }
    fn next_usize(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

/// Compute the bootstrap 95% confidence interval for the median.
fn bootstrap_median_ci(data: &[f64], n_resamples: usize) -> (f64, f64) {
    if data.len() < 2 {
        let m = if data.is_empty() { 0.0 } else { data[0] };
        return (m, m);
    }
    let n = data.len();
    let mut rng = SimpleRng::new(n as u64 ^ 0xDEADBEEF);
    let mut medians = Vec::with_capacity(n_resamples);

    for _ in 0..n_resamples {
        let mut sample = Vec::with_capacity(n);
        for _ in 0..n {
            sample.push(data[rng.next_usize(n)]);
        }
        sample.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        medians.push(percentile(&sample, 50.0));
    }

    medians.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo = percentile(&medians, 2.5);
    let hi = percentile(&medians, 97.5);
    (lo, hi)
}

impl Widget for &BoxPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.data.is_empty() {
            return;
        }

        // Compute global y range
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for d in &self.data {
            for &v in &d.values {
                y_min = y_min.min(v);
                y_max = y_max.max(v);
            }
        }
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(y_min, y_max);

        // Use NullLocator for x-axis to suppress x tick labels (categories drawn manually)
        let x_axis = Axis::new().locator(NullLocator);
        let n = self.data.len();
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

        // Force odd width so the center cell is exactly the whisker position
        let box_width = if self.fill_boxes {
            let w = ((pa.width * 3) / (n as u16 * 4)).max(3);
            if w.is_multiple_of(2) { w + 1 } else { w }
        } else {
            let w = ((pa.width * 4) / (n as u16 * 5)).max(5);
            if w.is_multiple_of(2) { w + 1 } else { w }
        };

        for (i, d) in self.data.iter().enumerate() {
            // Center box within its slot — single division, no rounding error
            let slot_start = pa.x + (i as u16 * pa.width / n as u16);
            let slot_end = pa.x + ((i as u16 + 1) * pa.width / n as u16);
            let box_left = (slot_start + slot_end).saturating_sub(box_width) / 2;
            let box_right = box_left + box_width;
            let center_x = box_left + box_width / 2;

            let (q1, median, q3) = d.quartiles();
            let (whisker_lo, whisker_hi) = d.whiskers();

            let sy_q1 = data_to_screen(q1, y_lo, y_hi, (pa.y + pa.height - 1) as f64, pa.y as f64)
                .round() as u16;
            let sy_median = data_to_screen(
                median,
                y_lo,
                y_hi,
                (pa.y + pa.height - 1) as f64,
                pa.y as f64,
            )
            .round() as u16;
            let sy_q3 = data_to_screen(q3, y_lo, y_hi, (pa.y + pa.height - 1) as f64, pa.y as f64)
                .round() as u16;
            let sy_wlo = data_to_screen(
                whisker_lo,
                y_lo,
                y_hi,
                (pa.y + pa.height - 1) as f64,
                pa.y as f64,
            )
            .round() as u16;
            let sy_whi = data_to_screen(
                whisker_hi,
                y_lo,
                y_hi,
                (pa.y + pa.height - 1) as f64,
                pa.y as f64,
            )
            .round() as u16;

            // Notch calculation: bootstrap CI or 1.57*IQR/sqrt(n)
            let (notch_lo_y, notch_hi_y, notch_left, notch_right) =
                if (self.notch || self.bootstrap_ci) && d.values.len() > 1 {
                    let (notch_lo, notch_hi) = if self.bootstrap_ci {
                        let (ci_lo, ci_hi) = bootstrap_median_ci(&d.values, self.bootstrap_n);
                        (ci_lo.max(q1), ci_hi.min(q3))
                    } else {
                        let iqr = q3 - q1;
                        let notch_extent = 1.57 * iqr / (d.values.len() as f64).sqrt();
                        (
                            (median - notch_extent).max(q1),
                            (median + notch_extent).min(q3),
                        )
                    };
                    let sy_notch_lo = data_to_screen(
                        notch_lo,
                        y_lo,
                        y_hi,
                        (pa.y + pa.height - 1) as f64,
                        pa.y as f64,
                    )
                    .round() as u16;
                    let sy_notch_hi = data_to_screen(
                        notch_hi,
                        y_lo,
                        y_hi,
                        (pa.y + pa.height - 1) as f64,
                        pa.y as f64,
                    )
                    .round() as u16;
                    let notch_inset = (box_width / 6).max(1);
                    let n_left = box_left + notch_inset;
                    let n_right = box_right.saturating_sub(notch_inset);
                    (sy_notch_lo, sy_notch_hi, n_left, n_right)
                } else {
                    (sy_median, sy_median, box_left, box_right)
                };

            let border_fg = if self.fill_boxes {
                self.theme.foreground
            } else {
                d.color
            };

            let is_notched = (self.notch || self.bootstrap_ci) && d.values.len() > 1;

            if self.fill_boxes {
                // Filled mode: solid color rectangle, NO outline. Fill IS the box.
                // When notched, narrow the box in the notch region to create a
                // visible pinch around the median.
                for y in sy_q3..=sy_q1 {
                    let (row_left, row_right) = if is_notched && y >= notch_hi_y && y <= notch_lo_y
                    {
                        (notch_left, notch_right)
                    } else {
                        (box_left, box_right)
                    };
                    for x in row_left..row_right {
                        if pa.contains(x, y) {
                            pb.set_cell(x, y, ' ', d.color, d.color, Z_DATA);
                        }
                    }
                }
            } else {
                // Unfilled mode: draw box outline only
                // Top edge (Q3) with corners
                if pa.contains(box_left, sy_q3) {
                    pb.set_char(
                        box_left,
                        sy_q3,
                        self.theme.chars.border.top_left,
                        border_fg,
                        Z_DATA,
                    );
                }
                for x in (box_left + 1)..box_right.saturating_sub(1) {
                    if pa.contains(x, sy_q3) {
                        pb.set_char(
                            x,
                            sy_q3,
                            self.theme.chars.border.horizontal,
                            border_fg,
                            Z_DATA,
                        );
                    }
                }
                if box_right > box_left + 1 && pa.contains(box_right - 1, sy_q3) {
                    pb.set_char(
                        box_right - 1,
                        sy_q3,
                        self.theme.chars.border.top_right,
                        border_fg,
                        Z_DATA,
                    );
                }
                // Bottom edge (Q1) with corners
                if pa.contains(box_left, sy_q1) {
                    pb.set_char(
                        box_left,
                        sy_q1,
                        self.theme.chars.border.bottom_left,
                        border_fg,
                        Z_DATA,
                    );
                }
                for x in (box_left + 1)..box_right.saturating_sub(1) {
                    if pa.contains(x, sy_q1) {
                        pb.set_char(
                            x,
                            sy_q1,
                            self.theme.chars.border.horizontal,
                            border_fg,
                            Z_DATA,
                        );
                    }
                }
                if box_right > box_left + 1 && pa.contains(box_right - 1, sy_q1) {
                    pb.set_char(
                        box_right - 1,
                        sy_q1,
                        self.theme.chars.border.bottom_right,
                        border_fg,
                        Z_DATA,
                    );
                }
                // Side walls — indented in the notch region when notched
                {
                    for y in (sy_q3 + 1)..sy_q1 {
                        let (wall_left, wall_right) =
                            if is_notched && y >= notch_hi_y && y <= notch_lo_y {
                                (notch_left, notch_right)
                            } else {
                                (box_left, box_right)
                            };
                        if pa.contains(wall_left, y) {
                            pb.set_char(
                                wall_left,
                                y,
                                self.theme.chars.border.vertical,
                                border_fg,
                                Z_DATA,
                            );
                        }
                        if wall_right > 0 && pa.contains(wall_right - 1, y) {
                            pb.set_char(
                                wall_right - 1,
                                y,
                                self.theme.chars.border.vertical,
                                border_fg,
                                Z_DATA,
                            );
                        }
                    }
                }
            }

            // Median line — narrower when notched to match the pinch
            let (median_left, median_right) = if is_notched {
                (notch_left, notch_right)
            } else {
                (box_left, box_right)
            };
            for x in median_left..median_right {
                if pa.contains(x, sy_median) {
                    pb.set_char(
                        x,
                        sy_median,
                        self.theme.chars.border.horizontal,
                        border_fg,
                        Z_DATA,
                    );
                }
            }

            // Whiskers (dashed style like matplotlib)
            for y in sy_whi..sy_q3 {
                if pa.contains(center_x, y) {
                    pb.set_char(center_x, y, '┆', border_fg, Z_DATA);
                }
            }
            for y in (sy_q1 + 1)..=sy_wlo {
                if pa.contains(center_x, y) {
                    pb.set_char(center_x, y, '┆', border_fg, Z_DATA);
                }
            }

            // Whisker caps (half box width for matplotlib-style T-caps)
            let cap_left = center_x.saturating_sub(box_width / 3);
            let cap_right = center_x + box_width / 3;
            for x in cap_left..=cap_right {
                if pa.contains(x, sy_whi) {
                    pb.set_char(
                        x,
                        sy_whi,
                        self.theme.chars.border.horizontal,
                        border_fg,
                        Z_DATA,
                    );
                }
                if pa.contains(x, sy_wlo) {
                    pb.set_char(
                        x,
                        sy_wlo,
                        self.theme.chars.border.horizontal,
                        border_fg,
                        Z_DATA,
                    );
                }
            }

            // Outliers
            if self.show_outliers {
                for &v in &d.outliers() {
                    let sy =
                        data_to_screen(v, y_lo, y_hi, (pa.y + pa.height - 1) as f64, pa.y as f64)
                            .round() as u16;
                    if pa.contains(center_x, sy) {
                        let marker_fg = if self.fill_boxes {
                            crate::drawing::contrasting_color(d.color)
                        } else {
                            self.theme.foreground
                        };
                        pb.set_char(
                            center_x,
                            sy,
                            self.theme.chars.marker.default_point,
                            marker_fg,
                            Z_MARKER,
                        );
                    }
                }
            }

            // Mean marker (diamond)
            if self.show_means {
                let mean_val = d.mean();
                let sy_mean = data_to_screen(
                    mean_val,
                    y_lo,
                    y_hi,
                    (pa.y + pa.height - 1) as f64,
                    pa.y as f64,
                )
                .round() as u16;
                if pa.contains(center_x, sy_mean) {
                    let marker_fg = if self.fill_boxes {
                        crate::drawing::contrasting_color(d.color)
                    } else {
                        self.theme.foreground
                    };
                    pb.set_char(
                        center_x,
                        sy_mean,
                        self.theme.chars.marker.default_point,
                        marker_fg,
                        Z_MARKER,
                    );
                }
            }

            // Individual data points with deterministic horizontal jitter
            if self.show_points {
                // Seed the RNG deterministically from the dataset index
                let mut rng = SimpleRng::new(i as u64 ^ 0xCAFEBABE);
                let jitter_range = (box_width / 3).max(1) as i16;
                for &v in &d.values {
                    let sy =
                        data_to_screen(v, y_lo, y_hi, (pa.y + pa.height - 1) as f64, pa.y as f64)
                            .round() as u16;
                    // Deterministic jitter: map RNG output to [-jitter_range, jitter_range]
                    let jitter =
                        (rng.next_u64() % (2 * jitter_range as u64 + 1)) as i16 - jitter_range;
                    let sx = (center_x as i16 + jitter).max(pa.x as i16) as u16;
                    if pa.contains(sx, sy) {
                        let marker_fg = if self.fill_boxes {
                            crate::drawing::contrasting_color(d.color)
                        } else {
                            self.theme.foreground
                        };
                        pb.set_char(
                            sx,
                            sy,
                            self.theme.chars.marker.default_point,
                            marker_fg,
                            Z_MARKER,
                        );
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

        // Draw End-positioned labels after composite (bypasses PB bounds)
        frame.draw_end_labels(buf, area, &pa);
    }
}
