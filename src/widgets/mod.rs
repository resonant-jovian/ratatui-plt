//! Plot widgets for ratatui.
//!
//! Each widget implements `Widget for &WidgetName` following ratatui conventions.
//! All widgets use a builder pattern for configuration.

pub mod band;
pub mod bar3d;
pub mod bar_chart;
pub mod box_plot;
pub mod boxen;
pub mod candlestick;
pub mod contour;
pub mod contour3d;
pub mod crosshair;
pub mod dendrogram;
pub mod ecdf;
pub mod error_bar;
pub mod event_plot;
pub mod heatmap;
pub mod hexbin;
pub mod hist2d;
pub mod histogram;
pub mod line_plot;
pub mod multi_panel;
pub mod parallel_coords;
pub mod pcolormesh;
pub mod pie_chart;
pub mod quiver3d;
pub mod radial;
pub mod rug;
pub mod scatter3d;
pub mod scatter_plot;
pub mod stacked_area;
pub mod stairs;
pub mod stem_plot;
pub mod streamplot;
pub mod strip;
pub mod surface3d;
pub mod swarm;
pub mod twin_axes;
pub mod vector_field;
pub mod violin_plot;
pub mod wireframe3d;
