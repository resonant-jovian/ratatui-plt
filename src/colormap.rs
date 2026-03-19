//! Scientific colormaps for data visualization.
//!
//! Provides perceptually uniform, diverging, and sequential colormaps inspired
//! by matplotlib. Each colormap maps a value in [0, 1] to a terminal [`Color`].
//!
//! # Example
//!
//! ```
//! use ratatui_sim::colormap::{Colormap, Viridis};
//! use ratatui::style::Color;
//!
//! let cmap = Viridis;
//! let color = cmap.color_at(0.5); // Mid-range viridis color
//! ```

use ratatui::layout::Rect;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::norm::{LinearNorm, Normalize};

/// Trait for mapping normalized values [0, 1] to terminal colors.
pub trait Colormap: Send + Sync {
    /// Map a value in [0, 1] to a color. Values outside [0, 1] are clamped.
    fn color_at(&self, t: f64) -> Color;

    /// Get the name of this colormap.
    fn name(&self) -> &str;
}

/// Interpolate between RGB color stops.
fn lerp_color_stops(t: f64, stops: &[(f64, (u8, u8, u8))]) -> Color {
    let t = t.clamp(0.0, 1.0);
    if stops.is_empty() {
        return Color::White;
    }
    if t <= stops[0].0 {
        let (r, g, b) = stops[0].1;
        return Color::Rgb(r, g, b);
    }
    if t >= stops[stops.len() - 1].0 {
        let (r, g, b) = stops[stops.len() - 1].1;
        return Color::Rgb(r, g, b);
    }
    for i in 0..stops.len() - 1 {
        let (t0, c0) = stops[i];
        let (t1, c1) = stops[i + 1];
        if t >= t0 && t <= t1 {
            let frac = (t - t0) / (t1 - t0);
            let r = (c0.0 as f64 + frac * (c1.0 as f64 - c0.0 as f64)).round() as u8;
            let g = (c0.1 as f64 + frac * (c1.1 as f64 - c0.1 as f64)).round() as u8;
            let b = (c0.2 as f64 + frac * (c1.2 as f64 - c0.2 as f64)).round() as u8;
            return Color::Rgb(r, g, b);
        }
    }
    Color::White
}

/// Viridis: perceptually uniform sequential colormap (dark purple → green → yellow).
///
/// Colorblind-safe. The default choice for scientific visualization.
#[derive(Clone, Debug)]
pub struct Viridis;

impl Colormap for Viridis {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(t, &[
            (0.0, (68, 1, 84)),
            (0.13, (72, 36, 117)),
            (0.25, (56, 88, 140)),
            (0.38, (39, 130, 142)),
            (0.5, (31, 158, 137)),
            (0.63, (53, 183, 121)),
            (0.75, (110, 206, 88)),
            (0.88, (181, 222, 43)),
            (1.0, (253, 231, 37)),
        ])
    }

    fn name(&self) -> &str {
        "viridis"
    }
}

/// Plasma: perceptually uniform sequential (dark purple → pink → yellow).
#[derive(Clone, Debug)]
pub struct Plasma;

impl Colormap for Plasma {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(t, &[
            (0.0, (13, 8, 135)),
            (0.13, (75, 3, 161)),
            (0.25, (126, 3, 168)),
            (0.38, (168, 34, 150)),
            (0.5, (203, 70, 121)),
            (0.63, (229, 107, 93)),
            (0.75, (248, 148, 65)),
            (0.88, (253, 195, 40)),
            (1.0, (240, 249, 33)),
        ])
    }

    fn name(&self) -> &str {
        "plasma"
    }
}

/// Inferno: perceptually uniform sequential (black → red → yellow → white).
#[derive(Clone, Debug)]
pub struct Inferno;

impl Colormap for Inferno {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(t, &[
            (0.0, (0, 0, 4)),
            (0.13, (31, 12, 72)),
            (0.25, (85, 15, 109)),
            (0.38, (136, 34, 106)),
            (0.5, (186, 54, 85)),
            (0.63, (227, 89, 51)),
            (0.75, (249, 140, 10)),
            (0.88, (249, 201, 50)),
            (1.0, (252, 255, 164)),
        ])
    }

    fn name(&self) -> &str {
        "inferno"
    }
}

/// Magma: perceptually uniform sequential (black → purple → pink → yellow).
#[derive(Clone, Debug)]
pub struct Magma;

impl Colormap for Magma {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(t, &[
            (0.0, (0, 0, 4)),
            (0.13, (28, 16, 68)),
            (0.25, (79, 18, 123)),
            (0.38, (129, 37, 129)),
            (0.5, (181, 54, 122)),
            (0.63, (229, 80, 100)),
            (0.75, (251, 135, 97)),
            (0.88, (254, 194, 140)),
            (1.0, (252, 253, 191)),
        ])
    }

    fn name(&self) -> &str {
        "magma"
    }
}

/// Cividis: perceptually uniform, optimized for color vision deficiency (blue → yellow).
#[derive(Clone, Debug)]
pub struct Cividis;

impl Colormap for Cividis {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(t, &[
            (0.0, (0, 32, 77)),
            (0.25, (57, 75, 107)),
            (0.5, (124, 123, 120)),
            (0.75, (194, 176, 120)),
            (1.0, (255, 234, 70)),
        ])
    }

    fn name(&self) -> &str {
        "cividis"
    }
}

/// Coolwarm: diverging colormap (blue → white → red).
#[derive(Clone, Debug)]
pub struct Coolwarm;

impl Colormap for Coolwarm {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(t, &[
            (0.0, (59, 76, 192)),
            (0.25, (124, 159, 237)),
            (0.5, (221, 221, 221)),
            (0.75, (230, 145, 113)),
            (1.0, (180, 4, 38)),
        ])
    }

    fn name(&self) -> &str {
        "coolwarm"
    }
}

/// RdBu: diverging colormap (red → white → blue).
#[derive(Clone, Debug)]
pub struct RdBu;

impl Colormap for RdBu {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(t, &[
            (0.0, (103, 0, 31)),
            (0.25, (214, 96, 77)),
            (0.5, (247, 247, 247)),
            (0.75, (67, 147, 195)),
            (1.0, (5, 48, 97)),
        ])
    }

    fn name(&self) -> &str {
        "RdBu"
    }
}

/// Seismic: diverging colormap (dark blue → white → dark red).
#[derive(Clone, Debug)]
pub struct Seismic;

impl Colormap for Seismic {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(t, &[
            (0.0, (0, 0, 76)),
            (0.25, (0, 0, 255)),
            (0.5, (255, 255, 255)),
            (0.75, (255, 0, 0)),
            (1.0, (128, 0, 0)),
        ])
    }

    fn name(&self) -> &str {
        "seismic"
    }
}

/// Grayscale: sequential (black → white).
#[derive(Clone, Debug)]
pub struct Grayscale;

impl Colormap for Grayscale {
    fn color_at(&self, t: f64) -> Color {
        let v = (t.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color::Rgb(v, v, v)
    }

    fn name(&self) -> &str {
        "grayscale"
    }
}

/// Jet: rainbow colormap (blue → cyan → green → yellow → red). Not perceptually uniform.
#[derive(Clone, Debug)]
pub struct Jet;

impl Colormap for Jet {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(t, &[
            (0.0, (0, 0, 127)),
            (0.11, (0, 0, 255)),
            (0.35, (0, 255, 255)),
            (0.5, (0, 255, 0)),
            (0.65, (255, 255, 0)),
            (0.89, (255, 0, 0)),
            (1.0, (127, 0, 0)),
        ])
    }

    fn name(&self) -> &str {
        "jet"
    }
}

/// Turbo: improved rainbow colormap. Better perceptual properties than Jet.
#[derive(Clone, Debug)]
pub struct Turbo;

impl Colormap for Turbo {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(t, &[
            (0.0, (48, 18, 59)),
            (0.13, (67, 85, 221)),
            (0.25, (29, 162, 254)),
            (0.38, (11, 224, 198)),
            (0.5, (80, 253, 107)),
            (0.63, (183, 244, 37)),
            (0.75, (246, 195, 28)),
            (0.88, (249, 114, 10)),
            (1.0, (122, 4, 3)),
        ])
    }

    fn name(&self) -> &str {
        "turbo"
    }
}

/// Hot: sequential (black → red → yellow → white).
#[derive(Clone, Debug)]
pub struct Hot;

impl Colormap for Hot {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(t, &[
            (0.0, (0, 0, 0)),
            (0.33, (230, 0, 0)),
            (0.66, (255, 210, 0)),
            (1.0, (255, 255, 255)),
        ])
    }

    fn name(&self) -> &str {
        "hot"
    }
}

/// User-defined colormap from a list of color stops.
///
/// # Example
///
/// ```
/// use ratatui_sim::colormap::{Colormap, ListedColormap};
/// use ratatui::style::Color;
///
/// let cmap = ListedColormap::new("custom", vec![
///     (0.0, Color::Blue),
///     (0.5, Color::White),
///     (1.0, Color::Red),
/// ]);
/// ```
#[derive(Clone, Debug)]
pub struct ListedColormap {
    name: String,
    stops: Vec<(f64, Color)>,
}

impl ListedColormap {
    /// Create a colormap from (position, color) stops.
    pub fn new(name: impl Into<String>, stops: Vec<(f64, Color)>) -> Self {
        Self {
            name: name.into(),
            stops,
        }
    }
}

impl Colormap for ListedColormap {
    fn color_at(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        if self.stops.is_empty() {
            return Color::White;
        }
        if self.stops.len() == 1 {
            return self.stops[0].1;
        }
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        let last = self.stops.len() - 1;
        if t >= self.stops[last].0 {
            return self.stops[last].1;
        }

        for i in 0..last {
            let (t0, c0) = &self.stops[i];
            let (t1, c1) = &self.stops[i + 1];
            if t >= *t0 && t <= *t1 {
                let frac = (t - t0) / (t1 - t0);
                return lerp_colors(*c0, *c1, frac);
            }
        }
        Color::White
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Linearly interpolate between two Colors.
fn lerp_colors(c0: Color, c1: Color, t: f64) -> Color {
    let (r0, g0, b0) = color_to_rgb(c0);
    let (r1, g1, b1) = color_to_rgb(c1);
    let r = (r0 as f64 + t * (r1 as f64 - r0 as f64)).round() as u8;
    let g = (g0 as f64 + t * (g1 as f64 - g0 as f64)).round() as u8;
    let b = (b0 as f64 + t * (b1 as f64 - b0 as f64)).round() as u8;
    Color::Rgb(r, g, b)
}

/// Extract RGB components from a Color (approximation for non-RGB colors).
fn color_to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (255, 0, 0),
        Color::Green => (0, 255, 0),
        Color::Yellow => (255, 255, 0),
        Color::Blue => (0, 0, 255),
        Color::Magenta => (255, 0, 255),
        Color::Cyan => (0, 255, 255),
        Color::White => (255, 255, 255),
        Color::Gray => (128, 128, 128),
        Color::DarkGray => (64, 64, 64),
        Color::LightRed => (255, 128, 128),
        Color::LightGreen => (128, 255, 128),
        Color::LightYellow => (255, 255, 128),
        Color::LightBlue => (128, 128, 255),
        Color::LightMagenta => (255, 128, 255),
        Color::LightCyan => (128, 255, 255),
        _ => (255, 255, 255),
    }
}

/// Colorbar widget: displays a vertical color gradient with value labels.
///
/// Rendered alongside heatmaps and contour plots to show the value→color mapping.
#[derive(Clone)]
pub struct Colorbar<'a> {
    /// Colormap to display.
    cmap: &'a dyn Colormap,
    /// Normalizer (determines value range).
    norm: Box<dyn Normalize>,
    /// Value range for labels.
    vmin: f64,
    vmax: f64,
    /// Number of label ticks.
    n_ticks: usize,
    /// Width in characters.
    width: u16,
}

impl<'a> Colorbar<'a> {
    /// Create a new colorbar.
    pub fn new(cmap: &'a dyn Colormap, vmin: f64, vmax: f64) -> Self {
        Self {
            cmap,
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            vmin,
            vmax,
            n_ticks: 5,
            width: 4,
        }
    }

    /// Set a custom normalizer.
    pub fn norm(mut self, norm: impl Normalize + 'static) -> Self {
        self.norm = Box::new(norm);
        self
    }

    /// Set the number of label ticks.
    pub fn n_ticks(mut self, n: usize) -> Self {
        self.n_ticks = n;
        self
    }

    /// Set the width in characters.
    pub fn width(mut self, w: u16) -> Self {
        self.width = w;
        self
    }
}

impl Widget for &Colorbar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let bar_width = self.width.min(area.width.saturating_sub(6));
        let label_x = area.x + bar_width + 1;

        // Draw color gradient (bottom = vmin, top = vmax)
        for row in 0..area.height {
            let t = 1.0 - row as f64 / (area.height.saturating_sub(1)) as f64;
            let color = self.cmap.color_at(t);
            for col in 0..bar_width {
                let x = area.x + col;
                let y = area.y + row;
                if x < area.x + area.width && y < area.y + area.height {
                    buf[(x, y)].set_char('█').set_fg(color);
                }
            }
        }

        // Draw tick labels
        if area.width > bar_width + 1 {
            let label_width = (area.width - bar_width - 1) as usize;
            for i in 0..self.n_ticks {
                let t = i as f64 / (self.n_ticks - 1).max(1) as f64;
                let row = ((1.0 - t) * (area.height.saturating_sub(1)) as f64).round() as u16;
                let value = self.vmin + t * (self.vmax - self.vmin);
                let label = format!("{:.2}", value);
                let label = if label.len() > label_width {
                    &label[..label_width]
                } else {
                    &label
                };
                let y = area.y + row;
                if y < area.y + area.height {
                    for (j, ch) in label.chars().enumerate() {
                        let x = label_x + j as u16;
                        if x < area.x + area.width {
                            buf[(x, y)].set_char(ch).set_style(Style::default());
                        }
                    }
                }
            }
        }
    }
}
