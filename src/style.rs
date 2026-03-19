//! Visual style configuration for plots.
//!
//! Provides line styles, marker shapes, fill patterns, and overall plot styling.

use ratatui::style::Color;

/// Line drawing style.
#[derive(Clone, Debug)]
pub struct LineStyle {
    /// Dash pattern.
    pub pattern: DashPattern,
    /// Line thickness.
    pub thickness: Thickness,
}

impl Default for LineStyle {
    fn default() -> Self {
        Self {
            pattern: DashPattern::Solid,
            thickness: Thickness::Normal,
        }
    }
}

impl LineStyle {
    /// Solid line.
    pub fn solid() -> Self {
        Self {
            pattern: DashPattern::Solid,
            ..Default::default()
        }
    }

    /// Dashed line.
    pub fn dashed() -> Self {
        Self {
            pattern: DashPattern::Dashed,
            ..Default::default()
        }
    }

    /// Dotted line.
    pub fn dotted() -> Self {
        Self {
            pattern: DashPattern::Dotted,
            ..Default::default()
        }
    }

    /// Set the thickness.
    pub fn thickness(mut self, t: Thickness) -> Self {
        self.thickness = t;
        self
    }
}

/// Dash pattern for lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DashPattern {
    /// Continuous line: ────
    Solid,
    /// Dashed line: ── ── ──
    Dashed,
    /// Dotted line: · · · ·
    Dotted,
    /// Dash-dot: ── · ── ·
    DashDot,
}

/// Line thickness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Thickness {
    /// Thin line (single-width characters).
    Thin,
    /// Normal line.
    Normal,
    /// Thick line (bold or double-width).
    Thick,
}

/// Marker shapes for data points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerShape {
    /// Single dot: ·
    Dot,
    /// Cross: ×
    Cross,
    /// Plus: +
    Plus,
    /// Circle: ○
    Circle,
    /// Filled circle: ●
    FilledCircle,
    /// Triangle: △
    Triangle,
    /// Square: □
    Square,
    /// Filled square: ■
    FilledSquare,
    /// Diamond: ◇
    Diamond,
    /// Star: ★
    Star,
    /// Braille pattern (sub-character resolution).
    Braille,
}

impl MarkerShape {
    /// Get the Unicode character for this marker.
    pub fn char(&self) -> char {
        match self {
            Self::Dot => '·',
            Self::Cross => '×',
            Self::Plus => '+',
            Self::Circle => '○',
            Self::FilledCircle => '●',
            Self::Triangle => '△',
            Self::Square => '□',
            Self::FilledSquare => '■',
            Self::Diamond => '◇',
            Self::Star => '★',
            Self::Braille => '⣿',
        }
    }
}

/// Fill style for regions between curves or under curves.
#[derive(Clone, Debug)]
pub struct FillStyle {
    /// Fill color.
    pub color: Color,
    /// Opacity approximation (uses different fill characters).
    pub density: FillDensity,
}

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            color: Color::White,
            density: FillDensity::Medium,
        }
    }
}

/// Fill density for region fills, approximating opacity in terminal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FillDensity {
    /// Light fill: ░
    Light,
    /// Medium fill: ▒
    Medium,
    /// Dense fill: ▓
    Dense,
    /// Solid fill: █
    Solid,
}

impl FillDensity {
    /// Get the block character for this density.
    pub fn char(&self) -> char {
        match self {
            Self::Light => '░',
            Self::Medium => '▒',
            Self::Dense => '▓',
            Self::Solid => '█',
        }
    }
}

/// Overall plot styling.
#[derive(Clone, Debug)]
pub struct PlotStyle {
    /// Plot title (rendered at top).
    pub title: Option<String>,
    /// Whether to draw a border around the plot area.
    pub border: bool,
    /// Background color for the plot area.
    pub bg_color: Option<Color>,
    /// Margin in characters around the plot content.
    pub margin: Margin,
}

impl Default for PlotStyle {
    fn default() -> Self {
        Self {
            title: None,
            border: true,
            bg_color: None,
            margin: Margin::default(),
        }
    }
}

/// Margin in characters.
#[derive(Clone, Debug)]
#[derive(Default)]
pub struct Margin {
    pub top: u16,
    pub bottom: u16,
    pub left: u16,
    pub right: u16,
}


impl Margin {
    /// Uniform margin on all sides.
    pub fn uniform(n: u16) -> Self {
        Self {
            top: n,
            bottom: n,
            left: n,
            right: n,
        }
    }
}
