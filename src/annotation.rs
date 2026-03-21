//! Text annotations for plots.
//!
//! Allows placing labeled text at specific data coordinates,
//! optionally with an arrow pointing to a target position.

use ratatui::style::Color;

/// Arrow style for annotations.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default)]
pub enum ArrowStyle {
    /// Simple line: ─
    Simple,
    /// Arrow with head: →
    Arrow,
    /// No arrow (just text).
    #[default]
    None,
}

/// A text annotation placed on a plot.
///
/// # Example
///
/// ```
/// use ratatui_plt::annotation::{Annotation, ArrowStyle};
/// use ratatui::style::Color;
///
/// let ann = Annotation::new("Peak", 3.14, 1.0)
///     .arrow_to(3.14, 0.0)
///     .color(Color::Yellow);
/// ```
#[derive(Clone, Debug)]
pub struct Annotation {
    /// Annotation text.
    pub text: String,
    /// Text position in data coordinates (x, y).
    pub text_x: f64,
    pub text_y: f64,
    /// Optional target position for arrow (data coordinates).
    pub target: Option<(f64, f64)>,
    /// Arrow style.
    pub arrow_style: ArrowStyle,
    /// Text color.
    pub color: Color,
}

impl Annotation {
    /// Create a new annotation at (x, y) with the given text.
    pub fn new(text: impl Into<String>, x: f64, y: f64) -> Self {
        Self {
            text: text.into(),
            text_x: x,
            text_y: y,
            target: None,
            arrow_style: ArrowStyle::None,
            color: Color::White,
        }
    }

    /// Set an arrow target position.
    pub fn arrow_to(mut self, x: f64, y: f64) -> Self {
        self.target = Some((x, y));
        self.arrow_style = ArrowStyle::Arrow;
        self
    }

    /// Set the arrow style.
    pub fn arrow_style(mut self, style: ArrowStyle) -> Self {
        self.arrow_style = style;
        self
    }

    /// Set the text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Get the arrow character for drawing from source to target.
    pub fn arrow_char(&self, dx: f64, dy: f64) -> char {
        match &self.arrow_style {
            ArrowStyle::None => ' ',
            ArrowStyle::Simple => '─',
            ArrowStyle::Arrow => {
                if dx.abs() > dy.abs() * 2.0 {
                    if dx > 0.0 { '→' } else { '←' }
                } else if dy.abs() > dx.abs() * 2.0 {
                    if dy > 0.0 { '↓' } else { '↑' }
                } else if dx > 0.0 {
                    if dy > 0.0 { '↘' } else { '↗' }
                } else if dy > 0.0 {
                    '↙'
                } else {
                    '↖'
                }
            }
        }
    }
}
