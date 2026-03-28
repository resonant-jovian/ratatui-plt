//! Map ratatui-plt `Theme` to plotters style primitives.

use plotters::style::RGBAColor;
use ratatui::style::Color;

use crate::theme::Theme;

/// Convert a ratatui `Color` to a plotters `RGBAColor`.
pub fn to_plotters_color(color: Color) -> RGBAColor {
    let (r, g, b) = color_to_rgb(color);
    RGBAColor(r, g, b, 1.0)
}

/// Convert a ratatui `Color` to a plotters `RGBAColor` with alpha.
pub fn to_plotters_color_alpha(color: Color, alpha: f64) -> RGBAColor {
    let (r, g, b) = color_to_rgb(color);
    RGBAColor(r, g, b, alpha)
}

/// Get the background color from a theme as (r, g, b).
pub fn theme_bg_rgb(theme: &Theme) -> (u8, u8, u8) {
    color_to_rgb(theme.background)
}

/// Get the i-th color from the theme's color cycle as a plotters color.
pub fn cycle_color(theme: &Theme, idx: usize) -> RGBAColor {
    to_plotters_color(theme.color_cycle.at(idx))
}

/// Get the grid line color from the theme.
pub fn grid_color(theme: &Theme) -> RGBAColor {
    to_plotters_color(theme.grid_color)
}

/// Get the axis color from the theme.
pub fn axis_color(theme: &Theme) -> RGBAColor {
    to_plotters_color(theme.axis_color)
}

/// Get the foreground (text) color from the theme.
pub fn fg_color(theme: &Theme) -> RGBAColor {
    to_plotters_color(theme.foreground)
}

/// Get foreground color as an (r, g, b) tuple for direct use.
pub fn fg_rgb(theme: &Theme) -> (u8, u8, u8) {
    color_to_rgb(theme.foreground)
}

/// Map a ratatui `Color` to an (r, g, b) triple.
///
/// Named ANSI colors use standard terminal defaults.
fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::Gray => (170, 170, 170),
        Color::DarkGray => (118, 118, 118),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        Color::White => (229, 229, 229),
        Color::Indexed(_) => (170, 170, 170),
        Color::Reset => (204, 204, 204),
    }
}
