//! Parallel coordinates plot widget.
//!
//! Visualizes multivariate data by drawing multiple vertical axes (one per
//! variable) and connecting each record's values with a polyline. Useful for
//! exploring correlations and clusters in high-dimensional data.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::parallel_coords::{ParallelAxis, ParallelRecord};
//!
//! let plot = ParallelCoords::new()
//!     .axes(vec![
//!         ParallelAxis::new("Speed", 0.0, 100.0),
//!         ParallelAxis::new("Power", 0.0, 500.0),
//!         ParallelAxis::new("Weight", 1000.0, 3000.0),
//!     ])
//!     .record(ParallelRecord::new(vec![60.0, 300.0, 1500.0]).color(Color::Cyan).name("Car A"))
//!     .record(ParallelRecord::new(vec![80.0, 450.0, 2000.0]).color(Color::Red).name("Car B"))
//!     .title("Vehicle Comparison");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::drawing::draw_braille_line_pb;
use crate::frame::PlotArea;
use crate::legend::{Legend, LegendEntry, LegendPosition};
use crate::plot_buffer::{PlotBuffer, Z_ANNOTATION, Z_CHROME, Z_DATA, Z_MARKER};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::transform::data_to_screen;

/// A single vertical axis in the parallel coordinates plot.
#[derive(Clone, Debug)]
pub struct ParallelAxis {
    /// Axis label.
    pub name: String,
    /// Minimum data value on this axis.
    pub min: f64,
    /// Maximum data value on this axis.
    pub max: f64,
}

impl ParallelAxis {
    /// Create a new axis with name and data range.
    pub fn new(name: impl Into<String>, min: f64, max: f64) -> Self {
        Self {
            name: name.into(),
            min,
            max,
        }
    }
}

/// A single data record (one polyline) in the parallel coordinates plot.
#[derive(Clone, Debug)]
pub struct ParallelRecord {
    /// One value per axis.
    pub values: Vec<f64>,
    /// Line color.
    pub color: Color,
    /// Optional display name for the legend.
    pub name: Option<String>,
}

impl ParallelRecord {
    /// Create a new record with the given values (one per axis).
    pub fn new(values: Vec<f64>) -> Self {
        Self {
            values,
            color: Color::White,
            name: None,
        }
    }

    /// Set the line color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the display name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// A parallel coordinates plot widget.
///
/// Multiple vertical axes are drawn equally spaced. Each record is a polyline
/// connecting its values on each axis.
///
/// Note: this widget does not use the standard `x_axis`/`y_axis` fields since
/// it manages multiple vertical axes internally. The `PlotFrame` is not used
/// for axis chrome; layout is handled manually.
#[derive(Clone)]
pub struct ParallelCoords {
    axes: Vec<ParallelAxis>,
    records: Vec<ParallelRecord>,
    title: Option<String>,
    show_legend: bool,
    legend_position: LegendPosition,
    theme: Theme,
    spines: Spines,
    annotations: Vec<Annotation>,
}

impl Default for ParallelCoords {
    fn default() -> Self {
        Self {
            axes: Vec::new(),
            records: Vec::new(),
            title: None,
            show_legend: true,
            legend_position: LegendPosition::TopRight,
            theme: Theme::get_default(),
            spines: Spines::default(),
            annotations: Vec::new(),
        }
    }
}

impl ParallelCoords {
    /// Create an empty parallel coordinates plot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the axes.
    pub fn axes(mut self, axes: Vec<ParallelAxis>) -> Self {
        self.axes = axes;
        self
    }

    /// Add a single axis.
    pub fn axis(mut self, axis: ParallelAxis) -> Self {
        self.axes.push(axis);
        self
    }

    /// Add a single record.
    pub fn record(mut self, rec: ParallelRecord) -> Self {
        self.records.push(rec);
        self
    }

    /// Set all records at once.
    pub fn records(mut self, records: Vec<ParallelRecord>) -> Self {
        self.records = records;
        self
    }

    /// Set the plot title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Show or hide the legend.
    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    /// Set the legend position.
    pub fn legend_position(mut self, pos: LegendPosition) -> Self {
        self.legend_position = pos;
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

    /// Add an annotation.
    pub fn annotation(mut self, ann: Annotation) -> Self {
        self.annotations.push(ann);
        self
    }
}

impl Widget for &ParallelCoords {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let n_axes = self.axes.len();
        if n_axes < 2 || area.width < 10 || area.height < 6 {
            return;
        }

        let mut pb = PlotBuffer::new(area);

        // Layout: title row, axis label row, plot area, tick label row
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let label_row = area.y + title_height; // axis names
        let plot_top = label_row + 1;
        let tick_row = area.y + area.height - 1; // bottom tick labels
        let plot_bottom = tick_row.saturating_sub(1);

        if plot_bottom <= plot_top {
            return;
        }
        let plot_height = plot_bottom - plot_top;

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

        // Compute horizontal positions for each axis
        let margin: u16 = 4; // left/right margin for labels
        let usable_width = area.width.saturating_sub(margin * 2);
        if usable_width < (n_axes as u16) {
            return;
        }

        let axis_positions: Vec<u16> = (0..n_axes)
            .map(|i| {
                let t = i as f64 / (n_axes as f64 - 1.0);
                area.x + margin + (t * (usable_width.saturating_sub(1)) as f64).round() as u16
            })
            .collect();

        // Draw vertical axes and labels
        for (i, ax) in self.axes.iter().enumerate() {
            let ax_x = axis_positions[i];

            // Axis label at top
            let label_start = ax_x.saturating_sub(ax.name.len() as u16 / 2);
            for (j, ch) in ax.name.chars().enumerate() {
                let x = label_start + j as u16;
                if x >= area.x && x < area.x + area.width {
                    pb.set_char(x, label_row, ch, self.theme.foreground, Z_CHROME);
                }
            }

            // Vertical axis line
            let axis_color = if self.spines.left || self.spines.right || i == 0 || i == n_axes - 1 {
                self.theme.axis_color
            } else {
                self.theme.grid_color
            };
            for y in plot_top..=plot_bottom {
                if ax_x >= area.x && ax_x < area.x + area.width {
                    pb.set_char(ax_x, y, '\u{2502}', axis_color, Z_CHROME); // │
                }
            }

            // Min/max tick labels
            let min_label = format_compact(ax.min);
            let max_label = format_compact(ax.max);

            // Max at top of axis
            let max_start = ax_x.saturating_sub(max_label.len() as u16 / 2);
            let max_row = plot_top;
            for (j, ch) in max_label.chars().enumerate() {
                let x = max_start + j as u16;
                if x >= area.x && x < area.x + area.width && max_row >= area.y {
                    pb.set_char(x, max_row, ch, self.theme.axis_color, Z_CHROME);
                }
            }

            // Min at bottom of axis
            let min_start = ax_x.saturating_sub(min_label.len() as u16 / 2);
            for (j, ch) in min_label.chars().enumerate() {
                let x = min_start + j as u16;
                if x >= area.x && x < area.x + area.width && tick_row < area.y + area.height {
                    pb.set_char(x, tick_row, ch, self.theme.axis_color, Z_CHROME);
                }
            }
        }

        // Build a PlotArea covering the plot region so draw_braille_line_pb can clip.
        let pa = PlotArea {
            x: area.x,
            y: plot_top,
            width: area.width,
            height: plot_bottom - plot_top + 1,
            x_lo: 0.0,
            x_hi: 0.0,
            y_lo: 0.0,
            y_hi: 0.0,
            area,
        };

        // Draw polylines for each record using Braille line drawing
        for (si, rec) in self.records.iter().enumerate() {
            let n_values = rec.values.len().min(n_axes);
            if n_values < 2 {
                continue;
            }

            for i in 0..n_values - 1 {
                let v0 = rec.values[i];
                let v1 = rec.values[i + 1];

                if !v0.is_finite() || !v1.is_finite() {
                    continue;
                }

                let ax0 = &self.axes[i];
                let ax1 = &self.axes[i + 1];

                let sx0 = axis_positions[i] as f64;
                let sy0 = data_to_screen(v0, ax0.min, ax0.max, plot_bottom as f64, plot_top as f64);

                let sx1 = axis_positions[i + 1] as f64;
                let sy1 = data_to_screen(v1, ax1.min, ax1.max, plot_bottom as f64, plot_top as f64);

                draw_braille_line_pb(
                    &mut pb,
                    sx0,
                    sy0,
                    sx1,
                    sy1,
                    rec.color,
                    &pa,
                    Z_DATA + si as u8,
                );
            }

            // Draw value markers on each axis
            for (i, (&v, ax_x)) in rec
                .values
                .iter()
                .zip(axis_positions.iter())
                .take(n_values)
                .enumerate()
            {
                if !v.is_finite() {
                    continue;
                }
                let ax = &self.axes[i];
                let sy = data_to_screen(v, ax.min, ax.max, plot_bottom as f64, plot_top as f64)
                    .round() as u16;

                if sy >= plot_top
                    && sy <= plot_bottom
                    && *ax_x >= area.x
                    && *ax_x < area.x + area.width
                {
                    pb.set_char(*ax_x, sy, '\u{25CF}', rec.color, Z_MARKER); // ●
                }
            }
        }

        // Draw annotations (manual placement since we don't have a PlotArea)
        for ann in &self.annotations {
            // Interpret text_x as screen-relative x, text_y as screen-relative y
            let xi = (area.x as f64 + ann.text_x).round() as u16;
            let yi = (area.y as f64 + ann.text_y).round() as u16;
            if yi >= area.y && yi < area.y + area.height {
                for (j, ch) in ann.text.chars().enumerate() {
                    let x = xi + j as u16;
                    if x >= area.x && x < area.x + area.width {
                        pb.set_char(
                            x,
                            yi,
                            ch,
                            ann.color.unwrap_or(self.theme.foreground),
                            Z_ANNOTATION,
                        );
                    }
                }
            }
        }

        // Composite to buffer before legend
        pb.composite(buf);

        // Draw legend (directly to buf, after composite)
        if self.show_legend {
            let named: Vec<&ParallelRecord> =
                self.records.iter().filter(|r| r.name.is_some()).collect();
            if !named.is_empty() {
                let entries: Vec<LegendEntry> = named
                    .iter()
                    .map(|r| LegendEntry {
                        name: r.name.clone().unwrap_or_default(),
                        color: r.color,
                        marker: Some('\u{2501}'), // ━
                    })
                    .collect();
                let legend = Legend::new(entries)
                    .position(self.legend_position.clone())
                    .theme(self.theme.clone());
                let legend_area = Rect::new(area.x + margin, plot_top, usable_width, plot_height);
                (&legend).render(legend_area, buf);
            }
        }
    }
}

/// Format a number compactly for tick labels.
fn format_compact(v: f64) -> String {
    if v.abs() >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v.abs() >= 1_000.0 {
        format!("{:.1}k", v / 1_000.0)
    } else if v.abs() < 0.01 && v != 0.0 {
        format!("{:.2e}", v)
    } else if v == v.floor() {
        format!("{:.0}", v)
    } else {
        format!("{:.1}", v)
    }
}
