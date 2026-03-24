//! Heatmap widget with colormap and normalization support.
//!
//! Renders a 2D grid using half-block characters for doubled vertical resolution.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis};
use crate::colormap::{Colorbar, Colormap, Viridis};
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::norm::{LinearNorm, Normalize};
use crate::plot_buffer::{PlotBuffer, Z_ANNOTATION, Z_DATA};
use crate::series::GridData;
use crate::spines::Spines;
use crate::theme::Theme;

/// A 2D heatmap widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
///
/// let data = GridData::from_fn((-2.0, 2.0), (-2.0, 2.0), 50, 50, |x, y| {
///     (-(x * x + y * y)).exp()
/// });
/// let heatmap = Heatmap::new(data)
///     .colormap(Viridis)
///     .title("2D Gaussian")
///     .aspect_ratio(AspectRatio::Equal);
/// ```
pub struct Heatmap {
    data: GridData,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_colorbar: bool,
    aspect_ratio: AspectRatio,
    show_values: bool,
    /// Color used for NaN/invalid cells.
    bad_color: Color,
    /// Optional boolean mask. When set, cells where `mask[row][col]` is `true`
    /// are rendered using [`bad_color`] (useful for triangular correlation matrices).
    mask: Option<Vec<Vec<bool>>>,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Heatmap {
    /// Create a heatmap from grid data.
    pub fn new(data: GridData) -> Self {
        let (vmin, vmax) = data.value_bounds();
        Self {
            data,
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_colorbar: true,
            aspect_ratio: AspectRatio::Auto,
            show_values: false,
            bad_color: Theme::get_default().bad_data_color,
            mask: None,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
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

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Show or hide the colorbar.
    pub fn show_colorbar(mut self, show: bool) -> Self {
        self.show_colorbar = show;
        self
    }

    /// Set the aspect ratio.
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
        self
    }

    /// Show values in cells (only useful for small grids).
    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Set the color used for NaN/invalid cells.
    pub fn bad_color(mut self, color: Color) -> Self {
        self.bad_color = color;
        self
    }

    /// Set a boolean mask. Cells where `mask[row][col]` is `true` are
    /// rendered as [`bad_color`] instead of the data value. This is useful
    /// for triangular correlation matrices where the upper or lower triangle
    /// should be hidden.
    pub fn mask(mut self, mask: Vec<Vec<bool>>) -> Self {
        self.mask = Some(mask);
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

impl Widget for &Heatmap {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let nrows = self.data.nrows();
        let ncols = self.data.ncols();
        if nrows == 0 || ncols == 0 || self.data.values.is_empty() || self.data.values[0].is_empty()
        {
            return;
        }

        let x_lo = *self.data.x.first().unwrap_or(&0.0);
        let x_hi = *self.data.x.last().unwrap_or(&1.0);
        let y_lo = *self.data.y.first().unwrap_or(&0.0);
        let y_hi = *self.data.y.last().unwrap_or(&1.0);

        let colorbar_width: u16 = if self.show_colorbar { 10 } else { 0 };

        let mut pb = PlotBuffer::new(area);

        // Create and render the plot frame (title, axes, grid, ticks, labels, spines, ref lines)
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .aspect_ratio(self.aspect_ratio.clone())
            .spines(self.spines.clone())
            .colorbar_width(colorbar_width)
            .y_label_width(7)
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

        let px = pa.x;
        let py = pa.y;
        let aw = pa.width;
        let ah = pa.height;

        // Use half-block rendering: each character cell encodes two vertical pixels
        // ▀ = top half, ▄ = bottom half, █ = both same color
        // We double the effective vertical resolution.
        let effective_height = ah as usize * 2;

        for cy in 0..ah {
            for cx in 0..aw {
                let screen_x = px + cx;
                let screen_y = py + cy;
                if screen_x >= area.x + area.width || screen_y >= area.y + area.height {
                    continue;
                }

                // Top half-pixel
                let top_row_f = (cy as usize * 2) as f64 / effective_height as f64;
                let top_data_row =
                    ((1.0 - top_row_f) * nrows as f64).min((nrows - 1) as f64) as usize;
                let top_data_col =
                    (cx as f64 / aw as f64 * ncols as f64).min((ncols - 1) as f64) as usize;
                let top_val = self.data.values[top_data_row][top_data_col];
                let top_masked = self.mask.as_ref().is_some_and(|m| {
                    top_data_row < m.len()
                        && top_data_col < m[top_data_row].len()
                        && m[top_data_row][top_data_col]
                });
                let top_color = if top_masked || !top_val.is_finite() {
                    self.bad_color
                } else {
                    let top_t = self.norm.normalize(top_val);
                    self.colormap.color_at(top_t)
                };

                // Bottom half-pixel
                let bot_row_f = (cy as usize * 2 + 1) as f64 / effective_height as f64;
                let bot_data_row =
                    ((1.0 - bot_row_f) * nrows as f64).min((nrows - 1) as f64) as usize;
                let bot_data_col = top_data_col;
                let bot_val = self.data.values[bot_data_row][bot_data_col];
                let bot_masked = self.mask.as_ref().is_some_and(|m| {
                    bot_data_row < m.len()
                        && bot_data_col < m[bot_data_row].len()
                        && m[bot_data_row][bot_data_col]
                });
                let bot_color = if bot_masked || !bot_val.is_finite() {
                    self.bad_color
                } else {
                    let bot_t = self.norm.normalize(bot_val);
                    self.colormap.color_at(bot_t)
                };

                // Use ▀ (upper half block): fg = top color, bg = bottom color
                pb.set_cell(screen_x, screen_y, '▀', top_color, bot_color, Z_DATA);
            }
        }

        // Draw cell values if enabled and cells are wide enough
        if self.show_values {
            let cell_width = aw as f64 / ncols as f64;
            let cell_height = ah as f64 / nrows as f64;

            if cell_width >= 4.0 {
                for row in 0..nrows {
                    for col in 0..ncols {
                        let val = self.data.values[row][col];
                        if !val.is_finite() {
                            continue;
                        }
                        // Skip masked cells
                        let is_masked = self
                            .mask
                            .as_ref()
                            .is_some_and(|m| row < m.len() && col < m[row].len() && m[row][col]);
                        if is_masked {
                            continue;
                        }

                        let label = format!("{:.1}", val);

                        // Compute center screen position for this cell
                        // Row 0 is at the top of data but bottom of screen (y inverted)
                        let center_x = px as f64 + (col as f64 + 0.5) * cell_width;
                        let center_y = py as f64 + ((nrows - 1 - row) as f64 + 0.5) * cell_height;

                        let xi = center_x.round() as u16;
                        let yi = center_y.round() as u16;

                        // Determine contrasting text color based on cell luminance
                        let t = self.norm.normalize(val);
                        let fg_color = match self.colormap.color_at(t) {
                            Color::Rgb(r, g, b) => {
                                let luminance =
                                    (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000;
                                if luminance > 128 {
                                    Color::Black
                                } else {
                                    Color::White
                                }
                            }
                            _ => Color::White,
                        };

                        // Center the label horizontally within the cell
                        let label_start = xi.saturating_sub(label.len() as u16 / 2);
                        if yi >= py && yi < py + ah {
                            for (j, ch) in label.chars().enumerate() {
                                let lx = label_start + j as u16;
                                if lx >= px && lx < px + aw {
                                    pb.set_char(lx, yi, ch, fg_color, Z_ANNOTATION);
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

        // Draw colorbar (after composite, as it manages its own rendering)
        if self.show_colorbar {
            let (vmin, vmax) = self.data.value_bounds();
            let cb = Colorbar::new(self.colormap.as_ref(), vmin, vmax)
                .label_color(self.theme.foreground);
            let cb_area = Rect::new(
                px + aw + 2,
                py,
                colorbar_width.min(area.x + area.width - px - aw - 2),
                ah,
            );
            if cb_area.x + cb_area.width <= area.x + area.width {
                (&cb).render(cb_area, buf);
            }
        }
    }
}
