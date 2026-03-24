//! Automatic color assignment for multi-series plots.
//!
//! When multiple data series are plotted together, each series needs a distinct color.
//! The [`ColorCycle`] provides matplotlib-style automatic color cycling using
//! perceptually distinct palettes.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::color_cycle::ColorCycle;
//!
//! let mut cycle = ColorCycle::default();
//! let c1 = cycle.next_color(); // tab10 blue
//! let c2 = cycle.next_color(); // tab10 orange
//! let c3 = cycle.next_color(); // tab10 green
//! ```

use ratatui::style::Color;

/// An automatic color cycle that assigns distinct colors to series.
///
/// Defaults to matplotlib's tab10 palette, which provides 10 perceptually
/// distinct colors suitable for most plots.
#[derive(Clone, Debug)]
pub struct ColorCycle {
    colors: Vec<Color>,
    index: usize,
}

impl Default for ColorCycle {
    /// matplotlib's tab10 palette.
    fn default() -> Self {
        Self {
            colors: vec![
                Color::Rgb(31, 119, 180),  // blue
                Color::Rgb(255, 127, 14),  // orange
                Color::Rgb(44, 160, 44),   // green
                Color::Rgb(214, 39, 40),   // red
                Color::Rgb(148, 103, 189), // purple
                Color::Rgb(140, 86, 75),   // brown
                Color::Rgb(227, 119, 194), // pink
                Color::Rgb(127, 127, 127), // gray
                Color::Rgb(188, 189, 34),  // olive
                Color::Rgb(23, 190, 207),  // cyan
            ],
            index: 0,
        }
    }
}

impl ColorCycle {
    /// Create a color cycle from a custom color list.
    pub fn new(colors: Vec<Color>) -> Self {
        Self { colors, index: 0 }
    }

    /// Get the next color in the cycle.
    pub fn next_color(&mut self) -> Color {
        if self.colors.is_empty() {
            return Color::White;
        }
        let color = self.colors[self.index % self.colors.len()];
        self.index += 1;
        color
    }

    /// Get a color at a specific index (wraps around).
    pub fn at(&self, index: usize) -> Color {
        if self.colors.is_empty() {
            return Color::White;
        }
        self.colors[index % self.colors.len()]
    }

    /// Reset the cycle to the beginning.
    pub fn reset(&mut self) {
        self.index = 0;
    }

    /// Number of colors before the cycle repeats.
    pub fn len(&self) -> usize {
        self.colors.len()
    }

    /// Whether the cycle has no colors.
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }
}
