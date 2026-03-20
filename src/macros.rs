//! Convenience macros for quick plot construction.
//!
//! These macros reduce boilerplate when creating common plot configurations.

/// Create a [`Series`](crate::series::Series) from a name and data points.
///
/// # Example
///
/// ```
/// use ratatui_plt::series;
/// use ratatui::style::Color;
///
/// let s = series!("sin(x)", [(0.0, 0.0), (1.0, 0.84), (2.0, 0.91)]);
/// let s = series!("cos(x)", [(0.0, 1.0), (1.0, 0.54)], color = Color::Red);
/// ```
#[macro_export]
macro_rules! series {
    ($name:expr, [$( ($x:expr, $y:expr) ),* $(,)?]) => {{
        $crate::series::Series::new($name)
            .data(vec![$( ($x as f64, $y as f64) ),*])
    }};
    ($name:expr, [$( ($x:expr, $y:expr) ),* $(,)?], color = $color:expr) => {{
        $crate::series::Series::new($name)
            .data(vec![$( ($x as f64, $y as f64) ),*])
            .color($color)
    }};
    ($name:expr, $data:expr) => {{
        $crate::series::Series::new($name).data($data)
    }};
    ($name:expr, $data:expr, color = $color:expr) => {{
        $crate::series::Series::new($name).data($data).color($color)
    }};
}

/// Create a [`LinePlot`](crate::widgets::line_plot::LinePlot) from series.
///
/// # Example
///
/// ```
/// use ratatui_plt::{plot, series};
/// use ratatui::style::Color;
///
/// let s1 = series!("A", [(0.0, 0.0), (1.0, 1.0)]);
/// let s2 = series!("B", [(0.0, 1.0), (1.0, 0.0)]);
/// let p = plot!(s1, s2);
/// ```
#[macro_export]
macro_rules! plot {
    ($($series:expr),+ $(,)?) => {{
        let mut p = $crate::widgets::line_plot::LinePlot::new();
        $(p = p.series($series);)+
        p
    }};
    (title = $title:expr, $($series:expr),+ $(,)?) => {{
        let mut p = $crate::widgets::line_plot::LinePlot::new().title($title);
        $(p = p.series($series);)+
        p
    }};
}

/// Create a [`Heatmap`](crate::widgets::heatmap::Heatmap) from a 2D array and colormap.
///
/// # Example
///
/// ```
/// use ratatui_plt::{heatmap_widget, prelude::*};
///
/// let data = GridData::from_fn((-1.0, 1.0), (-1.0, 1.0), 20, 20, |x, y| x * y);
/// let h = heatmap_widget!(data);
/// ```
#[macro_export]
macro_rules! heatmap_widget {
    ($data:expr) => {{ $crate::widgets::heatmap::Heatmap::new($data) }};
    ($data:expr, $cmap:expr) => {{ $crate::widgets::heatmap::Heatmap::new($data).colormap($cmap) }};
}

/// Create a [`MultiPanel`](crate::widgets::multi_panel::MultiPanel) subplot grid.
///
/// # Example
///
/// ```
/// use ratatui_plt::subplot;
///
/// let panel = subplot!(2, 2);
/// ```
#[macro_export]
macro_rules! subplot {
    ($rows:expr, $cols:expr) => {{ $crate::widgets::multi_panel::MultiPanel::new($rows, $cols) }};
    ($rows:expr, $cols:expr, gap = $gap:expr) => {{ $crate::widgets::multi_panel::MultiPanel::new($rows, $cols).gap($gap) }};
}

/// Create a [`ListedColormap`](crate::colormap::ListedColormap) from color stops.
///
/// # Example
///
/// ```
/// use ratatui_plt::colormap_custom;
/// use ratatui::style::Color;
///
/// let cmap = colormap_custom!("diverging",
///     0.0 => Color::Blue,
///     0.5 => Color::White,
///     1.0 => Color::Red,
/// );
/// ```
#[macro_export]
macro_rules! colormap_custom {
    ($name:expr, $($t:expr => $color:expr),+ $(,)?) => {{
        $crate::colormap::ListedColormap::new($name, vec![
            $( ($t as f64, $color) ),+
        ])
    }};
}
