//! Shared plotters helpers for chart configuration and drawing.
//!
//! Color conversion, mesh configuration, and common drawing
//! operations used by all widgets.

use plotters::coord::Shift;
use plotters::prelude::*;
use plotters::style::RGBAColor;
use ratatui::style::Color;

use crate::axis::Axis;
use crate::backend::TinySkiaDrawingBackend;
use crate::theme::Theme;

/// Standard margin (in pixels) around chart content.
pub const CHART_MARGIN: u32 = 10;
/// Label area size (pixels) for bottom x-axis.
pub const X_LABEL_SIZE: u32 = 35;
/// Label area size (pixels) for left y-axis.
pub const Y_LABEL_SIZE: u32 = 50;

// ── Color conversion ───────────────────────────────────────────

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

/// Map a ratatui `Color` to an (r, g, b) triple.
pub fn color_to_rgb(color: Color) -> (u8, u8, u8) {
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

// ── Chart building ─────────────────────────────────────────────

/// Build a 2D Cartesian chart configured from ratatui-plt Axis/Theme.
///
/// Returns a configured `ChartContext` with axes, grid, and labels
/// drawn. The caller adds data series via `chart.draw_series(...)`.
#[allow(clippy::type_complexity)]
pub fn build_cartesian_2d<'a>(
    root: &'a DrawingArea<TinySkiaDrawingBackend, Shift>,
    x_axis: &Axis,
    y_axis: &Axis,
    title: Option<&str>,
    theme: &Theme,
    x_range: std::ops::Range<f64>,
    y_range: std::ops::Range<f64>,
) -> Result<
    ChartContext<
        'a,
        TinySkiaDrawingBackend,
        Cartesian2d<
            plotters::coord::types::RangedCoordf64,
            plotters::coord::types::RangedCoordf64,
        >,
    >,
    DrawingAreaErrorKind<
        <TinySkiaDrawingBackend as DrawingBackend>::ErrorType,
    >,
> {
    let fg = fg_color(theme);
    let grid = grid_color(theme);

    let mut builder = ChartBuilder::on(root);
    builder
        .margin(CHART_MARGIN)
        .x_label_area_size(X_LABEL_SIZE)
        .y_label_area_size(Y_LABEL_SIZE);

    if let Some(t) = title {
        builder.caption(t, ("sans-serif", 16.0).into_font().color(&fg));
    }

    let mut chart =
        builder.build_cartesian_2d(x_range, y_range)?;

    // Configure mesh (grid lines, ticks, labels).
    let mut mesh = chart.configure_mesh();

    if x_axis.grid {
        mesh.x_max_light_lines(0);
    } else {
        mesh.disable_x_mesh();
    }
    if y_axis.grid {
        mesh.y_max_light_lines(0);
    } else {
        mesh.disable_y_mesh();
    }

    mesh.axis_style(ShapeStyle::from(fg).stroke_width(1))
        .label_style(("sans-serif", 12.0).into_font().color(&fg))
        .light_line_style(ShapeStyle::from(grid).stroke_width(1));

    if let Some(label) = &x_axis.label {
        mesh.x_desc(label.as_str());
    }
    if let Some(label) = &y_axis.label {
        mesh.y_desc(label.as_str());
    }

    mesh.draw()?;

    Ok(chart)
}
