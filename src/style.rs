//! Visual style configuration for plots.
//!
//! Provides line styles, marker shapes, fill patterns, and overall plot styling.

use ratatui::style::Color;

/// Line drawing style.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Custom on/off lengths (in sub-pixel steps).
    /// E.g. `vec![6, 3]` means 6 on, 3 off, repeating.
    Custom(Vec<u16>),
}

/// Line thickness.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Four-pointed star: ✦
    FourPointedStar,
    /// Six-pointed star: ✶
    SixPointedStar,
    /// Eight-pointed star: ✴
    EightPointedStar,
    /// Sparkle: ❖
    Sparkle,
    /// Small circle: ∘
    SmallCircle,
    /// Ring (bullseye/double circle): ◎
    Ring,
    /// Triangle down: ▼
    TriangleDown,
    /// Triangle right: ▶
    TriangleRight,
    /// Triangle left: ◀
    TriangleLeft,
    /// Filled diamond: ◆
    FilledDiamond,
    /// Circle half left: ◐
    CircleHalfLeft,
    /// Circle half right: ◑
    CircleHalfRight,
    /// Circle half top: ◓
    CircleHalfTop,
    /// Circle half bottom: ◒
    CircleHalfBottom,
    /// Pentagon: ⬠
    Pentagon,
    /// Hexagon: ⬡
    Hexagon,
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
            Self::FourPointedStar => '\u{2726}',  // ✦
            Self::SixPointedStar => '\u{2736}',   // ✶
            Self::EightPointedStar => '\u{2734}', // ✴
            Self::Sparkle => '\u{2756}',          // ❖
            Self::SmallCircle => '\u{2218}',      // ∘
            Self::Ring => '\u{25CE}',             // ◎
            Self::TriangleDown => '\u{25BC}',     // ▼
            Self::TriangleRight => '\u{25B6}',    // ▶
            Self::TriangleLeft => '\u{25C0}',     // ◀
            Self::FilledDiamond => '\u{25C6}',    // ◆
            Self::CircleHalfLeft => '\u{25D0}',   // ◐
            Self::CircleHalfRight => '\u{25D1}',  // ◑
            Self::CircleHalfTop => '\u{25D3}',    // ◓
            Self::CircleHalfBottom => '\u{25D2}', // ◒
            Self::Pentagon => '\u{2B20}',         // ⬠
            Self::Hexagon => '\u{2B21}',          // ⬡
        }
    }
}

/// Hatch pattern for filled regions (bars, histograms, etc.).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HatchPattern {
    /// No hatch.
    None,
    /// Forward diagonal: /
    Forward,
    /// Backward diagonal: \
    Backward,
    /// Vertical lines: |
    Vertical,
    /// Horizontal lines: -
    Horizontal,
    /// Cross: +
    Cross,
    /// Diagonal cross: x
    DiagonalCross,
    /// Dots: ·
    Dots,
    /// Custom character pattern.
    Custom(char),
}

impl HatchPattern {
    /// Return the hatch character to draw at the given row/col position,
    /// or `None` if this cell should be skipped.
    pub fn char_at(&self, row: u16, col: u16) -> Option<char> {
        match self {
            Self::None => Option::None,
            Self::Forward => {
                if (row + col).is_multiple_of(3) {
                    Some('╱')
                } else {
                    Option::None
                }
            }
            Self::Backward => {
                if (row + 2u16.wrapping_mul(col)).is_multiple_of(3) {
                    Some('╲')
                } else {
                    Option::None
                }
            }
            Self::Vertical => {
                if col.is_multiple_of(3) {
                    Some('│')
                } else {
                    Option::None
                }
            }
            Self::Horizontal => {
                if row.is_multiple_of(2) {
                    Some('─')
                } else {
                    Option::None
                }
            }
            Self::Cross => {
                if col.is_multiple_of(3) || row.is_multiple_of(2) {
                    if col.is_multiple_of(3) && row.is_multiple_of(2) {
                        Some('┼')
                    } else if col.is_multiple_of(3) {
                        Some('│')
                    } else {
                        Some('─')
                    }
                } else {
                    Option::None
                }
            }
            Self::DiagonalCross => {
                if (row + col).is_multiple_of(3) || (row + 2u16.wrapping_mul(col)).is_multiple_of(3)
                {
                    Some('×')
                } else {
                    Option::None
                }
            }
            Self::Dots => {
                if (row + col).is_multiple_of(2) {
                    Some('·')
                } else {
                    Option::None
                }
            }
            Self::Custom(ch) => {
                if (row + col).is_multiple_of(2) {
                    Some(*ch)
                } else {
                    Option::None
                }
            }
        }
    }
}

/// Fill style for regions between curves or under curves.
#[derive(Clone, Debug)]
pub struct FillStyle {
    /// Fill color (`None` = use theme foreground).
    pub color: Option<Color>,
    /// Opacity approximation (uses different fill characters).
    pub density: FillDensity,
    /// Optional hatch pattern overlay.
    pub hatch: Option<HatchPattern>,
}

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            color: None,
            density: FillDensity::Medium,
            hatch: None,
        }
    }
}

/// Fill density for region fills, approximating opacity in terminal.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Resolve the fill character using the theme's [`FillChars`](crate::chars::FillChars).
    pub fn char_with(&self, chars: &crate::chars::FillChars) -> char {
        match self {
            Self::Light => chars.light,
            Self::Medium => chars.medium,
            Self::Dense => chars.dense,
            Self::Solid => chars.solid,
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
#[derive(Clone, Debug, Default)]
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
