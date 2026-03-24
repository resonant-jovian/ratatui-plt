//! Scientific colormaps for data visualization.
//!
//! Provides perceptually uniform, diverging, and sequential colormaps inspired
//! by matplotlib. Each colormap maps a value in [0, 1] to a terminal [`Color`].
//!
//! # Example
//!
//! ```
//! use ratatui_plt::colormap::{Colormap, Viridis};
//! use ratatui::style::Color;
//!
//! let cmap = Viridis;
//! let color = cmap.color_at(0.5); // Mid-range viridis color
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
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

impl Colormap for Box<dyn Colormap> {
    fn color_at(&self, t: f64) -> Color {
        (**self).color_at(t)
    }
    fn name(&self) -> &str {
        (**self).name()
    }
}

/// Interpolate between RGB color stops.
fn lerp_color_stops(t: f64, stops: &[(f64, (u8, u8, u8))]) -> Color {
    let t = t.clamp(0.0, 1.0);
    if stops.is_empty() {
        return Color::Rgb(128, 128, 128);
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
    Color::Rgb(128, 128, 128)
}

/// Viridis: perceptually uniform sequential colormap (dark purple → green → yellow).
///
/// Colorblind-safe. The default choice for scientific visualization.
#[derive(Clone, Debug)]
pub struct Viridis;

impl Colormap for Viridis {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(
            t,
            &[
                (0.0, (68, 1, 84)),
                (0.13, (72, 36, 117)),
                (0.25, (56, 88, 140)),
                (0.38, (39, 130, 142)),
                (0.5, (31, 158, 137)),
                (0.63, (53, 183, 121)),
                (0.75, (110, 206, 88)),
                (0.88, (181, 222, 43)),
                (1.0, (253, 231, 37)),
            ],
        )
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
        lerp_color_stops(
            t,
            &[
                (0.0, (13, 8, 135)),
                (0.13, (75, 3, 161)),
                (0.25, (126, 3, 168)),
                (0.38, (168, 34, 150)),
                (0.5, (203, 70, 121)),
                (0.63, (229, 107, 93)),
                (0.75, (248, 148, 65)),
                (0.88, (253, 195, 40)),
                (1.0, (240, 249, 33)),
            ],
        )
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
        lerp_color_stops(
            t,
            &[
                (0.0, (0, 0, 4)),
                (0.13, (31, 12, 72)),
                (0.25, (85, 15, 109)),
                (0.38, (136, 34, 106)),
                (0.5, (186, 54, 85)),
                (0.63, (227, 89, 51)),
                (0.75, (249, 140, 10)),
                (0.88, (249, 201, 50)),
                (1.0, (252, 255, 164)),
            ],
        )
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
        lerp_color_stops(
            t,
            &[
                (0.0, (0, 0, 4)),
                (0.13, (28, 16, 68)),
                (0.25, (79, 18, 123)),
                (0.38, (129, 37, 129)),
                (0.5, (181, 54, 122)),
                (0.63, (229, 80, 100)),
                (0.75, (251, 135, 97)),
                (0.88, (254, 194, 140)),
                (1.0, (252, 253, 191)),
            ],
        )
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
        lerp_color_stops(
            t,
            &[
                (0.0, (0, 32, 77)),
                (0.25, (57, 75, 107)),
                (0.5, (124, 123, 120)),
                (0.75, (194, 176, 120)),
                (1.0, (255, 234, 70)),
            ],
        )
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
        lerp_color_stops(
            t,
            &[
                (0.0, (59, 76, 192)),
                (0.25, (124, 159, 237)),
                (0.5, (221, 221, 221)),
                (0.75, (230, 145, 113)),
                (1.0, (180, 4, 38)),
            ],
        )
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
        lerp_color_stops(
            t,
            &[
                (0.0, (103, 0, 31)),
                (0.25, (214, 96, 77)),
                (0.5, (247, 247, 247)),
                (0.75, (67, 147, 195)),
                (1.0, (5, 48, 97)),
            ],
        )
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
        lerp_color_stops(
            t,
            &[
                (0.0, (0, 0, 76)),
                (0.25, (0, 0, 255)),
                (0.5, (255, 255, 255)),
                (0.75, (255, 0, 0)),
                (1.0, (128, 0, 0)),
            ],
        )
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
        lerp_color_stops(
            t,
            &[
                (0.0, (0, 0, 127)),
                (0.11, (0, 0, 255)),
                (0.35, (0, 255, 255)),
                (0.5, (0, 255, 0)),
                (0.65, (255, 255, 0)),
                (0.89, (255, 0, 0)),
                (1.0, (127, 0, 0)),
            ],
        )
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
        lerp_color_stops(
            t,
            &[
                (0.0, (48, 18, 59)),
                (0.13, (67, 85, 221)),
                (0.25, (29, 162, 254)),
                (0.38, (11, 224, 198)),
                (0.5, (80, 253, 107)),
                (0.63, (183, 244, 37)),
                (0.75, (246, 195, 28)),
                (0.88, (249, 114, 10)),
                (1.0, (122, 4, 3)),
            ],
        )
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
        lerp_color_stops(
            t,
            &[
                (0.0, (0, 0, 0)),
                (0.33, (230, 0, 0)),
                (0.66, (255, 210, 0)),
                (1.0, (255, 255, 255)),
            ],
        )
    }

    fn name(&self) -> &str {
        "hot"
    }
}

/// User-defined colormap from a list of color stops.
/// Cubehelix: perceptually monotonic spiral through color space (black → purple → teal → green → white).
///
/// Based on Green (2011), this colormap spirals through RGB while keeping perceived
/// brightness monotonically increasing. Excellent for scientific data where grayscale
/// printing must also work.
#[derive(Clone, Copy, Debug)]
pub struct Cubehelix;

impl Colormap for Cubehelix {
    fn color_at(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        // 9-stop approximation of the default cubehelix (start=0.5, rotations=-1.5, hue=1.0)
        const STOPS: [(f64, u8, u8, u8); 9] = [
            (0.000, 0, 0, 0),
            (0.125, 22, 17, 42),
            (0.250, 15, 56, 62),
            (0.375, 28, 98, 47),
            (0.500, 87, 117, 58),
            (0.625, 168, 115, 103),
            (0.750, 196, 130, 182),
            (0.875, 199, 180, 238),
            (1.000, 255, 255, 255),
        ];
        // Find surrounding stops and interpolate
        let mut lo = 0;
        for (i, stop) in STOPS.iter().enumerate().skip(1) {
            if stop.0 >= t {
                lo = i - 1;
                break;
            }
            lo = i;
        }
        let hi = (lo + 1).min(STOPS.len() - 1);
        let (t0, r0, g0, b0) = STOPS[lo];
        let (t1, r1, g1, b1) = STOPS[hi];
        let frac = if (t1 - t0).abs() < 1e-12 {
            0.0
        } else {
            (t - t0) / (t1 - t0)
        };
        let lerp = |a: u8, b: u8| -> u8 { (a as f64 + (b as f64 - a as f64) * frac) as u8 };
        Color::Rgb(lerp(r0, r1), lerp(g0, g1), lerp(b0, b1))
    }
    fn name(&self) -> &str {
        "cubehelix"
    }
}

///
/// # Example
///
/// ```
/// use ratatui_plt::colormap::{Colormap, ListedColormap};
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
            return Color::Rgb(128, 128, 128);
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
        Color::Rgb(128, 128, 128)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// A reversed version of any colormap.
///
/// Wraps an existing colormap and reverses the direction: `color_at(t)` returns
/// the wrapped colormap's `color_at(1.0 - t)`.
///
/// # Example
///
/// ```
/// use ratatui_plt::colormap::{Colormap, Viridis, Reversed};
///
/// let cmap = Reversed::new(Viridis);
/// // cmap.color_at(0.0) == Viridis.color_at(1.0)
/// ```
#[derive(Clone, Debug)]
pub struct Reversed<C: Colormap> {
    inner: C,
}

impl<C: Colormap> Reversed<C> {
    /// Create a reversed version of the given colormap.
    pub fn new(inner: C) -> Self {
        Self { inner }
    }
}

impl<C: Colormap> Colormap for Reversed<C> {
    fn color_at(&self, t: f64) -> Color {
        self.inner.color_at(1.0 - t.clamp(0.0, 1.0))
    }

    fn name(&self) -> &str {
        // Not ideal but avoids allocation in a trait method
        self.inner.name()
    }
}

// -- Additional sequential colormaps --

/// Spring: magenta → yellow.
#[derive(Clone, Debug)]
pub struct Spring;

impl Colormap for Spring {
    fn color_at(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        let r = 255;
        let g = (t * 255.0).round() as u8;
        let b = (255.0 - t * 255.0).round() as u8;
        Color::Rgb(r, g, b)
    }
    fn name(&self) -> &str {
        "spring"
    }
}

/// Summer: green → yellow.
#[derive(Clone, Debug)]
pub struct Summer;

impl Colormap for Summer {
    fn color_at(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        let r = (t * 255.0).round() as u8;
        let g = (128.0 + t * 127.0).round() as u8;
        let b = 102;
        Color::Rgb(r, g, b)
    }
    fn name(&self) -> &str {
        "summer"
    }
}

/// Autumn: red → yellow.
#[derive(Clone, Debug)]
pub struct Autumn;

impl Colormap for Autumn {
    fn color_at(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        let r = 255;
        let g = (t * 255.0).round() as u8;
        let b = 0;
        Color::Rgb(r, g, b)
    }
    fn name(&self) -> &str {
        "autumn"
    }
}

/// Winter: blue → green.
#[derive(Clone, Debug)]
pub struct Winter;

impl Colormap for Winter {
    fn color_at(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        let r = 0;
        let g = (t * 255.0).round() as u8;
        let b = (255.0 - t * 127.0).round() as u8;
        Color::Rgb(r, g, b)
    }
    fn name(&self) -> &str {
        "winter"
    }
}

/// Twilight: cyclic colormap suitable for phase/angle data.
#[derive(Clone, Debug)]
pub struct Twilight;

impl Colormap for Twilight {
    fn color_at(&self, t: f64) -> Color {
        lerp_color_stops(
            t,
            &[
                (0.0, (226, 217, 226)),
                (0.15, (166, 133, 193)),
                (0.3, (81, 71, 153)),
                (0.5, (18, 36, 61)),
                (0.7, (69, 99, 68)),
                (0.85, (171, 173, 117)),
                (1.0, (226, 217, 226)),
            ],
        )
    }
    fn name(&self) -> &str {
        "twilight"
    }
}

/// HSV: cyclic hue-saturation-value rainbow.
#[derive(Clone, Debug)]
pub struct Hsv;

impl Colormap for Hsv {
    fn color_at(&self, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        let h = t * 360.0;
        let s = 1.0_f64;
        let v = 1.0_f64;
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        let (r1, g1, b1) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        Color::Rgb(
            ((r1 + m) * 255.0).round() as u8,
            ((g1 + m) * 255.0).round() as u8,
            ((b1 + m) * 255.0).round() as u8,
        )
    }
    fn name(&self) -> &str {
        "hsv"
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

/// Scale an RGB color by a brightness factor (for lighting effects).
pub fn scale_color(color: Color, factor: f64) -> Color {
    let (r, g, b) = color_to_rgb(color);
    let f = factor.clamp(0.0, 1.0);
    Color::Rgb(
        (r as f64 * f).round() as u8,
        (g as f64 * f).round() as u8,
        (b as f64 * f).round() as u8,
    )
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

// -- Macro for defining colormaps from color stops --

macro_rules! define_colormap {
    ($name:ident, $display_name:expr, $($t:expr => ($r:expr, $g:expr, $b:expr)),+ $(,)?) => {
        #[derive(Clone, Debug)]
        pub struct $name;

        impl Colormap for $name {
            fn color_at(&self, t: f64) -> Color {
                lerp_color_stops(t, &[$(($t, ($r, $g, $b)),)+])
            }
            fn name(&self) -> &str {
                $display_name
            }
        }
    };
}

// -- Piecewise-linear R/G/B channel colormap --

/// Colormap with piecewise-linear interpolation of R, G, B channels independently.
#[derive(Clone, Debug)]
pub struct LinearSegmentedColormap {
    name: String,
    /// Each channel is a list of (x, y0, y1) anchor points.
    /// x in [0,1], y0 = value leaving, y1 = value entering.
    r_stops: Vec<(f64, f64, f64)>,
    g_stops: Vec<(f64, f64, f64)>,
    b_stops: Vec<(f64, f64, f64)>,
}

impl LinearSegmentedColormap {
    pub fn new(
        name: impl Into<String>,
        r_stops: Vec<(f64, f64, f64)>,
        g_stops: Vec<(f64, f64, f64)>,
        b_stops: Vec<(f64, f64, f64)>,
    ) -> Self {
        Self {
            name: name.into(),
            r_stops,
            g_stops,
            b_stops,
        }
    }

    fn interpolate_channel(t: f64, stops: &[(f64, f64, f64)]) -> u8 {
        let t = t.clamp(0.0, 1.0);
        if stops.is_empty() {
            return 0;
        }
        if t <= stops[0].0 {
            return (stops[0].2 * 255.0).round() as u8;
        }
        let last = stops.len() - 1;
        if t >= stops[last].0 {
            return (stops[last].1 * 255.0).round() as u8;
        }
        for i in 0..last {
            if t >= stops[i].0 && t <= stops[i + 1].0 {
                let frac = (t - stops[i].0) / (stops[i + 1].0 - stops[i].0);
                let val = stops[i].1 + frac * (stops[i + 1].2 - stops[i].1);
                return (val * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
        0
    }
}

impl Colormap for LinearSegmentedColormap {
    fn color_at(&self, t: f64) -> Color {
        let r = Self::interpolate_channel(t, &self.r_stops);
        let g = Self::interpolate_channel(t, &self.g_stops);
        let b = Self::interpolate_channel(t, &self.b_stops);
        Color::Rgb(r, g, b)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// -- Sequential single-hue colormaps --

define_colormap!(Blues, "Blues",
    0.0 => (247, 251, 255), 0.25 => (189, 215, 231), 0.5 => (107, 174, 214),
    0.75 => (33, 113, 181), 1.0 => (8, 48, 107));

define_colormap!(Greens, "Greens",
    0.0 => (247, 252, 245), 0.25 => (199, 233, 192), 0.5 => (116, 196, 118),
    0.75 => (35, 139, 69), 1.0 => (0, 68, 27));

define_colormap!(Reds, "Reds",
    0.0 => (255, 245, 240), 0.25 => (252, 187, 161), 0.5 => (251, 106, 74),
    0.75 => (203, 24, 29), 1.0 => (103, 0, 13));

define_colormap!(Oranges, "Oranges",
    0.0 => (255, 245, 235), 0.25 => (253, 208, 162), 0.5 => (253, 141, 60),
    0.75 => (217, 72, 1), 1.0 => (127, 39, 4));

define_colormap!(Purples, "Purples",
    0.0 => (252, 251, 253), 0.25 => (218, 218, 235), 0.5 => (158, 154, 200),
    0.75 => (106, 81, 163), 1.0 => (63, 0, 125));

define_colormap!(Greys, "Greys",
    0.0 => (255, 255, 255), 0.25 => (217, 217, 217), 0.5 => (150, 150, 150),
    0.75 => (82, 82, 82), 1.0 => (0, 0, 0));

// -- ColorBrewer sequential multi-hue --

define_colormap!(YlOrBr, "YlOrBr",
    0.0 => (255, 255, 229), 0.25 => (254, 217, 142), 0.5 => (254, 153, 41),
    0.75 => (204, 76, 2), 1.0 => (102, 37, 6));

define_colormap!(YlOrRd, "YlOrRd",
    0.0 => (255, 255, 204), 0.25 => (254, 204, 92), 0.5 => (253, 141, 60),
    0.75 => (227, 26, 28), 1.0 => (128, 0, 38));

define_colormap!(OrRd, "OrRd",
    0.0 => (255, 247, 236), 0.25 => (253, 204, 138), 0.5 => (252, 141, 89),
    0.75 => (215, 48, 31), 1.0 => (127, 0, 0));

define_colormap!(PuRd, "PuRd",
    0.0 => (247, 244, 249), 0.25 => (215, 181, 216), 0.5 => (223, 101, 176),
    0.75 => (206, 18, 86), 1.0 => (103, 0, 31));

define_colormap!(RdPu, "RdPu",
    0.0 => (255, 247, 243), 0.25 => (253, 185, 190), 0.5 => (247, 104, 161),
    0.75 => (174, 1, 126), 1.0 => (73, 0, 106));

define_colormap!(BuPu, "BuPu",
    0.0 => (247, 252, 253), 0.25 => (179, 205, 227), 0.5 => (140, 150, 198),
    0.75 => (136, 65, 157), 1.0 => (77, 0, 75));

define_colormap!(GnBu, "GnBu",
    0.0 => (247, 252, 240), 0.25 => (186, 228, 188), 0.5 => (123, 204, 196),
    0.75 => (43, 140, 190), 1.0 => (8, 64, 129));

define_colormap!(PuBu, "PuBu",
    0.0 => (255, 247, 251), 0.25 => (189, 201, 225), 0.5 => (116, 169, 207),
    0.75 => (5, 112, 176), 1.0 => (3, 35, 120));

define_colormap!(YlGnBu, "YlGnBu",
    0.0 => (255, 255, 217), 0.25 => (161, 218, 180), 0.5 => (65, 182, 196),
    0.75 => (34, 94, 168), 1.0 => (8, 29, 88));

define_colormap!(PuBuGn, "PuBuGn",
    0.0 => (255, 247, 251), 0.25 => (189, 201, 225), 0.5 => (103, 169, 207),
    0.75 => (2, 129, 138), 1.0 => (1, 70, 54));

define_colormap!(BuGn, "BuGn",
    0.0 => (247, 252, 253), 0.25 => (178, 226, 226), 0.5 => (102, 194, 164),
    0.75 => (35, 139, 69), 1.0 => (0, 68, 27));

define_colormap!(YlGn, "YlGn",
    0.0 => (255, 255, 229), 0.25 => (194, 230, 153), 0.5 => (120, 198, 121),
    0.75 => (35, 132, 67), 1.0 => (0, 69, 41));

// -- Diverging colormaps --

define_colormap!(PiYG, "PiYG",
    0.0 => (142, 1, 82), 0.25 => (222, 119, 174), 0.5 => (247, 247, 247),
    0.75 => (127, 188, 65), 1.0 => (39, 100, 25));

define_colormap!(PRGn, "PRGn",
    0.0 => (64, 0, 75), 0.25 => (153, 112, 171), 0.5 => (247, 247, 247),
    0.75 => (90, 174, 97), 1.0 => (0, 68, 27));

define_colormap!(BrBG, "BrBG",
    0.0 => (84, 48, 5), 0.25 => (191, 153, 83), 0.5 => (245, 245, 245),
    0.75 => (90, 180, 172), 1.0 => (0, 60, 48));

define_colormap!(PuOr, "PuOr",
    0.0 => (127, 59, 8), 0.25 => (224, 163, 46), 0.5 => (247, 247, 247),
    0.75 => (153, 142, 195), 1.0 => (45, 0, 75));

define_colormap!(RdGy, "RdGy",
    0.0 => (103, 0, 31), 0.25 => (214, 96, 77), 0.5 => (255, 255, 255),
    0.75 => (150, 150, 150), 1.0 => (26, 26, 26));

define_colormap!(RdYlBu, "RdYlBu",
    0.0 => (165, 0, 38), 0.25 => (244, 109, 67), 0.5 => (255, 255, 191),
    0.75 => (116, 173, 209), 1.0 => (49, 54, 149));

define_colormap!(RdYlGn, "RdYlGn",
    0.0 => (165, 0, 38), 0.25 => (244, 109, 67), 0.5 => (255, 255, 191),
    0.75 => (102, 189, 99), 1.0 => (0, 104, 55));

define_colormap!(Spectral, "Spectral",
    0.0 => (158, 1, 66), 0.25 => (244, 109, 67), 0.5 => (255, 255, 191),
    0.75 => (102, 194, 165), 1.0 => (94, 79, 162));

// -- Qualitative colormaps (nearest-color lookup, no interpolation) --

/// Helper for qualitative colormaps: picks nearest color from a palette.
fn qualitative_color(t: f64, colors: &[(u8, u8, u8)]) -> Color {
    if colors.is_empty() {
        return Color::Rgb(128, 128, 128);
    }
    let idx = (t.clamp(0.0, 1.0) * (colors.len() - 1) as f64).round() as usize;
    let (r, g, b) = colors[idx.min(colors.len() - 1)];
    Color::Rgb(r, g, b)
}

macro_rules! define_qualitative {
    ($name:ident, $display_name:expr, $($color:expr),+ $(,)?) => {
        #[derive(Clone, Debug)]
        pub struct $name;

        impl Colormap for $name {
            fn color_at(&self, t: f64) -> Color {
                qualitative_color(t, &[$($color,)+])
            }
            fn name(&self) -> &str {
                $display_name
            }
        }
    };
}

define_qualitative!(
    Tab20,
    "tab20",
    (31, 119, 180),
    (174, 199, 232),
    (255, 127, 14),
    (255, 187, 120),
    (44, 160, 44),
    (152, 223, 138),
    (214, 39, 40),
    (255, 152, 150),
    (148, 103, 189),
    (197, 176, 213),
    (140, 86, 75),
    (196, 156, 148),
    (227, 119, 194),
    (247, 182, 210),
    (127, 127, 127),
    (199, 199, 199),
    (188, 189, 34),
    (219, 219, 141),
    (23, 190, 207),
    (158, 218, 229)
);

define_qualitative!(
    Tab20b,
    "tab20b",
    (57, 59, 121),
    (82, 84, 163),
    (107, 110, 207),
    (156, 158, 222),
    (99, 121, 57),
    (140, 162, 82),
    (181, 207, 107),
    (206, 219, 156),
    (140, 109, 49),
    (189, 158, 57),
    (231, 186, 82),
    (231, 203, 148),
    (132, 60, 57),
    (173, 73, 74),
    (214, 97, 107),
    (231, 150, 156),
    (123, 65, 115),
    (165, 81, 148),
    (206, 109, 189),
    (222, 158, 214)
);

define_qualitative!(
    Tab20c,
    "tab20c",
    (49, 130, 189),
    (107, 174, 214),
    (158, 202, 225),
    (198, 219, 239),
    (230, 85, 13),
    (253, 141, 60),
    (253, 174, 107),
    (253, 208, 162),
    (49, 163, 84),
    (116, 196, 118),
    (161, 217, 155),
    (199, 233, 192),
    (117, 107, 177),
    (158, 154, 200),
    (188, 189, 220),
    (218, 218, 235),
    (158, 1, 66),
    (213, 62, 79),
    (244, 109, 67),
    (253, 174, 97)
);

define_qualitative!(
    Paired,
    "Paired",
    (166, 206, 227),
    (31, 120, 180),
    (178, 223, 138),
    (51, 160, 44),
    (251, 154, 153),
    (227, 26, 28),
    (253, 191, 111),
    (255, 127, 0),
    (202, 178, 214),
    (106, 61, 154),
    (255, 255, 153),
    (177, 89, 40)
);

define_qualitative!(
    Set1,
    "Set1",
    (228, 26, 28),
    (55, 126, 184),
    (77, 175, 74),
    (152, 78, 163),
    (255, 127, 0),
    (255, 255, 51),
    (166, 86, 40),
    (247, 129, 191),
    (153, 153, 153)
);

define_qualitative!(
    Set2,
    "Set2",
    (102, 194, 165),
    (252, 141, 98),
    (141, 160, 203),
    (231, 138, 195),
    (166, 216, 84),
    (255, 217, 47),
    (229, 196, 148),
    (179, 179, 179)
);

define_qualitative!(
    Set3,
    "Set3",
    (141, 211, 199),
    (255, 255, 179),
    (190, 186, 218),
    (251, 128, 114),
    (128, 177, 211),
    (253, 180, 98),
    (179, 222, 105),
    (252, 205, 229),
    (217, 217, 217),
    (188, 128, 189),
    (204, 235, 197),
    (255, 237, 111)
);

define_qualitative!(
    Pastel1,
    "Pastel1",
    (251, 180, 174),
    (179, 205, 227),
    (204, 235, 197),
    (222, 203, 228),
    (254, 217, 166),
    (255, 255, 204),
    (229, 216, 189),
    (253, 218, 236),
    (242, 242, 242)
);

define_qualitative!(
    Pastel2,
    "Pastel2",
    (179, 226, 205),
    (253, 205, 172),
    (203, 213, 232),
    (244, 202, 228),
    (230, 245, 201),
    (255, 242, 174),
    (241, 226, 204),
    (204, 204, 204)
);

define_qualitative!(
    Accent,
    "Accent",
    (127, 201, 127),
    (190, 174, 212),
    (253, 192, 134),
    (255, 255, 153),
    (56, 108, 176),
    (240, 2, 127),
    (191, 91, 23),
    (102, 102, 102)
);

define_qualitative!(
    Dark2,
    "Dark2",
    (27, 158, 119),
    (217, 95, 2),
    (117, 112, 179),
    (231, 41, 138),
    (102, 166, 30),
    (230, 171, 2),
    (166, 118, 29),
    (102, 102, 102)
);

// -- Colormap registry --

/// Look up a colormap by name. Returns `None` if the name is not recognized.
///
/// # Example
///
/// ```
/// use ratatui_plt::colormap::{get_colormap, Colormap};
///
/// let cmap = get_colormap("viridis").unwrap();
/// let color = cmap.color_at(0.5);
/// ```
pub fn get_colormap(name: &str) -> Option<Box<dyn Colormap>> {
    match name.to_lowercase().as_str() {
        // Perceptually uniform
        "viridis" => Some(Box::new(Viridis)),
        "plasma" => Some(Box::new(Plasma)),
        "inferno" => Some(Box::new(Inferno)),
        "magma" => Some(Box::new(Magma)),
        "cividis" => Some(Box::new(Cividis)),
        // Sequential
        "hot" => Some(Box::new(Hot)),
        "spring" => Some(Box::new(Spring)),
        "summer" => Some(Box::new(Summer)),
        "autumn" => Some(Box::new(Autumn)),
        "winter" => Some(Box::new(Winter)),
        "grayscale" | "gray" | "grey" => Some(Box::new(Grayscale)),
        // Sequential single-hue
        "blues" => Some(Box::new(Blues)),
        "greens" => Some(Box::new(Greens)),
        "reds" => Some(Box::new(Reds)),
        "oranges" => Some(Box::new(Oranges)),
        "purples" => Some(Box::new(Purples)),
        "greys" => Some(Box::new(Greys)),
        // ColorBrewer sequential
        "ylorbr" => Some(Box::new(YlOrBr)),
        "ylorrd" => Some(Box::new(YlOrRd)),
        "orrd" => Some(Box::new(OrRd)),
        "purd" => Some(Box::new(PuRd)),
        "rdpu" => Some(Box::new(RdPu)),
        "bupu" => Some(Box::new(BuPu)),
        "gnbu" => Some(Box::new(GnBu)),
        "pubu" => Some(Box::new(PuBu)),
        "ylgnbu" => Some(Box::new(YlGnBu)),
        "pubugn" => Some(Box::new(PuBuGn)),
        "bugn" => Some(Box::new(BuGn)),
        "ylgn" => Some(Box::new(YlGn)),
        // Diverging
        "coolwarm" => Some(Box::new(Coolwarm)),
        "rdbu" => Some(Box::new(RdBu)),
        "seismic" => Some(Box::new(Seismic)),
        "piyg" => Some(Box::new(PiYG)),
        "prgn" => Some(Box::new(PRGn)),
        "brbg" => Some(Box::new(BrBG)),
        "puor" => Some(Box::new(PuOr)),
        "rdgy" => Some(Box::new(RdGy)),
        "rdylbu" => Some(Box::new(RdYlBu)),
        "rdylgn" => Some(Box::new(RdYlGn)),
        "spectral" => Some(Box::new(Spectral)),
        // Cyclic
        "twilight" => Some(Box::new(Twilight)),
        "hsv" => Some(Box::new(Hsv)),
        // Rainbow
        "jet" => Some(Box::new(Jet)),
        "turbo" => Some(Box::new(Turbo)),
        // Qualitative
        "tab20" => Some(Box::new(Tab20)),
        "tab20b" => Some(Box::new(Tab20b)),
        "tab20c" => Some(Box::new(Tab20c)),
        "paired" => Some(Box::new(Paired)),
        "set1" => Some(Box::new(Set1)),
        "set2" => Some(Box::new(Set2)),
        "set3" => Some(Box::new(Set3)),
        "pastel1" => Some(Box::new(Pastel1)),
        "pastel2" => Some(Box::new(Pastel2)),
        "accent" => Some(Box::new(Accent)),
        "dark2" => Some(Box::new(Dark2)),
        // Spiral
        "cubehelix" => Some(Box::new(Cubehelix)),
        _ => None,
    }
}

/// All available named colormaps in the registry.
pub fn colormap_names() -> &'static [&'static str] {
    &[
        "viridis", "plasma", "inferno", "magma", "cividis", "hot", "spring", "summer", "autumn",
        "winter", "blues", "greens", "reds", "oranges", "purples", "greys", "ylorbr", "ylorrd",
        "orrd", "purd", "rdpu", "bupu", "gnbu", "pubu", "ylgnbu", "pubugn", "bugn", "ylgn",
        "coolwarm", "rdbu", "seismic", "piyg", "prgn", "brbg", "puor", "rdgy", "rdylbu",
        "rdylgn", "spectral", "twilight", "hsv", "jet", "turbo", "tab20", "tab20b", "tab20c",
        "paired", "set1", "set2", "set3", "pastel1", "pastel2", "accent", "dark2", "cubehelix",
    ]
}

/// Controls how out-of-range values are displayed on the colorbar.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ColorbarExtend {
    /// No extensions at either end.
    #[default]
    Neither,
    /// Triangular extension at the minimum end.
    Min,
    /// Triangular extension at the maximum end.
    Max,
    /// Triangular extensions at both ends.
    Both,
}

/// Resample a colormap at `n` evenly-spaced points, producing a [`ListedColormap`].
pub fn resample(cmap: &dyn Colormap, n: usize) -> ListedColormap {
    let n = n.max(2);
    let stops: Vec<(f64, Color)> = (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            (t, cmap.color_at(t))
        })
        .collect();
    ListedColormap::new(format!("{}_resampled", cmap.name()), stops)
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
    /// Color for tick labels (`None` = use theme foreground).
    label_color: Option<Color>,
    /// How to display out-of-range values.
    extend: ColorbarExtend,
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
            label_color: None,
            extend: ColorbarExtend::Neither,
        }
    }

    /// Set the label color.
    pub fn label_color(mut self, color: Color) -> Self {
        self.label_color = Some(color);
        self
    }

    /// Set extension mode for out-of-range values.
    pub fn extend(mut self, extend: ColorbarExtend) -> Self {
        self.extend = extend;
        self
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
        let theme = crate::theme::Theme::get_default();

        let bar_width = self.width.min(area.width.saturating_sub(6));
        let label_x = area.x + bar_width + 1;

        // Reserve rows for extend triangles
        let top_ext = matches!(self.extend, ColorbarExtend::Max | ColorbarExtend::Both) as u16;
        let bot_ext = matches!(self.extend, ColorbarExtend::Min | ColorbarExtend::Both) as u16;
        let grad_start = area.y + top_ext;
        let grad_height = area.height.saturating_sub(top_ext + bot_ext);

        // Draw extend triangle at top (max)
        if top_ext > 0 {
            let color = self.cmap.color_at(1.0);
            let mid = area.x + bar_width / 2;
            if mid < area.x + area.width {
                buf[(mid, area.y)].set_char(theme.chars.colorbar.extend_max).set_fg(color);
            }
        }

        // Draw color gradient
        for row in 0..grad_height {
            let t = 1.0 - row as f64 / grad_height.saturating_sub(1).max(1) as f64;
            let color = self.cmap.color_at(t);
            for col in 0..bar_width {
                let x = area.x + col;
                let y = grad_start + row;
                if x < area.x + area.width && y < area.y + area.height {
                    buf[(x, y)].set_char(theme.chars.fill.solid).set_fg(color);
                }
            }
        }

        // Draw extend triangle at bottom (min)
        if bot_ext > 0 {
            let color = self.cmap.color_at(0.0);
            let mid = area.x + bar_width / 2;
            let y = grad_start + grad_height;
            if mid < area.x + area.width && y < area.y + area.height {
                buf[(mid, y)].set_char(theme.chars.colorbar.extend_min).set_fg(color);
            }
        }

        // Draw tick labels
        if area.width > bar_width + 1 {
            let label_width = (area.width - bar_width - 1) as usize;
            for i in 0..self.n_ticks {
                let t = i as f64 / (self.n_ticks - 1).max(1) as f64;
                let row = ((1.0 - t) * grad_height.saturating_sub(1).max(1) as f64).round() as u16;
                let value = self.vmin + t * (self.vmax - self.vmin);
                let label = format!("{:.2}", value);
                let label = if label.len() > label_width {
                    &label[..label_width]
                } else {
                    &label
                };
                let y = grad_start + row;
                if y < area.y + area.height {
                    for (j, ch) in label.chars().enumerate() {
                        let x = label_x + j as u16;
                        if x < area.x + area.width {
                            buf[(x, y)]
                                .set_char(ch)
                                .set_style(Style::default().fg(self.label_color.unwrap_or(theme.foreground)));
                        }
                    }
                }
            }
        }
    }
}
