//! Radial (polar) plot widget.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::drawing::BRAILLE_BITS;
use crate::plot_buffer::{PlotBackend, Z_CHROME, Z_DATA, Z_FILL, Z_GRID, Z_MARKER, create_backend};
use crate::series::Series;
use crate::theme::Theme;

/// Theta direction for polar plots.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ThetaDirection {
    /// Angles increase counter-clockwise (default, mathematical convention).
    #[default]
    CounterClockwise,
    /// Angles increase clockwise (compass convention).
    Clockwise,
}

/// Type of polar plot rendering.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PolarPlotType {
    /// Connected line segments (default).
    #[default]
    Line,
    /// Markers only, no connecting lines.
    Scatter,
    /// Wedge/bar segments from origin.
    Bar,
    /// Filled region between data and r_min.
    FillBetween,
}

/// A radial (polar coordinate) plot widget.
///
/// Displays data in polar coordinates with circular grid lines and angular tick marks.
/// Supports multiple visualization modes via [`PolarPlotType`]:
///
/// - **Radar/spider chart:** Use `PolarPlotType::Line` (or `FillBetween` for filled).
///   Supply data as equally-spaced angles with the polygon closed by appending the first
///   point.
/// - **Rose/polar bar chart:** Use `PolarPlotType::Bar` for filled wedge sectors from
///   the origin. Each data point defines a wedge at angle theta with height r.
/// - **Polar scatter:** Use `PolarPlotType::Scatter` for point markers only.
///
/// The type alias [`RadarPlot`] is provided for convenience when building radar/spider charts.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// // Cardioid: r = 1 + cos(θ)
/// let data: Vec<(f64, f64)> = (0..360)
///     .map(|i| {
///         let theta = i as f64 * std::f64::consts::PI / 180.0;
///         (theta, 1.0 + theta.cos())
///     })
///     .collect();
/// let plot = RadialPlot::new()
///     .series(Series::new("cardioid").data(data).color(Color::Cyan))
///     .title("Polar Plot");
/// ```
pub struct RadialPlot {
    /// Series data: (theta_radians, r) pairs.
    series: Vec<Series>,
    title: Option<String>,
    n_rings: usize,
    n_spokes: usize,
    r_max: Option<f64>,
    /// Minimum radial value (default: 0.0).
    r_min: Option<f64>,
    /// Direction of increasing theta.
    theta_direction: ThetaDirection,
    /// Offset for theta=0 position in radians (default: 0 = right/east).
    theta_offset: f64,
    /// Type of polar plot rendering.
    plot_type: PolarPlotType,
    theme: Theme,
}

impl Default for RadialPlot {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            title: None,
            n_rings: 4,
            n_spokes: 8,
            r_max: None,
            r_min: None,
            theta_direction: ThetaDirection::default(),
            theta_offset: 0.0,
            plot_type: PolarPlotType::default(),
            theme: Theme::get_default(),
        }
    }
}

impl RadialPlot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a series. Data should be (theta, r) pairs where theta is in radians.
    pub fn series(mut self, s: Series) -> Self {
        self.series.push(s);
        self
    }
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }
    pub fn n_rings(mut self, n: usize) -> Self {
        self.n_rings = n;
        self
    }
    pub fn n_spokes(mut self, n: usize) -> Self {
        self.n_spokes = n;
        self
    }
    pub fn r_max(mut self, r: f64) -> Self {
        self.r_max = Some(r);
        self
    }
    /// Set the minimum radial value.
    pub fn r_min(mut self, r: f64) -> Self {
        self.r_min = Some(r);
        self
    }
    /// Set the direction of increasing theta.
    pub fn theta_direction(mut self, dir: ThetaDirection) -> Self {
        self.theta_direction = dir;
        self
    }
    /// Set the theta=0 offset in radians.
    pub fn theta_offset(mut self, offset: f64) -> Self {
        self.theta_offset = offset;
        self
    }
    /// Set the polar plot type.
    pub fn plot_type(mut self, pt: PolarPlotType) -> Self {
        self.plot_type = pt;
        self
    }
    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}


impl Widget for &RadialPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 8 || area.height < 6 {
            return;
        }

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let py = area.y + title_height;
        let ph = area.height.saturating_sub(title_height);
        let pw = area.width;

        let mut pb = create_backend(area);

        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    pb.set_char(x, area.y, ch, self.theme.foreground, Z_CHROME);
                }
            }
        }

        // Center of the plot
        let cx = area.x + pw / 2;
        let cy = py + ph / 2;

        // Radius in screen characters, compensating for ~2:1 terminal cell aspect ratio.
        // Use the smaller of the two radii (adjusted for cell aspect) so circles
        // don't overflow the available space.
        let cell_aspect = crate::axis::terminal_cell_aspect();
        let avail_x = (pw / 2).saturating_sub(2) as f64;
        let avail_y = (ph / 2).saturating_sub(1) as f64;
        let r_data = avail_y.min(avail_x * cell_aspect);
        let r_screen_x = r_data / cell_aspect; // wider to compensate
        let r_screen_y = r_data;

        // Data radius range
        let r_min = self.r_min.unwrap_or(0.0);
        let r_max = self.r_max.unwrap_or_else(|| {
            self.series
                .iter()
                .flat_map(|s| s.data.iter().map(|&(_, r)| r))
                .fold(0.0f64, f64::max)
        });
        let r_max = if r_max == r_min { r_min + 1.0 } else { r_max };

        // Helper: transform theta based on direction and offset
        let transform_theta = |theta: f64| -> f64 {
            let t = theta + self.theta_offset;
            match self.theta_direction {
                ThetaDirection::CounterClockwise => t,
                ThetaDirection::Clockwise => -t,
            }
        };

        // Draw concentric rings using Braille sub-pixel rendering
        for ring in 1..=self.n_rings {
            let r_frac = ring as f64 / self.n_rings as f64;
            let rx = r_frac * r_screen_x;
            let ry = r_frac * r_screen_y;

            // Draw ring as connected Braille line segments
            let n_seg = (2.0 * std::f64::consts::PI * rx.max(ry)).round().max(40.0) as usize;
            for k in 0..n_seg {
                let theta0 = 2.0 * std::f64::consts::PI * k as f64 / n_seg as f64;
                let theta1 = 2.0 * std::f64::consts::PI * (k + 1) as f64 / n_seg as f64;
                let sx0 = cx as f64 + rx * theta0.cos();
                let sy0 = cy as f64 + ry * theta0.sin();
                let sx1 = cx as f64 + rx * theta1.cos();
                let sy1 = cy as f64 + ry * theta1.sin();
                draw_braille_line_clipped_pb(
                    &mut pb,
                    sx0,
                    sy0,
                    sx1,
                    sy1,
                    self.theme.grid_color,
                    Z_GRID,
                    &ClipRect {
                        x: area.x,
                        y: py,
                        w: area.width,
                        h: ph,
                    },
                );
            }

            // Ring label
            let label = format!("{:.1}", r_max * r_frac);
            let lx = cx;
            let ly = (cy as i16 - ry as i16) as u16;
            if ly >= py && ly < py + ph {
                for (j, ch) in label.chars().enumerate() {
                    let x = lx + j as u16 + 1;
                    if x < area.x + area.width {
                        pb.set_char(x, ly, ch, self.theme.axis_color, Z_CHROME);
                    }
                }
            }
        }

        // Draw spokes using Braille sub-pixel rendering
        for spoke in 0..self.n_spokes {
            let theta = 2.0 * std::f64::consts::PI * spoke as f64 / self.n_spokes as f64;
            let dx = r_screen_x * theta.cos();
            let dy = r_screen_y * theta.sin();
            draw_braille_line_clipped_pb(
                &mut pb,
                cx as f64,
                cy as f64,
                cx as f64 + dx,
                cy as f64 + dy,
                self.theme.grid_color,
                Z_GRID,
                &ClipRect {
                    x: area.x,
                    y: py,
                    w: area.width,
                    h: ph,
                },
            );

            // Angle label
            let deg = (theta.to_degrees()).round() as i32;
            let label = format!("{}°", deg);
            let lx = (cx as f64 + (r_screen_x + 2.0) * theta.cos()).round() as u16;
            let ly = (cy as f64 + (r_screen_y + 1.0) * theta.sin()).round() as u16;
            if lx >= area.x
                && lx + label.len() as u16 <= area.x + area.width
                && ly >= py
                && ly < py + ph
            {
                for (j, ch) in label.chars().enumerate() {
                    pb.set_char(lx + j as u16, ly, ch, self.theme.foreground, Z_CHROME);
                }
            }
        }

        // Draw series data
        for (si, s) in self.series.iter().enumerate() {
            let color = s.color.unwrap_or_else(|| self.theme.color_cycle.at(si));
            let mut prev: Option<(u16, u16)> = None;
            // For Bar mode: track previous transformed theta and r_frac for gap filling
            let mut prev_bar: Option<(f64, f64)> = None;
            for &(theta, r) in &s.data {
                let t = transform_theta(theta);
                let r_frac = ((r - r_min) / (r_max - r_min)).clamp(0.0, 1.0);
                let sx = (cx as f64 + r_frac * r_screen_x * t.cos()).round() as u16;
                let sy = (cy as f64 + r_frac * r_screen_y * t.sin()).round() as u16;

                match self.plot_type {
                    PolarPlotType::Scatter => {
                        // Only markers, no lines
                        if sx >= area.x && sx < area.x + area.width && sy >= py && sy < py + ph {
                            let ch = s
                                .marker
                                .map_or(self.theme.chars.marker.default_point, |m| m.char());
                            pb.set_char(sx, sy, ch, color, Z_MARKER);
                        }
                    }
                    PolarPlotType::Bar => {
                        // Radial bar from origin to data point
                        let steps = r_frac * r_screen_y;
                        let n_steps = steps.round().max(1.0) as usize;
                        for step in 0..=n_steps {
                            let frac = step as f64 / n_steps as f64 * r_frac;
                            let bx = (cx as f64 + frac * r_screen_x * t.cos()).round() as u16;
                            let by = (cy as f64 + frac * r_screen_y * t.sin()).round() as u16;
                            if bx >= area.x && bx < area.x + area.width && by >= py && by < py + ph
                            {
                                pb.set_cell(
                                    bx,
                                    by,
                                    self.theme.chars.fill.solid,
                                    color,
                                    color,
                                    Z_DATA,
                                );
                            }
                        }
                        // Fill gap to previous bar by sweeping the arc at each radius level
                        if let Some((prev_t, prev_rf)) = prev_bar {
                            let min_rf = r_frac.min(prev_rf);
                            let outer_r = min_rf * r_screen_y;
                            let n_rad = outer_r.round().max(1.0) as usize;
                            for ri in 1..=n_rad {
                                let frac = ri as f64 / n_rad as f64 * min_rf;
                                let arc_r = frac * r_screen_x.max(r_screen_y);
                                let n_interp = arc_r.round().max(2.0) as usize;
                                for ai in 0..=n_interp {
                                    let a_frac = ai as f64 / n_interp as f64;
                                    let interp_t = prev_t + (t - prev_t) * a_frac;
                                    let bx = (cx as f64 + frac * r_screen_x * interp_t.cos())
                                        .round()
                                        as u16;
                                    let by = (cy as f64 + frac * r_screen_y * interp_t.sin())
                                        .round()
                                        as u16;
                                    if bx >= area.x
                                        && bx < area.x + area.width
                                        && by >= py
                                        && by < py + ph
                                    {
                                        pb.set_cell(
                                            bx,
                                            by,
                                            self.theme.chars.fill.solid,
                                            color,
                                            color,
                                            Z_DATA,
                                        );
                                    }
                                }
                            }
                        }
                        prev_bar = Some((t, r_frac));
                    }
                    PolarPlotType::FillBetween => {
                        // Fill from r_min to r
                        let steps = r_frac * r_screen_y;
                        let n_steps = steps.round().max(1.0) as usize;
                        for step in 0..=n_steps {
                            let frac = step as f64 / n_steps as f64 * r_frac;
                            let bx = (cx as f64 + frac * r_screen_x * t.cos()).round() as u16;
                            let by = (cy as f64 + frac * r_screen_y * t.sin()).round() as u16;
                            if bx >= area.x && bx < area.x + area.width && by >= py && by < py + ph
                            {
                                pb.set_bg(bx, by, color, Z_FILL);
                            }
                        }
                    }
                    PolarPlotType::Line => {
                        // Marker at data point
                        if sx >= area.x && sx < area.x + area.width && sy >= py && sy < py + ph {
                            let ch = s
                                .marker
                                .map_or(self.theme.chars.marker.default_point, |m| m.char());
                            pb.set_char(sx, sy, ch, color, Z_MARKER);
                        }
                        // Connect to previous point using Braille sub-pixel rendering
                        if let Some((px, py_prev)) = prev
                            && (sx != px || sy != py_prev)
                        {
                            draw_braille_line_clipped_pb(
                                &mut pb,
                                px as f64,
                                py_prev as f64,
                                sx as f64,
                                sy as f64,
                                color,
                                Z_DATA,
                                &ClipRect {
                                    x: area.x,
                                    y: py,
                                    w: area.width,
                                    h: ph,
                                },
                            );
                        }
                    }
                }
                prev = Some((sx, sy));
            }
        }

        pb.composite(buf);
    }
}

/// Clipping rectangle for Braille line drawing.
struct ClipRect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

/// Draw a Braille sub-pixel line clipped to a rectangular region, writing into a [`PlotBuffer`].
///
/// Coordinates are in terminal cell space (floating point). The line is
/// rendered at 2x4 sub-pixel resolution using Unicode Braille characters.
#[allow(clippy::too_many_arguments)]
fn draw_braille_line_clipped_pb(
    pb: &mut dyn PlotBackend,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: ratatui::style::Color,
    z: u8,
    clip: &ClipRect,
) {
    let mut ix0 = (x0 * 2.0).round() as i32;
    let mut iy0 = (y0 * 4.0).round() as i32;
    let ix1 = (x1 * 2.0).round() as i32;
    let iy1 = (y1 * 4.0).round() as i32;

    let dx = (ix1 - ix0).abs();
    let dy = -(iy1 - iy0).abs();
    let sx = if ix0 < ix1 { 1 } else { -1 };
    let sy = if iy0 < iy1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if ix0 >= 0 && iy0 >= 0 {
            let cell_x = (ix0 / 2) as u16;
            let cell_y = (iy0 / 4) as u16;
            if cell_x >= clip.x
                && cell_x < clip.x + clip.w
                && cell_y >= clip.y
                && cell_y < clip.y + clip.h
            {
                let dot_col = (ix0 % 2) as usize;
                let dot_row = (iy0 % 4) as usize;
                let bit = BRAILLE_BITS[dot_col][dot_row];
                pb.set_braille(cell_x, cell_y, bit, color, z);
            }
        }

        if ix0 == ix1 && iy0 == iy1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            ix0 += sx;
        }
        if e2 <= dx {
            err += dx;
            iy0 += sy;
        }
    }
}

/// Type alias for [`RadialPlot`] when used for radar/spider charts.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use std::f64::consts::TAU;
///
/// // 5-axis radar chart
/// let n = 5;
/// let mut data: Vec<(f64, f64)> = (0..n)
///     .map(|i| (i as f64 * TAU / n as f64, (i as f64 + 1.0) * 0.2))
///     .collect();
/// // Close the polygon by appending the first point
/// data.push(data[0]);
///
/// let radar = RadarPlot::new()
///     .series(Series::new("Scores").data(data).color(Color::Cyan))
///     .plot_type(PolarPlotType::FillBetween)
///     .n_spokes(n)
///     .title("Radar Chart");
/// ```
pub type RadarPlot = RadialPlot;
