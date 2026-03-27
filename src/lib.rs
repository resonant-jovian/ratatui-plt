//! # ratatui-plt
//!
//! Scientific visualization widgets for [ratatui](https://ratatui.rs/) — matplotlib for the terminal.
//!
//! `ratatui-plt` provides a comprehensive suite of configurable plot widgets, colormaps,
//! axis systems, and layout tools designed for scientific computing and simulation monitoring.
//! Built primarily for astrophysical applications (Vlasov-Poisson solvers, phase-space analysis),
//! it works anywhere you need publication-quality terminal plots.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ratatui_plt::prelude::*;
//!
//! let series = Series::new("sin(x)")
//!     .data((0..100).map(|i| {
//!         let x = i as f64 * 0.1;
//!         (x, x.sin())
//!     }).collect())
//!     .color(Color::Cyan);
//!
//! let plot = LinePlot::new()
//!     .series(series)
//!     .title("Sine Wave")
//!     .x_axis(Axis::new().label("x"))
//!     .y_axis(Axis::new().label("y"));
//!
//! // Render with ratatui:
//! // frame.render_widget(&plot, area);
//! ```
//!
//! ## Features
//!
//! - **2D Plots**: Line, scatter, heatmap, histogram, bar chart, contour, stem, error bar,
//!   box plot, vector field, hexbin
//! - **3D Plots**: Surface, wireframe, scatter with interactive camera control
//! - **Colormaps**: Viridis, plasma, inferno, magma, cividis, coolwarm, and more
//! - **Normalization**: Linear, log, symlog, power, two-slope — essential for multi-scale data
//! - **Axis System**: Auto-ticking, log/symlog scales, aspect ratio control, twin axes
//! - **Layout**: Multi-panel GridSpec-like subplot grids with configurable ratios
//! - **Macros**: `series!`, `plot!`, `heatmap!`, `subplot!` for quick construction

pub mod annotation;
pub mod axis;
pub mod brushing;
pub mod chars;
pub mod collections;
pub mod color_cycle;
pub mod colormap;
pub mod config;
pub mod drawing;
pub mod export;
pub mod frame;
pub mod legend;
pub mod macros;
pub mod mathtext;
pub mod norm;
pub mod picking;
pub mod plot_buffer;
pub mod prelude;
pub mod series;
pub mod spines;
pub mod style;
pub mod theme;
pub mod ticker;
pub mod transform;
pub mod triangulation;
pub mod widgets;

pub mod linked_view;

#[cfg(feature = "kitty")]
pub mod kitty_backend;
#[cfg(feature = "sixel")]
pub mod sixel_backend;

#[cfg(feature = "statistics")]
pub mod statistics;

#[cfg(feature = "fft")]
pub mod fft;

#[cfg(feature = "chrono")]
pub mod date_axis;

#[cfg(feature = "async")]
pub mod animation;
#[cfg(feature = "async")]
pub mod async_data;
#[cfg(feature = "async")]
pub mod compute;
