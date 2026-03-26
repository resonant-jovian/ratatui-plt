//! Contour plot widget using marching squares.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis};
use crate::colormap::{Colormap, Viridis};
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::norm::{LinearNorm, Normalize};
use crate::plot_buffer::{PlotBuffer, Z_ANNOTATION, Z_DATA};
use crate::series::GridData;
use crate::spines::Spines;
use crate::theme::Theme;

/// A contour plot widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 40, 40, |x, y| {
///     (-(x*x + y*y) / 2.0).exp()
/// });
/// let plot = ContourPlot::new(data).levels(10).title("Gaussian");
/// ```
pub struct ContourPlot {
    data: GridData,
    levels: Vec<f64>,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    aspect_ratio: AspectRatio,
    filled: bool,
    show_labels: bool,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl ContourPlot {
    /// Create a contour plot from grid data with auto-computed levels.
    pub fn new(data: GridData) -> Self {
        let (vmin, vmax) = data.value_bounds();
        Self {
            data,
            levels: Vec::new(),
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            aspect_ratio: AspectRatio::Auto,
            filled: false,
            show_labels: false,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }

    /// Set the number of contour levels (auto-spaced).
    pub fn levels(mut self, n: usize) -> Self {
        let (vmin, vmax) = self.data.value_bounds();
        self.levels = (0..n)
            .map(|i| vmin + (vmax - vmin) * (i as f64 + 0.5) / n as f64)
            .collect();
        self
    }

    /// Set explicit contour level values.
    pub fn level_values(mut self, levels: Vec<f64>) -> Self {
        self.levels = levels;
        self
    }

    /// Set the colormap.
    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    /// Set the normalization.
    pub fn norm(mut self, norm: impl Normalize + 'static) -> Self {
        self.norm = Box::new(norm);
        self
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
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

    /// Set the aspect ratio.
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
        self
    }

    /// Enable filled contours (contourf-style).
    pub fn filled(mut self, f: bool) -> Self {
        self.filled = f;
        self
    }

    /// Show level labels on contour lines.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
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
}

impl Widget for &ContourPlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let nrows = self.data.nrows();
        let ncols = self.data.ncols();
        if nrows < 2 || ncols < 2 {
            return;
        }

        let x_lo = self.data.x[0];
        let x_hi = self.data.x[ncols - 1];
        let y_lo = self.data.y[0];
        let y_hi = self.data.y[nrows - 1];

        let mut pb = PlotBuffer::new(area);

        // Create and render the plot frame (title, axes, grid, ticks, labels, spines, ref lines)
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .aspect_ratio(self.aspect_ratio.clone())
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

        let levels = if self.levels.is_empty() {
            let (vmin, vmax) = self.data.value_bounds();
            (0..8)
                .map(|i| vmin + (vmax - vmin) * (i as f64 + 0.5) / 8.0)
                .collect::<Vec<_>>()
        } else {
            self.levels.clone()
        };

        // Filled contours: half-block rendering with bilinear interpolation
        if self.filled {
            let virt_h = pa.height as f64 * 2.0;
            for cy in 0..pa.height {
                for cx in 0..pa.width {
                    let data_x = x_lo + (cx as f64 / (pa.width - 1).max(1) as f64) * (x_hi - x_lo);

                    let sample_color = |vy: f64| -> Color {
                        let data_y = y_hi - (vy / (virt_h - 1.0).max(1.0)) * (y_hi - y_lo);
                        let gx = ((data_x - x_lo) / (x_hi - x_lo) * (ncols - 1) as f64)
                            .clamp(0.0, (ncols - 1) as f64);
                        let gy = ((data_y - y_lo) / (y_hi - y_lo) * (nrows - 1) as f64)
                            .clamp(0.0, (nrows - 1) as f64);
                        let ix = (gx.floor() as usize).min(ncols - 2);
                        let iy = (gy.floor() as usize).min(nrows - 2);
                        let fx = gx - ix as f64;
                        let fy = gy - iy as f64;
                        let val = self.data.values[iy][ix] * (1.0 - fx) * (1.0 - fy)
                            + self.data.values[iy][ix + 1] * fx * (1.0 - fy)
                            + self.data.values[iy + 1][ix] * (1.0 - fx) * fy
                            + self.data.values[iy + 1][ix + 1] * fx * fy;
                        let band = levels.partition_point(|&l| l <= val);
                        let t = band as f64 / levels.len() as f64;
                        self.colormap.color_at(t)
                    };

                    let top = sample_color(cy as f64 * 2.0);
                    let bot = sample_color(cy as f64 * 2.0 + 1.0);

                    let sx = pa.x + cx;
                    let sy = pa.y + cy;
                    if pa.in_area(sx, sy) {
                        pb.set_cell(sx, sy, self.theme.chars.fill.half_upper, top, bot, Z_DATA);
                    }
                }
            }
        }

        // Draw contour lines (skip when filled -- bands already show levels)
        if !self.filled {
            // Track first segment midpoint for each level (for label placement)
            let mut level_label_positions: Vec<Option<(f64, f64)>> = vec![None; levels.len()];

            // Edge naming: top=v00-v10, right=v10-v11, bottom=v01-v11, left=v00-v01
            // Corners: v00=top-left(j,i), v10=top-right(j,i+1), v01=bottom-left(j+1,i), v11=bottom-right(j+1,i+1)
            for (level_idx, &level) in levels.iter().enumerate() {
                let t = self.norm.normalize(level);
                let color = self.colormap.color_at(t);

                for j in 0..nrows - 1 {
                    for i in 0..ncols - 1 {
                        let v00 = self.data.values[j][i];
                        let v10 = self.data.values[j][i + 1];
                        let v01 = self.data.values[j + 1][i];
                        let v11 = self.data.values[j + 1][i + 1];

                        let case = ((v00 >= level) as u8)
                            | (((v10 >= level) as u8) << 1)
                            | (((v01 >= level) as u8) << 2)
                            | (((v11 >= level) as u8) << 3);

                        if case == 0 || case == 15 {
                            continue;
                        }

                        // Interpolation fraction along an edge
                        let interp = |va: f64, vb: f64| -> f64 {
                            if (vb - va).abs() < 1e-12 {
                                0.5
                            } else {
                                (level - va) / (vb - va)
                            }
                        };

                        // Data coordinates of the four corners
                        let x0 = self.data.x[i];
                        let x1 = self.data.x[i + 1];
                        let y0 = self.data.y[j];
                        let y1 = self.data.y[j + 1];

                        // Edge crossing points in data coordinates
                        let top_edge = || {
                            let f = interp(v00, v10);
                            (x0 + f * (x1 - x0), y0)
                        };
                        let bottom_edge = || {
                            let f = interp(v01, v11);
                            (x0 + f * (x1 - x0), y1)
                        };
                        let left_edge = || {
                            let f = interp(v00, v01);
                            (x0, y0 + f * (y1 - y0))
                        };
                        let right_edge = || {
                            let f = interp(v10, v11);
                            (x1, y0 + f * (y1 - y0))
                        };

                        // Collect line segments for this cell
                        let segments: Vec<((f64, f64), (f64, f64))> = match case {
                            1 | 14 => vec![(top_edge(), left_edge())],
                            2 | 13 => vec![(top_edge(), right_edge())],
                            3 | 12 => vec![(left_edge(), right_edge())],
                            4 | 11 => vec![(bottom_edge(), left_edge())],
                            5 => vec![(top_edge(), left_edge()), (bottom_edge(), right_edge())],
                            6 | 9 => vec![(top_edge(), bottom_edge())],
                            7 | 8 => vec![(bottom_edge(), right_edge())],
                            10 => vec![(top_edge(), right_edge()), (bottom_edge(), left_edge())],
                            _ => vec![],
                        };

                        // Record the midpoint of the first segment for label placement
                        if self.show_labels
                            && level_label_positions[level_idx].is_none()
                            && let Some(&((dx0, dy0), (dx1, dy1))) = segments.first()
                        {
                            level_label_positions[level_idx] =
                                Some(((dx0 + dx1) / 2.0, (dy0 + dy1) / 2.0));
                        }

                        // Draw each segment using Braille sub-pixel rendering
                        for ((dx0, dy0), (dx1, dy1)) in segments {
                            let sx0 = pa.screen_x(dx0);
                            let sy0 = pa.screen_y(dy0);
                            let sx1 = pa.screen_x(dx1);
                            let sy1 = pa.screen_y(dy1);

                            pb.draw_line(
                                sx0,
                                sy0,
                                sx1,
                                sy1,
                                color,
                                &pa,
                                Z_DATA + level_idx as u8,
                            );
                        }
                    }
                }
            }

            // Draw contour level labels
            if self.show_labels {
                for (level_idx, &level) in levels.iter().enumerate() {
                    if let Some((lx, ly)) = level_label_positions[level_idx] {
                        let label = format!("{:.2}", level);
                        let sx = pa.screen_x(lx).round() as u16;
                        let sy = pa.screen_y(ly).round() as u16;
                        let t = self.norm.normalize(level);
                        let color = self.colormap.color_at(t);
                        if pa.contains(sx, sy) {
                            for (j, ch) in label.chars().enumerate() {
                                let cx = sx + j as u16;
                                if pa.contains(cx, sy) {
                                    pb.set_char(cx, sy, ch, color, Z_ANNOTATION);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);
    }
}
