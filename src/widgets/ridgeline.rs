//! Ridgeline plot widget for overlapping density distributions.
//!
//! Displays multiple overlapping KDE density curves, one per category,
//! vertically offset to create a "joy division" or ridgeline effect.
//!
//! Requires the `statistics` feature.
//!
//! # Example
//!
//! ```ignore
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::ridgeline::{RidgelinePlot, RidgelineGroup};
//!
//! let plot = RidgelinePlot::new()
//!     .group(RidgelineGroup::new("Group A", vec![1.0, 2.0, 3.0, 2.5]))
//!     .group(RidgelineGroup::new("Group B", vec![2.0, 3.0, 4.0, 3.5]))
//!     .overlap(0.5)
//!     .fill(true)
//!     .title("Ridgeline Plot");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA, Z_FILL};
use crate::spines::Spines;
use crate::statistics::Kde;
use crate::theme::Theme;
use crate::ticker::NullLocator;
use crate::transform::data_to_screen;

/// A single group (category) in a ridgeline plot.
///
/// Each group contains raw data values that will be converted to a KDE
/// density curve during rendering.
#[derive(Clone, Debug)]
pub struct RidgelineGroup {
    /// Label for this group (displayed on the y-axis).
    pub name: String,
    /// Raw data values. Non-finite values are filtered out.
    pub data: Vec<f64>,
    /// Optional color override. If `None`, a color is auto-assigned from the theme.
    pub color: Option<Color>,
}

impl RidgelineGroup {
    /// Create a new ridgeline group with a label and data.
    pub fn new(name: impl Into<String>, data: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            data,
            color: None,
        }
    }

    /// Set the color for this group's density curve.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Return data with non-finite values filtered out.
    fn finite_data(&self) -> Vec<f64> {
        self.data
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .collect()
    }
}

/// A ridgeline plot widget displaying overlapping KDE density curves.
///
/// Multiple data groups are shown as stacked, partially overlapping
/// density curves. Each group occupies a horizontal band with its
/// KDE curve drawn within. The `overlap` parameter controls how much
/// adjacent bands overlap vertically.
pub struct RidgelinePlot {
    /// Data groups, rendered bottom-to-top.
    groups: Vec<RidgelineGroup>,
    /// X-axis configuration (shared data axis).
    x_axis: Axis,
    /// Chart title.
    title: Option<String>,
    /// Vertical overlap factor (0.0 = no overlap, 1.0 = full overlap). Default: 0.5.
    overlap: f64,
    /// Whether to fill under the density curves. Default: true.
    fill: bool,
    /// Visual theme.
    theme: Theme,
    /// Spine visibility control.
    spines: Spines,
    /// Reference lines drawn across the plot area.
    reference_lines: Vec<ReferenceLine>,
    /// Number of KDE evaluation points per group.
    kde_points: usize,
}

impl Default for RidgelinePlot {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            x_axis: Axis::new(),
            title: None,
            overlap: 0.5,
            fill: true,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            kde_points: 200,
        }
    }
}

impl RidgelinePlot {
    /// Create a new empty ridgeline plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a data group.
    pub fn group(mut self, g: RidgelineGroup) -> Self {
        self.groups.push(g);
        self
    }

    /// Add multiple data groups.
    pub fn groups(mut self, groups: Vec<RidgelineGroup>) -> Self {
        self.groups.extend(groups);
        self
    }

    /// Set the X-axis configuration.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the vertical overlap factor (0.0 to 1.0, default 0.5).
    ///
    /// Higher values cause adjacent density curves to overlap more,
    /// creating a denser visual. 0.0 means no overlap, 1.0 means
    /// each band fully overlaps the one below.
    pub fn overlap(mut self, overlap: f64) -> Self {
        self.overlap = overlap.clamp(0.0, 1.0);
        self
    }

    /// Set whether to fill under the density curves (default: true).
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
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

    /// Set the number of KDE evaluation points per group (default: 200).
    pub fn kde_points(mut self, n: usize) -> Self {
        self.kde_points = n.max(10);
        self
    }
}

impl Widget for &RidgelinePlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.groups.is_empty() {
            return;
        }

        let n_groups = self.groups.len();

        // Compute KDE for each group and collect results
        let kde = Kde::new().n_points(self.kde_points);
        let mut kde_results: Vec<(Vec<f64>, Vec<f64>)> = Vec::with_capacity(n_groups);
        let mut global_x_min = f64::INFINITY;
        let mut global_x_max = f64::NEG_INFINITY;
        let mut global_density_max = 0.0_f64;

        for group in &self.groups {
            let finite = group.finite_data();
            let (x_vals, densities) = if finite.is_empty() {
                (vec![0.0, 1.0], vec![0.0, 0.0])
            } else {
                kde.fit(&finite)
            };

            for &x in &x_vals {
                if x.is_finite() {
                    if x < global_x_min {
                        global_x_min = x;
                    }
                    if x > global_x_max {
                        global_x_max = x;
                    }
                }
            }

            for &d in &densities {
                if d.is_finite() && d > global_density_max {
                    global_density_max = d;
                }
            }

            kde_results.push((x_vals, densities));
        }

        // Handle degenerate case
        if !global_x_min.is_finite() || !global_x_max.is_finite() {
            global_x_min = 0.0;
            global_x_max = 1.0;
        }
        if (global_x_max - global_x_min).abs() < f64::EPSILON {
            global_x_min -= 1.0;
            global_x_max += 1.0;
        }
        if global_density_max <= 0.0 {
            global_density_max = 1.0;
        }

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(global_x_min, global_x_max);

        // Set up y-axis bounds for categorical layout
        let y_lo = 0.0;
        let y_hi = n_groups as f64;

        // Compute label width for y-axis margin
        let label_width: u16 = self
            .groups
            .iter()
            .map(|g| g.name.len() as u16)
            .max()
            .unwrap_or(0)
            .min(12)
            + 1;

        let mut pb = PlotBuffer::new(area);

        // Use NullLocator for y-axis (categorical labels drawn manually)
        let y_axis = Axis::new().locator(NullLocator);

        let frame = PlotFrame::new(&self.x_axis, &y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .y_label_width(label_width)
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

        // Each group gets a band; with overlap the effective band height grows
        // Band height without overlap:
        let base_band_height = pa.height as f64 / n_groups as f64;
        // With overlap, the band stretches: each curve can extend into the band above
        let overlap_pixels = base_band_height * self.overlap;
        let effective_band_height = base_band_height + overlap_pixels;

        // Render groups from bottom (last) to top (first) so that upper groups
        // visually overlay lower groups.
        for gi in (0..n_groups).rev() {
            let group = &self.groups[gi];
            let group_color = group
                .color
                .unwrap_or_else(|| self.theme.color_cycle.at(gi));

            let (ref x_vals, ref densities) = kde_results[gi];

            // Band baseline: bottom of this group's row (in screen coords, higher y = lower)
            // Group 0 is at the top of the plot, group n-1 at the bottom.
            let band_top_screen = pa.y as f64 + gi as f64 * base_band_height;
            let band_baseline_screen = band_top_screen + base_band_height;

            // Draw group label in the y-axis margin, vertically centered in the band
            let label_y = (band_top_screen + base_band_height * 0.5).round() as u16;
            let label = if group.name.len() > label_width as usize - 1 {
                &group.name[..label_width as usize - 1]
            } else {
                &group.name
            };
            let label_start = pa.x.saturating_sub(label_width);
            if label_y >= pa.y && label_y < pa.y + pa.height {
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < pa.x.saturating_sub(1) {
                        pb.set_char(lx, label_y, ch, group_color, Z_CHROME);
                    }
                }
            }

            // Render the density curve within this band.
            // The density maps from 0 (at baseline) to max_density (at the top of the band).
            // The curve extends upward from the baseline by effective_band_height pixels.

            // If fill is enabled, fill columns under the curve
            if self.fill {
                for (i, &xv) in x_vals.iter().enumerate() {
                    if !xv.is_finite() {
                        continue;
                    }
                    let sx = data_to_screen(
                        xv,
                        x_lo,
                        x_hi,
                        pa.x as f64,
                        (pa.x + pa.width - 1) as f64,
                    );
                    let xi = sx.round() as u16;
                    if xi < pa.x || xi >= pa.x + pa.width {
                        continue;
                    }

                    let d = densities[i];
                    if !d.is_finite() || d <= 0.0 {
                        continue;
                    }

                    // Curve height in pixels (density normalized to effective band height)
                    let curve_height = (d / global_density_max) * effective_band_height;
                    let curve_top_screen = band_baseline_screen - curve_height;

                    let y_top = curve_top_screen.round().max(pa.y as f64) as u16;
                    let y_bot = band_baseline_screen.round().min((pa.y + pa.height - 1) as f64) as u16;

                    for y in y_top..=y_bot {
                        if pa.contains(xi, y) {
                            pb.set_bg(xi, y, group_color, Z_FILL);
                        }
                    }
                }
            }

            // Draw the density curve line using Braille
            for i in 0..x_vals.len().saturating_sub(1) {
                let xv0 = x_vals[i];
                let xv1 = x_vals[i + 1];
                let d0 = densities[i];
                let d1 = densities[i + 1];

                if !xv0.is_finite() || !xv1.is_finite() || !d0.is_finite() || !d1.is_finite() {
                    continue;
                }

                let sx0 = data_to_screen(
                    xv0,
                    x_lo,
                    x_hi,
                    pa.x as f64,
                    (pa.x + pa.width - 1) as f64,
                );
                let sx1 = data_to_screen(
                    xv1,
                    x_lo,
                    x_hi,
                    pa.x as f64,
                    (pa.x + pa.width - 1) as f64,
                );

                let curve_h0 = (d0 / global_density_max) * effective_band_height;
                let curve_h1 = (d1 / global_density_max) * effective_band_height;

                let sy0 = band_baseline_screen - curve_h0;
                let sy1 = band_baseline_screen - curve_h1;

                pb.draw_line(
                    sx0,
                    sy0,
                    sx1,
                    sy1,
                    group_color,
                    &pa,
                    Z_DATA + (gi as u8).min(64),
                );
            }
        }

        // Composite to terminal buffer
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);
    }
}
