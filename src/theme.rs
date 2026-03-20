//! Themes and stylesheets for consistent plot styling.
//!
//! Themes control colors, fonts, grid visibility, and other visual properties
//! across all widgets. Inspired by matplotlib's `plt.style.use()`.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::theme::Theme;
//!
//! // Use the dark theme (good for most terminals)
//! let theme = Theme::dark();
//!
//! // Or set it as the global default
//! Theme::set_default(Theme::dark());
//! ```

use ratatui::style::Color;

use crate::color_cycle::ColorCycle;
use crate::style::DashPattern;

/// A theme controlling the visual appearance of all plot elements.
#[derive(Clone, Debug)]
pub struct Theme {
    /// Background color for the plot area.
    pub background: Color,
    /// Default text color (titles, labels, tick marks).
    pub foreground: Color,
    /// Color for grid lines.
    pub grid_color: Color,
    /// Color for axis borders/spines.
    pub axis_color: Color,
    /// Color cycle for auto-coloring series.
    pub color_cycle: ColorCycle,
    /// Whether to show grid lines by default.
    pub grid_visible: bool,
    /// Grid line dash pattern.
    pub grid_pattern: DashPattern,
    /// Whether titles should be bold.
    pub bold_title: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Dark theme — light text on dark background. Good for most terminals.
    pub fn dark() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::White,
            grid_color: Color::Rgb(60, 60, 60),
            axis_color: Color::Gray,
            color_cycle: ColorCycle::default(),
            grid_visible: true,
            grid_pattern: DashPattern::Dotted,
            bold_title: true,
        }
    }

    /// Light theme — dark text, suitable for light terminal backgrounds.
    pub fn light() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::Black,
            grid_color: Color::Rgb(200, 200, 200),
            axis_color: Color::Gray,
            color_cycle: ColorCycle::default(),
            grid_visible: true,
            grid_pattern: DashPattern::Dotted,
            bold_title: true,
        }
    }

    /// Minimal theme — reduced chrome, clean look.
    pub fn minimal() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::White,
            grid_color: Color::DarkGray,
            axis_color: Color::DarkGray,
            color_cycle: ColorCycle::default(),
            grid_visible: false,
            grid_pattern: DashPattern::Dotted,
            bold_title: false,
        }
    }

    /// Publication theme — high contrast, monochrome-friendly.
    pub fn publication() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::White,
            grid_color: Color::DarkGray,
            axis_color: Color::White,
            color_cycle: ColorCycle::new(vec![
                Color::White,
                Color::Rgb(200, 200, 200),
                Color::Rgb(150, 150, 150),
                Color::Rgb(100, 100, 100),
            ]),
            grid_visible: true,
            grid_pattern: DashPattern::Dotted,
            bold_title: true,
        }
    }

    /// Solarized dark theme.
    pub fn solarized() -> Self {
        Self {
            background: Color::Rgb(0, 43, 54),
            foreground: Color::Rgb(131, 148, 150),
            grid_color: Color::Rgb(30, 70, 80),
            axis_color: Color::Rgb(88, 110, 117),
            color_cycle: ColorCycle::new(vec![
                Color::Rgb(38, 139, 210),  // blue
                Color::Rgb(211, 54, 130),  // magenta
                Color::Rgb(133, 153, 0),   // green
                Color::Rgb(203, 75, 22),   // orange
                Color::Rgb(108, 113, 196), // violet
                Color::Rgb(42, 161, 152),  // cyan
                Color::Rgb(181, 137, 0),   // yellow
                Color::Rgb(220, 50, 47),   // red
            ]),
            grid_visible: true,
            grid_pattern: DashPattern::Dotted,
            bold_title: true,
        }
    }

    /// Set the global default theme.
    pub fn set_default(theme: Theme) {
        DEFAULT_THEME.with(|t| {
            *t.borrow_mut() = theme;
        });
    }

    /// Get a clone of the current global default theme.
    pub fn get_default() -> Theme {
        DEFAULT_THEME.with(|t| t.borrow().clone())
    }
}

std::thread_local! {
    static DEFAULT_THEME: std::cell::RefCell<Theme> = std::cell::RefCell::new(Theme::dark());
}
