//! Spectrogram widget for visualizing frequency content over time.
//!
//! Renders STFT magnitude data as a heatmap using half-block characters
//! for doubled vertical resolution.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::axis::Axis;
use crate::colormap::{Colorbar, Colormap, Viridis};
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::norm::{LinearNorm, Normalize};
use crate::series::GridData;
use crate::spines::Spines;
use crate::theme::Theme;

/// A spectrogram widget that renders frequency-time magnitude data.
///
/// Accepts [`GridData`] where x = time bins, y = frequency bins,
/// and values = magnitude. Typically produced by [`crate::fft::stft`].
///
/// # Example
///
/// ```
/// use ratatui_plt::prelude::*;
/// use ratatui_plt::fft::stft;
/// use ratatui_plt::widgets::spectrogram::Spectrogram;
///
/// let signal: Vec<f64> = (0..4096).map(|i| (i as f64 * 0.05).sin()).collect();
/// let grid = stft(&signal, 256, 128);
/// let spec = Spectrogram::new(grid)
///     .title("Spectrogram")
///     .colormap(Viridis);
/// ```
pub struct Spectrogram {
    data: GridData,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    title: Option<String>,
    x_axis: Axis,
    y_axis: Axis,
    show_colorbar: bool,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
}

impl Spectrogram {
    /// Create a spectrogram from grid data.
    pub fn new(data: GridData) -> Self {
        let (vmin, vmax) = data.value_bounds();
        Self {
            data,
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            title: None,
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            show_colorbar: true,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
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

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the X axis (time axis).
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y axis (frequency axis).
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Show or hide the colorbar.
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

    /// Set all reference lines.
    pub fn reference_lines(mut self, lines: Vec<ReferenceLine>) -> Self {
        self.reference_lines = lines;
        self
    }
}

impl Widget for &Spectrogram {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let nrows = self.data.nrows();
        let ncols = self.data.ncols();
        if nrows == 0 || ncols == 0 {
            return;
        }

        let x_lo = *self.data.x.first().unwrap_or(&0.0);
        let x_hi = *self.data.x.last().unwrap_or(&1.0);
        let y_lo = *self.data.y.first().unwrap_or(&0.0);
        let y_hi = *self.data.y.last().unwrap_or(&1.0);

        let colorbar_width: u16 = if self.show_colorbar { 10 } else { 0 };

        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .colorbar_width(colorbar_width)
            .y_label_width(7)
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(area, buf, DataBounds { x_lo, x_hi, y_lo, y_hi }) else {
            return;
        };

        let px = pa.x;
        let py = pa.y;
        let aw = pa.width;
        let ah = pa.height;

        // Half-block rendering for 2x vertical resolution
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
                let top_color = if !top_val.is_finite() {
                    Color::DarkGray
                } else {
                    let t = self.norm.normalize(top_val);
                    self.colormap.color_at(t)
                };

                // Bottom half-pixel
                let bot_row_f = (cy as usize * 2 + 1) as f64 / effective_height as f64;
                let bot_data_row =
                    ((1.0 - bot_row_f) * nrows as f64).min((nrows - 1) as f64) as usize;
                let bot_data_col = top_data_col;
                let bot_val = self.data.values[bot_data_row][bot_data_col];
                let bot_color = if !bot_val.is_finite() {
                    Color::DarkGray
                } else {
                    let t = self.norm.normalize(bot_val);
                    self.colormap.color_at(t)
                };

                buf[(screen_x, screen_y)]
                    .set_char('\u{2580}')
                    .set_style(Style::default().fg(top_color).bg(bot_color));
            }
        }

        // Draw colorbar
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
