//! 2D histogram widget (matplotlib's hist2d).
//!
//! Bins (x, y) point data into a 2D grid and renders as a heatmap.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::colormap::{Colorbar, Colormap, Viridis};
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::norm::{LinearNorm, Normalize};
use crate::plot_buffer::{PlotBuffer, Z_DATA};
use crate::spines::Spines;
use crate::theme::Theme;

/// A 2D histogram widget.
///
/// # Example
///
/// ```
/// use ratatui_plt::widgets::hist2d::Hist2D;
///
/// let data: Vec<(f64, f64)> = (0..1000).map(|i| {
///     let x = (i as f64 * 0.01).sin();
///     let y = (i as f64 * 0.01).cos();
///     (x, y)
/// }).collect();
/// let hist = Hist2D::new(data).bins_x(20).bins_y(20);
/// ```
pub struct Hist2D {
    data: Vec<(f64, f64)>,
    bins_x: usize,
    bins_y: usize,
    colormap: Box<dyn Colormap>,
    norm: Option<Box<dyn Normalize>>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    show_colorbar: bool,
    /// Visual theme.
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Hist2D {
    pub fn new(data: Vec<(f64, f64)>) -> Self {
        Self {
            data,
            bins_x: 20,
            bins_y: 20,
            colormap: Box::new(Viridis),
            norm: None,
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

    pub fn bins_x(mut self, n: usize) -> Self {
        self.bins_x = n.max(1);
        self
    }

    pub fn bins_y(mut self, n: usize) -> Self {
        self.bins_y = n.max(1);
        self
    }

    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    pub fn norm(mut self, norm: impl Normalize + 'static) -> Self {
        self.norm = Some(Box::new(norm));
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

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn show_colorbar(mut self, show: bool) -> Self {
        self.show_colorbar = show;
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

    fn compute_bins(&self) -> (Vec<Vec<f64>>, f64, f64, f64, f64) {
        // Filter NaN
        let valid: Vec<(f64, f64)> = self
            .data
            .iter()
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .copied()
            .collect();

        if valid.is_empty() {
            return (
                vec![vec![0.0; self.bins_x]; self.bins_y],
                0.0,
                1.0,
                0.0,
                1.0,
            );
        }

        let x_min = valid.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x_max = valid.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let y_min = valid.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let y_max = valid.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);

        let x_range = if (x_max - x_min).abs() < 1e-15 {
            1.0
        } else {
            x_max - x_min
        };
        let y_range = if (y_max - y_min).abs() < 1e-15 {
            1.0
        } else {
            y_max - y_min
        };

        let mut grid = vec![vec![0.0f64; self.bins_x]; self.bins_y];

        for &(x, y) in &valid {
            let xi = ((x - x_min) / x_range * self.bins_x as f64).floor() as usize;
            let yi = ((y - y_min) / y_range * self.bins_y as f64).floor() as usize;
            let xi = xi.min(self.bins_x - 1);
            let yi = yi.min(self.bins_y - 1);
            grid[yi][xi] += 1.0;
        }

        (grid, x_min, x_max, y_min, y_max)
    }
}

impl Widget for &Hist2D {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 3 {
            return;
        }

        let (grid, x_min, x_max, y_min, y_max) = self.compute_bins();
        let nrows = grid.len();
        let ncols = if nrows > 0 { grid[0].len() } else { 0 };

        if nrows == 0 || ncols == 0 {
            return;
        }

        // Determine value range for normalization
        let vmax = grid
            .iter()
            .flat_map(|r| r.iter())
            .cloned()
            .fold(0.0f64, f64::max);
        let norm: Box<dyn Normalize> = match &self.norm {
            Some(n) => n.box_clone(),
            None => Box::new(LinearNorm::new(0.0, vmax.max(1.0))),
        };

        let colorbar_width: u16 = if self.show_colorbar { 10 } else { 0 };

        let mut pb = PlotBuffer::new(area);

        // Create and render the plot frame (title, axes, grid, ticks, labels, spines, ref lines)
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .colorbar_width(colorbar_width)
            .y_label_width(7)
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render_to_pb(
            &mut pb,
            area,
            DataBounds {
                x_lo: x_min,
                x_hi: x_max,
                y_lo: y_min,
                y_hi: y_max,
            },
        ) else {
            return;
        };

        let px = pa.x;
        let py = pa.y;
        let pw = pa.width;
        let ph = pa.height;

        // Render using half-block characters
        let effective_height = ph as usize * 2;
        for cy in 0..ph {
            for cx in 0..pw {
                let screen_x = px + cx;
                let screen_y = py + cy;
                if !pa.in_area(screen_x, screen_y) {
                    continue;
                }

                // Top half
                let top_row_f = (cy as usize * 2) as f64 / effective_height as f64;
                let top_row = ((1.0 - top_row_f) * nrows as f64).min((nrows - 1) as f64) as usize;
                let top_col =
                    (cx as f64 / pw as f64 * ncols as f64).min((ncols - 1) as f64) as usize;
                let top_val = grid[top_row][top_col];
                let top_t = norm.normalize(top_val);
                let top_color = self.colormap.color_at(top_t);

                // Bottom half
                let bot_row_f = (cy as usize * 2 + 1) as f64 / effective_height as f64;
                let bot_row = ((1.0 - bot_row_f) * nrows as f64).min((nrows - 1) as f64) as usize;
                let bot_val = grid[bot_row][top_col];
                let bot_t = norm.normalize(bot_val);
                let bot_color = self.colormap.color_at(bot_t);

                pb.set_cell(screen_x, screen_y, self.theme.chars.fill.half_upper, top_color, bot_color, Z_DATA);
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);

        // Colorbar (after composite, as it manages its own rendering)
        if self.show_colorbar {
            let cb = Colorbar::new(self.colormap.as_ref(), 0.0, vmax);
            let cb_area = Rect::new(
                px + pw + 2,
                py,
                colorbar_width.min(area.x + area.width - px - pw - 2),
                ph,
            );
            if cb_area.x + cb_area.width <= area.x + area.width {
                (&cb).render(cb_area, buf);
            }
        }
    }
}
