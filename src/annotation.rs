//! Text annotations for plots.
//!
//! Allows placing labeled text at specific data coordinates,
//! optionally with an arrow pointing to a target position.

use ratatui::style::Color;

/// Convert a number (1-20) to its enclosed circled variant.
/// Returns the original number as a string if out of range.
pub fn enclosed_number(n: usize) -> String {
    match n {
        1 => "\u{2460}".to_string(),  // ①
        2 => "\u{2461}".to_string(),  // ②
        3 => "\u{2462}".to_string(),  // ③
        4 => "\u{2463}".to_string(),  // ④
        5 => "\u{2464}".to_string(),  // ⑤
        6 => "\u{2465}".to_string(),  // ⑥
        7 => "\u{2466}".to_string(),  // ⑦
        8 => "\u{2467}".to_string(),  // ⑧
        9 => "\u{2468}".to_string(),  // ⑨
        10 => "\u{2469}".to_string(), // ⑩
        11 => "\u{246A}".to_string(), // ⑪
        12 => "\u{246B}".to_string(), // ⑫
        13 => "\u{246C}".to_string(), // ⑬
        14 => "\u{246D}".to_string(), // ⑭
        15 => "\u{246E}".to_string(), // ⑮
        16 => "\u{246F}".to_string(), // ⑯
        17 => "\u{2470}".to_string(), // ⑰
        18 => "\u{2471}".to_string(), // ⑱
        19 => "\u{2472}".to_string(), // ⑲
        20 => "\u{2473}".to_string(), // ⑳
        _ => n.to_string(),
    }
}

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
    /// Text color (`None` = use theme annotation color).
    pub color: Option<Color>,
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
            color: None,
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
        self.color = Some(color);
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
