//! Shared chart-building helpers for PlottersRenderable
//! implementations.
//!
//! These helpers configure plotters `ChartBuilder` from ratatui-plt
//! `Axis` and `Theme` settings, so each widget only needs to provide
//! its data-specific rendering.

use plotters::coord::Shift;
use plotters::prelude::*;

use super::backend::TinySkiaDrawingBackend;
use super::theme_bridge;
use crate::axis::Axis;
use crate::theme::Theme;

/// Standard margin (in pixels) around chart content.
pub const CHART_MARGIN: u32 = 10;
/// Label area size (pixels) for bottom x-axis.
pub const X_LABEL_SIZE: u32 = 35;
/// Label area size (pixels) for left y-axis.
pub const Y_LABEL_SIZE: u32 = 50;

/// Build a 2D Cartesian chart configured from ratatui-plt Axis/Theme.
///
/// Returns a configured `ChartContext` with axes, grid, and labels
/// drawn. The caller adds data series via `chart.draw_series(...)`.
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
    let fg = theme_bridge::fg_color(theme);
    let grid = theme_bridge::grid_color(theme);

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
