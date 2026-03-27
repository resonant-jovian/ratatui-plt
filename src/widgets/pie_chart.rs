//! Pie/donut chart widget inspired by matplotlib's `pie`.
//!
//! Renders slices of a circle, optionally with a donut hole, labels, and
//! percentage annotations. Supports exploded slices for emphasis.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::pie_chart::{PieChart, PieSlice};
//! use ratatui::style::Color;
//!
//! let chart = PieChart::new()
//!     .slice(PieSlice::new("Rust", 45.0).color(Color::Cyan))
//!     .slice(PieSlice::new("Python", 30.0).color(Color::Yellow))
//!     .slice(PieSlice::new("C++", 25.0).color(Color::Red))
//!     .title("Language Usage")
//!     .show_percentages(true)
//!     .donut_ratio(0.4);
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::plot_buffer::{PlotBackend, Z_CHROME, Z_DATA, create_backend};
use crate::theme::Theme;

/// A single slice of the pie chart.
#[derive(Clone, Debug)]
pub struct PieSlice {
    /// Label for the slice.
    pub label: String,
    /// Numeric value (proportion is computed from sum of all slices).
    pub value: f64,
    /// Fill color for the slice.
    pub color: Option<Color>,
    /// Explode offset as a fraction of the radius (0.0 = no offset).
    pub explode: f64,
}

impl PieSlice {
    /// Create a new slice with the given label and value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
            explode: 0.0,
        }
    }

    /// Set the slice color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the explode offset (fraction of radius, e.g. 0.1).
    pub fn explode(mut self, offset: f64) -> Self {
        self.explode = offset;
        self
    }
}

/// A ring of slices for multi-level pie charts.
#[derive(Clone, Debug)]
pub struct PieRing {
    slices: Vec<PieSlice>,
}

impl PieRing {
    /// Create a new ring from slices.
    pub fn new(slices: Vec<PieSlice>) -> Self {
        Self { slices }
    }
}

/// A pie/donut chart widget.
///
/// Renders data as angular slices of a circle. When `donut_ratio` is set,
/// the centre is hollowed out to form a donut chart. Additional concentric
/// rings can be added via [`PieChart::ring`] for nested pie charts
/// (plotly-style), where each ring is an independent pie at a different
/// radius.
pub struct PieChart {
    /// Slices of the pie (innermost ring / ring 0).
    slices: Vec<PieSlice>,
    /// Additional outer rings.
    rings: Vec<PieRing>,
    /// Chart title.
    title: Option<String>,
    /// Inner-to-outer radius ratio for donut mode (None = filled pie).
    donut_ratio: Option<f64>,
    /// Whether to draw labels outside the pie.
    show_labels: bool,
    /// Whether to show percentage text on each slice.
    show_percentages: bool,
    /// Visual theme.
    theme: Theme,
}

impl Default for PieChart {
    fn default() -> Self {
        Self {
            slices: Vec::new(),
            rings: Vec::new(),
            title: None,
            donut_ratio: None,
            show_labels: true,
            show_percentages: false,
            theme: Theme::get_default(),
        }
    }
}

impl PieChart {
    /// Create a new empty pie chart.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a slice to the pie chart.
    pub fn slice(mut self, s: PieSlice) -> Self {
        self.slices.push(s);
        self
    }

    /// Add multiple slices at once.
    pub fn slices(mut self, slices: Vec<PieSlice>) -> Self {
        self.slices.extend(slices);
        self
    }

    /// Append an outer ring of slices for a nested (concentric) pie chart.
    ///
    /// Each ring is an independent pie laid out across 360 degrees at a
    /// different radius band. The primary `slices` form the innermost
    /// ring; each call to `ring()` adds the next ring outward.
    pub fn ring(mut self, ring: PieRing) -> Self {
        self.rings.push(ring);
        self
    }

    /// Set all outer rings at once.
    pub fn rings(mut self, rings: Vec<PieRing>) -> Self {
        self.rings = rings;
        self
    }

    /// Set the chart title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    /// Enable donut mode with the given inner-to-outer radius ratio (0.0..1.0).
    pub fn donut_ratio(mut self, ratio: f64) -> Self {
        self.donut_ratio = Some(ratio.clamp(0.0, 0.99));
        self
    }

    /// Whether to show labels positioned outside the pie.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Whether to show percentage text on or near each slice.
    pub fn show_percentages(mut self, show: bool) -> Self {
        self.show_percentages = show;
        self
    }

    /// Set the visual theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

/// Precomputed per-ring data used during rendering.
struct RingData {
    /// References to the slices in this ring.
    slices: Vec<PieSlice>,
    /// Cumulative angular extents for each slice.
    angles: Vec<(f64, f64)>,
    /// Resolved colors for each slice.
    colors: Vec<Color>,
    /// Total value of all slices in this ring.
    total: f64,
    /// Normalised inner radius of this ring band (0.0..1.0).
    r_inner: f64,
    /// Normalised outer radius of this ring band (0.0..1.0).
    r_outer: f64,
}

impl Widget for &PieChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 8 || area.height < 6 || self.slices.is_empty() {
            return;
        }

        let total: f64 = self.slices.iter().map(|s| s.value.max(0.0)).sum();
        if total <= 0.0 {
            return;
        }

        // ── Single-ring fast path (preserves existing behaviour) ─────
        if self.rings.is_empty() {
            self.render_single_ring(area, buf, total);
            return;
        }

        // ── Multi-ring path ──────────────────────────────────────────
        self.render_multi_ring(area, buf, total);
    }
}

impl PieChart {
    // ── Single-ring rendering (unchanged original logic) ─────────
    fn render_single_ring(&self, area: Rect, buf: &mut Buffer, total: f64) {
        let mut pb = create_backend(area);

        // Reserve space for title
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let py = area.y + title_height;
        let ph = area.height.saturating_sub(title_height);

        // Reserve margin for labels on left/right
        let label_margin: u16 = if self.show_labels { 10 } else { 0 };
        let pw = area.width;

        if pw < label_margin * 2 + 4 || ph < 4 {
            return;
        }

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

        // Centre of the pie in screen coordinates
        let cx = area.x as f64 + pw as f64 / 2.0;
        let cy = py as f64 + ph as f64 / 2.0;

        // Radius (account for terminal cells being ~2x tall as wide)
        let r_screen_x = ((pw as f64 / 2.0) - label_margin as f64 - 1.0).max(2.0);
        let r_screen_y = if self.show_labels || self.show_percentages {
            ((ph as f64 / 2.3) - 1.0).max(2.0)
        } else {
            ((ph as f64 / 2.0) - 1.0).max(2.0)
        };

        let inner_ratio = self.donut_ratio.unwrap_or(0.0);

        // Precompute slice angles (cumulative)
        let mut angles: Vec<(f64, f64)> = Vec::with_capacity(self.slices.len());
        let mut angle_start = -std::f64::consts::FRAC_PI_2;
        for slice in &self.slices {
            let fraction = slice.value.max(0.0) / total;
            let sweep = fraction * 2.0 * std::f64::consts::PI;
            angles.push((angle_start, angle_start + sweep));
            angle_start += sweep;
        }

        // Pre-resolve slice colors
        let slice_colors: Vec<Color> = self
            .slices
            .iter()
            .enumerate()
            .map(|(i, s)| s.color.unwrap_or_else(|| self.theme.color_cycle.at(i)))
            .collect();

        // For each cell, determine which slice it belongs to
        for screen_y in py..py + ph {
            for screen_x in area.x..area.x + pw {
                let nx = (screen_x as f64 - cx) / r_screen_x;
                let ny = (screen_y as f64 - cy) / r_screen_y;
                let r = (nx * nx + ny * ny).sqrt();

                if r > 1.0 || r < inner_ratio {
                    continue;
                }

                for (si, (slice, &(a_start, a_end))) in
                    self.slices.iter().zip(angles.iter()).enumerate()
                {
                    let (ecx, ecy) = if slice.explode > 0.0 {
                        let mid_angle = (a_start + a_end) / 2.0;
                        let ex = slice.explode * mid_angle.cos() * r_screen_x;
                        let ey = slice.explode * mid_angle.sin() * r_screen_y;
                        (cx + ex, cy + ey)
                    } else {
                        (cx, cy)
                    };

                    let enx = (screen_x as f64 - ecx) / r_screen_x;
                    let eny = (screen_y as f64 - ecy) / r_screen_y;
                    let er = (enx * enx + eny * eny).sqrt();

                    if er > 1.0 || er < inner_ratio {
                        continue;
                    }

                    let mut ea = eny.atan2(enx);
                    while ea < a_start {
                        ea += 2.0 * std::f64::consts::PI;
                    }
                    while ea >= a_start + 2.0 * std::f64::consts::PI {
                        ea -= 2.0 * std::f64::consts::PI;
                    }

                    if ea >= a_start && ea < a_end {
                        let sc = slice_colors[si];
                        pb.set_cell(
                            screen_x,
                            screen_y,
                            self.theme.chars.fill.solid,
                            sc,
                            sc,
                            Z_DATA,
                        );
                        break;
                    }
                }
            }
        }

        #[cfg(feature = "unicode-extended")]
        {
            let n_arc = (2.0 * std::f64::consts::PI * r_screen_x.max(r_screen_y))
                .round()
                .max(24.0) as usize;
            for step in 0..n_arc {
                let theta = 2.0 * std::f64::consts::PI * step as f64 / n_arc as f64;
                let sx = cx + r_screen_x * theta.cos();
                let sy = cy + r_screen_y * theta.sin();
                let xi = sx.round() as u16;
                let yi = sy.round() as u16;
                if xi >= area.x && xi < area.x + area.width && yi >= py && yi < py + ph {
                    let arc_ch = match (theta.cos() >= 0.0, theta.sin() < 0.0) {
                        (false, true) => self.theme.chars.arc.top_left,
                        (true, true) => self.theme.chars.arc.top_right,
                        (true, false) => self.theme.chars.arc.bottom_right,
                        (false, false) => self.theme.chars.arc.bottom_left,
                    };
                    pb.set_char(xi, yi, arc_ch, self.theme.muted, Z_CHROME);
                }
            }
        }

        // Draw labels and/or percentages outside the pie
        if self.show_labels || self.show_percentages {
            for (i, slice) in self.slices.iter().enumerate() {
                let (a_start, a_end) = angles[i];
                let mid_angle = (a_start + a_end) / 2.0;
                let fraction = slice.value.max(0.0) / total;

                let mut text = String::new();
                if self.show_labels {
                    text.push_str(&slice.label);
                }
                if self.show_percentages {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(&format!("{:.1}%", fraction * 100.0));
                }

                if text.is_empty() {
                    continue;
                }

                let label_r = 1.15 + slice.explode;
                let lx = cx + label_r * r_screen_x * mid_angle.cos();
                let ly = cy + label_r * r_screen_y * mid_angle.sin();

                let xi = if mid_angle.cos() < 0.0 {
                    (lx - text.len() as f64).round() as i32
                } else {
                    lx.round() as i32
                };
                let yi = ly.round() as u16;

                if yi >= py && yi < py + ph {
                    for (j, ch) in text.chars().enumerate() {
                        let x = xi + j as i32;
                        if x >= area.x as i32 && x < (area.x + area.width) as i32 {
                            pb.set_char(x as u16, yi, ch, slice_colors[i], Z_CHROME);
                        }
                    }
                }
            }
        }

        pb.composite(buf);
    }

    // ── Multi-ring rendering ─────────────────────────────────────
    fn render_multi_ring(&self, area: Rect, buf: &mut Buffer, _ring0_total: f64) {
        let mut pb = create_backend(area);

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let py = area.y + title_height;
        let ph = area.height.saturating_sub(title_height);

        // Labels only for the outermost ring; still reserve margin
        // so the label text has room.
        let label_margin: u16 = if self.show_labels { 10 } else { 0 };
        let pw = area.width;

        if pw < label_margin * 2 + 4 || ph < 4 {
            return;
        }

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

        let cx = area.x as f64 + pw as f64 / 2.0;
        let cy = py as f64 + ph as f64 / 2.0;

        let r_screen_x = ((pw as f64 / 2.0) - label_margin as f64 - 1.0).max(2.0);
        let r_screen_y = if self.show_labels || self.show_percentages {
            ((ph as f64 / 2.3) - 1.0).max(2.0)
        } else {
            ((ph as f64 / 2.0) - 1.0).max(2.0)
        };

        let inner_ratio = self.donut_ratio.unwrap_or(0.0);
        let two_pi = 2.0 * std::f64::consts::PI;
        let start_angle = -std::f64::consts::FRAC_PI_2;

        // Collect all rings: ring 0 = self.slices, ring 1+ = self.rings
        let num_rings = 1 + self.rings.len();
        let ring_width = (1.0 - inner_ratio) / num_rings as f64;

        // Build per-ring precomputed data
        let mut ring_data: Vec<RingData> = Vec::with_capacity(num_rings);

        // Helper: build RingData from a slice list and a color-cycle
        // offset so colors across rings do not collide.
        let build_ring = |slices: &[PieSlice],
                          color_offset: usize,
                          r_inner: f64,
                          r_outer: f64,
                          theme: &Theme| {
            let total: f64 = slices.iter().map(|s| s.value.max(0.0)).sum();
            let mut angles = Vec::with_capacity(slices.len());
            let mut cursor = start_angle;
            if total > 0.0 {
                for slice in slices {
                    let frac = slice.value.max(0.0) / total;
                    let sweep = frac * two_pi;
                    angles.push((cursor, cursor + sweep));
                    cursor += sweep;
                }
            }
            let colors: Vec<Color> = slices
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    s.color
                        .unwrap_or_else(|| theme.color_cycle.at(color_offset + i))
                })
                .collect();
            RingData {
                slices: slices.to_vec(),
                angles,
                colors,
                total,
                r_inner,
                r_outer,
            }
        };

        // Ring 0 (innermost)
        let mut color_offset: usize = 0;
        ring_data.push(build_ring(
            &self.slices,
            color_offset,
            inner_ratio,
            inner_ratio + ring_width,
            &self.theme,
        ));
        color_offset += self.slices.len();

        // Outer rings
        for extra_ring in &self.rings {
            let ri = ring_data.len();
            let r_inner_band = inner_ratio + ri as f64 * ring_width;
            let r_outer_band = r_inner_band + ring_width;
            ring_data.push(build_ring(
                &extra_ring.slices,
                color_offset,
                r_inner_band,
                r_outer_band,
                &self.theme,
            ));
            color_offset += extra_ring.slices.len();
        }

        // ── Fill pixels ──────────────────────────────────────────
        for screen_y in py..py + ph {
            for screen_x in area.x..area.x + pw {
                let nx = (screen_x as f64 - cx) / r_screen_x;
                let ny = (screen_y as f64 - cy) / r_screen_y;
                let r = (nx * nx + ny * ny).sqrt();

                if r > 1.0 || r < inner_ratio {
                    continue;
                }

                // Determine which ring band
                let ring_idx = ring_data
                    .iter()
                    .position(|rd| r >= rd.r_inner && r < rd.r_outer);
                let ring_idx = match ring_idx {
                    Some(idx) => idx,
                    None => {
                        // Edge case: r == 1.0 exactly; assign to
                        // outermost ring.
                        if (r - 1.0).abs() < 1e-9 {
                            num_rings - 1
                        } else {
                            continue;
                        }
                    }
                };
                let rd = &ring_data[ring_idx];
                if rd.total <= 0.0 || rd.angles.is_empty() {
                    continue;
                }

                // Compute angle
                let mut angle = ny.atan2(nx);
                while angle < start_angle {
                    angle += two_pi;
                }
                while angle >= start_angle + two_pi {
                    angle -= two_pi;
                }

                // Find matching slice in this ring
                for (si, &(a_start, a_end)) in rd.angles.iter().enumerate() {
                    if angle >= a_start && angle < a_end {
                        let sc = rd.colors[si];
                        pb.set_cell(
                            screen_x,
                            screen_y,
                            self.theme.chars.fill.solid,
                            sc,
                            sc,
                            Z_DATA,
                        );
                        break;
                    }
                }
            }
        }

        // ── Unicode-extended arc characters on outer rim ─────────
        #[cfg(feature = "unicode-extended")]
        {
            let n_arc = (two_pi * r_screen_x.max(r_screen_y)).round().max(24.0) as usize;
            for step in 0..n_arc {
                let theta = two_pi * step as f64 / n_arc as f64;
                let sx = cx + r_screen_x * theta.cos();
                let sy = cy + r_screen_y * theta.sin();
                let xi = sx.round() as u16;
                let yi = sy.round() as u16;
                if xi >= area.x && xi < area.x + area.width && yi >= py && yi < py + ph {
                    let arc_ch = match (theta.cos() >= 0.0, theta.sin() < 0.0) {
                        (false, true) => self.theme.chars.arc.top_left,
                        (true, true) => self.theme.chars.arc.top_right,
                        (true, false) => self.theme.chars.arc.bottom_right,
                        (false, false) => self.theme.chars.arc.bottom_left,
                    };
                    pb.set_char(xi, yi, arc_ch, self.theme.muted, Z_CHROME);
                }
            }
        }

        // ── Labels (outermost ring only) ─────────────────────────
        if self.show_labels || self.show_percentages {
            let outer = &ring_data[num_rings - 1];
            if outer.total > 0.0 {
                for (i, slice) in outer.slices.iter().enumerate() {
                    if i >= outer.angles.len() {
                        break;
                    }
                    let (a_start, a_end) = outer.angles[i];
                    let mid_angle = (a_start + a_end) / 2.0;
                    let fraction = slice.value.max(0.0) / outer.total;

                    let mut text = String::new();
                    if self.show_labels {
                        text.push_str(&slice.label);
                    }
                    if self.show_percentages {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(&format!("{:.1}%", fraction * 100.0));
                    }

                    if text.is_empty() {
                        continue;
                    }

                    let label_r = 1.15;
                    let lx = cx + label_r * r_screen_x * mid_angle.cos();
                    let ly = cy + label_r * r_screen_y * mid_angle.sin();

                    let xi = if mid_angle.cos() < 0.0 {
                        (lx - text.len() as f64).round() as i32
                    } else {
                        lx.round() as i32
                    };
                    let yi = ly.round() as u16;

                    if yi >= py && yi < py + ph {
                        for (j, ch) in text.chars().enumerate() {
                            let x = xi + j as i32;
                            if x >= area.x as i32 && x < (area.x + area.width) as i32 {
                                pb.set_char(x as u16, yi, ch, outer.colors[i], Z_CHROME);
                            }
                        }
                    }
                }
            }
        }

        pb.composite(buf);
    }
}
