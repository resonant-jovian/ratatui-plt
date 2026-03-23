//! Image display widget (imshow) with colormap, normalization, and RGB support.
//!
//! Renders a 2D matrix as an image using half-block characters for doubled vertical
//! resolution. Supports scalar data mapped through colormaps, direct RGB, and RGBA
//! color matrices. Includes `spy()` and `matshow()` convenience constructors.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::{AspectRatio, Axis, Bounds};
use crate::colormap::{Colorbar, Colormap, Viridis};
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::norm::{LinearNorm, Normalize};
use crate::plot_buffer::{PlotBuffer, Z_DATA};
use crate::spines::Spines;
use crate::theme::Theme;

/// Image data source.
///
/// Supports three modes: scalar data mapped through a colormap, direct RGB
/// colors, and RGBA colors (alpha is ignored in the terminal).
#[derive(Clone, Debug)]
pub enum ImageData {
    /// Scalar matrix mapped through colormap + normalization.
    Scalar(Vec<Vec<f64>>),
    /// RGB matrix: each cell is (r, g, b) in 0-255.
    Rgb(Vec<Vec<(u8, u8, u8)>>),
    /// RGBA matrix: each cell is (r, g, b, a) in 0-255.
    Rgba(Vec<Vec<(u8, u8, u8, u8)>>),
}

impl ImageData {
    /// Number of rows in the image.
    pub fn nrows(&self) -> usize {
        match self {
            Self::Scalar(data) => data.len(),
            Self::Rgb(data) => data.len(),
            Self::Rgba(data) => data.len(),
        }
    }

    /// Number of columns in the image.
    pub fn ncols(&self) -> usize {
        match self {
            Self::Scalar(data) => data.first().map_or(0, |r| r.len()),
            Self::Rgb(data) => data.first().map_or(0, |r| r.len()),
            Self::Rgba(data) => data.first().map_or(0, |r| r.len()),
        }
    }
}

/// Interpolation method for mapping data pixels to screen pixels.
#[derive(Clone, Debug, Default)]
pub enum Interpolation {
    /// Nearest-neighbor: each screen pixel takes the value of the closest data pixel.
    #[default]
    Nearest,
    /// Bilinear: interpolate between the 4 nearest data pixels.
    Bilinear,
}

/// Y-axis origin convention.
#[derive(Clone, Debug, Default)]
pub enum ImageOrigin {
    /// Row 0 at the top (image convention, default).
    #[default]
    Upper,
    /// Row 0 at the bottom (mathematical convention).
    Lower,
}

/// Image display widget.
///
/// Renders a 2D matrix as an image using half-block characters for doubled
/// vertical resolution. Supports scalar data (mapped through colormaps),
/// direct RGB, and RGBA data.
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::widgets::image_plot::{ImageData, ImagePlot};
///
/// let data: Vec<Vec<f64>> = (0..10)
///     .map(|r| (0..10).map(|c| (r as f64 - 5.0).powi(2) + (c as f64 - 5.0).powi(2)).collect())
///     .collect();
/// let img = ImagePlot::new(ImageData::Scalar(data))
///     .colormap(Viridis)
///     .title("2D Gaussian")
///     .show_colorbar(true);
/// ```
pub struct ImagePlot {
    data: ImageData,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    interpolation: Interpolation,
    aspect_ratio: AspectRatio,
    origin: ImageOrigin,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_colorbar: bool,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl ImagePlot {
    /// Create an image plot from image data.
    ///
    /// Defaults: Viridis colormap, LinearNorm (auto-ranged for scalar data),
    /// nearest interpolation, equal aspect ratio, upper origin.
    pub fn new(data: ImageData) -> Self {
        let (vmin, vmax) = scalar_value_bounds(&data);
        Self {
            data,
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            interpolation: Interpolation::Nearest,
            aspect_ratio: AspectRatio::Equal,
            origin: ImageOrigin::Upper,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            show_colorbar: true,
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

    /// Set the interpolation method.
    pub fn interpolation(mut self, interp: Interpolation) -> Self {
        self.interpolation = interp;
        self
    }

    /// Set the aspect ratio.
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.aspect_ratio = ar;
        self
    }

    /// Set the y-axis origin convention.
    pub fn origin(mut self, origin: ImageOrigin) -> Self {
        self.origin = origin;
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

    /// Show or hide the colorbar (only applies to scalar data).
    pub fn show_colorbar(mut self, show: bool) -> Self {
        self.show_colorbar = show;
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

    /// Add an annotation.
    pub fn annotation(mut self, ann: Annotation) -> Self {
        self.annotations.push(ann);
        self
    }

    /// Sample the color for a data pixel using nearest-neighbor lookup.
    fn sample_color(&self, row: usize, col: usize) -> Color {
        match &self.data {
            ImageData::Scalar(data) => {
                let val = data[row][col];
                if !val.is_finite() {
                    return self.theme.background;
                }
                let t = self.norm.normalize(val);
                self.colormap.color_at(t)
            }
            ImageData::Rgb(data) => {
                let (r, g, b) = data[row][col];
                Color::Rgb(r, g, b)
            }
            ImageData::Rgba(data) => {
                let (r, g, b, _a) = data[row][col];
                Color::Rgb(r, g, b) // terminal doesn't support alpha
            }
        }
    }

    /// Sample the color for a data position using bilinear interpolation.
    fn sample_color_bilinear(&self, row_f: f64, col_f: f64, nrows: usize, ncols: usize) -> Color {
        if nrows == 0 || ncols == 0 {
            return self.theme.background;
        }

        let r0 = (row_f.floor() as usize).min(nrows.saturating_sub(1));
        let r1 = (r0 + 1).min(nrows.saturating_sub(1));
        let c0 = (col_f.floor() as usize).min(ncols.saturating_sub(1));
        let c1 = (c0 + 1).min(ncols.saturating_sub(1));

        let fr = row_f - row_f.floor();
        let fc = col_f - col_f.floor();

        match &self.data {
            ImageData::Scalar(data) => {
                let v00 = data[r0][c0];
                let v01 = data[r0][c1];
                let v10 = data[r1][c0];
                let v11 = data[r1][c1];
                // If any neighbor is non-finite, fall back to nearest
                if !v00.is_finite() || !v01.is_finite() || !v10.is_finite() || !v11.is_finite() {
                    let nearest_r =
                        if fr < 0.5 { r0 } else { r1 };
                    let nearest_c =
                        if fc < 0.5 { c0 } else { c1 };
                    return self.sample_color(nearest_r, nearest_c);
                }
                let val = v00 * (1.0 - fr) * (1.0 - fc)
                    + v01 * (1.0 - fr) * fc
                    + v10 * fr * (1.0 - fc)
                    + v11 * fr * fc;
                let t = self.norm.normalize(val);
                self.colormap.color_at(t)
            }
            ImageData::Rgb(data) => {
                let (r00, g00, b00) = data[r0][c0];
                let (r01, g01, b01) = data[r0][c1];
                let (r10, g10, b10) = data[r1][c0];
                let (r11, g11, b11) = data[r1][c1];
                let interp_channel = |a: u8, b: u8, c: u8, d: u8| -> u8 {
                    let val = a as f64 * (1.0 - fr) * (1.0 - fc)
                        + b as f64 * (1.0 - fr) * fc
                        + c as f64 * fr * (1.0 - fc)
                        + d as f64 * fr * fc;
                    val.round().clamp(0.0, 255.0) as u8
                };
                Color::Rgb(
                    interp_channel(r00, r01, r10, r11),
                    interp_channel(g00, g01, g10, g11),
                    interp_channel(b00, b01, b10, b11),
                )
            }
            ImageData::Rgba(data) => {
                let (r00, g00, b00, _) = data[r0][c0];
                let (r01, g01, b01, _) = data[r0][c1];
                let (r10, g10, b10, _) = data[r1][c0];
                let (r11, g11, b11, _) = data[r1][c1];
                let interp_channel = |a: u8, b: u8, c: u8, d: u8| -> u8 {
                    let val = a as f64 * (1.0 - fr) * (1.0 - fc)
                        + b as f64 * (1.0 - fr) * fc
                        + c as f64 * fr * (1.0 - fc)
                        + d as f64 * fr * fc;
                    val.round().clamp(0.0, 255.0) as u8
                };
                Color::Rgb(
                    interp_channel(r00, r01, r10, r11),
                    interp_channel(g00, g01, g10, g11),
                    interp_channel(b00, b01, b10, b11),
                )
            }
        }
    }
}

/// Compute value bounds for scalar data. Returns (0.0, 1.0) for non-scalar data.
fn scalar_value_bounds(data: &ImageData) -> (f64, f64) {
    match data {
        ImageData::Scalar(vals) => {
            let mut vmin = f64::INFINITY;
            let mut vmax = f64::NEG_INFINITY;
            for row in vals {
                for &v in row {
                    if v.is_finite() {
                        if v < vmin {
                            vmin = v;
                        }
                        if v > vmax {
                            vmax = v;
                        }
                    }
                }
            }
            if vmin > vmax {
                (0.0, 1.0)
            } else {
                (vmin, vmax)
            }
        }
        _ => (0.0, 1.0),
    }
}

impl Widget for &ImagePlot {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let nrows = self.data.nrows();
        let ncols = self.data.ncols();
        if nrows == 0 || ncols == 0 {
            return;
        }

        let x_lo = 0.0_f64;
        let x_hi = ncols as f64;
        let y_lo = 0.0_f64;
        let y_hi = nrows as f64;

        let is_scalar = matches!(self.data, ImageData::Scalar(_));
        let colorbar_width: u16 = if self.show_colorbar && is_scalar {
            10
        } else {
            0
        };

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
        // ▀ = top half, ▄ = bottom half
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
                let top_col_f = cx as f64 / aw as f64;

                let top_color = self.resolve_pixel_color(
                    top_row_f, top_col_f, nrows, ncols,
                );

                // Bottom half-pixel
                let bot_row_f = (cy as usize * 2 + 1) as f64 / effective_height as f64;

                let bot_color = self.resolve_pixel_color(
                    bot_row_f, top_col_f, nrows, ncols,
                );

                // Use ▀ (upper half block): fg = top color, bg = bottom color
                pb.set_cell(screen_x, screen_y, '\u{2580}', top_color, bot_color, Z_DATA);
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Draw colorbar (only for scalar data, after composite)
        if self.show_colorbar && is_scalar {
            let (vmin, vmax) = scalar_value_bounds(&self.data);
            let cb = Colorbar::new(self.colormap.as_ref(), vmin, vmax)
                .label_color(self.theme.foreground);
            let cb_x = px + aw + 2;
            let remaining = (area.x + area.width).saturating_sub(cb_x);
            let cb_w = colorbar_width.min(remaining);
            if cb_w > 0 {
                let cb_area = Rect::new(cb_x, py, cb_w, ah);
                if cb_area.x + cb_area.width <= area.x + area.width {
                    (&cb).render(cb_area, buf);
                }
            }
        }
    }
}

impl ImagePlot {
    /// Map a fractional row/column position to a color, handling origin and interpolation.
    fn resolve_pixel_color(
        &self,
        row_frac: f64,
        col_frac: f64,
        nrows: usize,
        ncols: usize,
    ) -> Color {
        // Apply origin: Upper means row 0 at top (direct mapping),
        // Lower means row 0 at bottom (invert).
        let effective_row_frac = match self.origin {
            ImageOrigin::Upper => row_frac,
            ImageOrigin::Lower => 1.0 - row_frac,
        };

        match self.interpolation {
            Interpolation::Nearest => {
                let data_row =
                    (effective_row_frac * nrows as f64).min((nrows - 1) as f64).max(0.0) as usize;
                let data_col =
                    (col_frac * ncols as f64).min((ncols - 1) as f64).max(0.0) as usize;
                self.sample_color(data_row, data_col)
            }
            Interpolation::Bilinear => {
                let data_row_f =
                    (effective_row_frac * nrows as f64 - 0.5).clamp(0.0, (nrows - 1) as f64);
                let data_col_f =
                    (col_frac * ncols as f64 - 0.5).clamp(0.0, (ncols - 1) as f64);
                self.sample_color_bilinear(data_row_f, data_col_f, nrows, ncols)
            }
        }
    }
}

/// Create a sparsity pattern plot.
///
/// Displays the non-zero structure of a matrix using a two-color scheme:
/// zero/NaN cells are white, non-zero cells are blue.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::image_plot::spy;
///
/// let matrix = vec![
///     vec![1.0, 0.0, 0.0],
///     vec![0.0, 2.0, 0.0],
///     vec![0.0, 0.0, 3.0],
/// ];
/// let plot = spy(&matrix);
/// ```
pub fn spy(matrix: &[Vec<f64>]) -> ImagePlot {
    let binary: Vec<Vec<f64>> = matrix
        .iter()
        .map(|row| {
            row.iter()
                .map(|&v| if v != 0.0 && v.is_finite() { 1.0 } else { 0.0 })
                .collect()
        })
        .collect();
    let nrows = binary.len();
    let ncols = binary.first().map_or(0, |r| r.len());
    ImagePlot::new(ImageData::Scalar(binary))
        .colormap(crate::colormap::Blues)
        .norm(LinearNorm::new(0.0, 1.0))
        .show_colorbar(false)
        .title("Spy")
        .x_axis(Axis::new().bounds(Bounds::Manual(0.0, ncols as f64)))
        .y_axis(Axis::new().bounds(Bounds::Manual(0.0, nrows as f64)))
}

/// Create a matrix visualization with automatic value range.
///
/// Displays a matrix as a color-mapped image with a colorbar, using the
/// upper origin convention (row 0 at top) and equal aspect ratio.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::image_plot::matshow;
///
/// let matrix = vec![
///     vec![1.0, 2.0, 3.0],
///     vec![4.0, 5.0, 6.0],
///     vec![7.0, 8.0, 9.0],
/// ];
/// let plot = matshow(matrix);
/// ```
pub fn matshow(matrix: Vec<Vec<f64>>) -> ImagePlot {
    let (vmin, vmax) = scalar_value_bounds(&ImageData::Scalar(matrix.clone()));
    ImagePlot::new(ImageData::Scalar(matrix))
        .norm(LinearNorm::new(vmin, vmax))
        .origin(ImageOrigin::Upper)
        .aspect_ratio(AspectRatio::Equal)
        .show_colorbar(true)
}
