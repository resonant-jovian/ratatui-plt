//! Plot widgets for ratatui.
//!
//! Each widget implements `Widget for &WidgetName` following ratatui conventions.
//! All widgets use a builder pattern for configuration.

pub mod bar_chart;
pub mod box_plot;
pub mod contour;
pub mod error_bar;
pub mod event_plot;
pub mod heatmap;
pub mod hexbin;
pub mod hist2d;
pub mod histogram;
pub mod line_plot;
pub mod multi_panel;
pub mod pie_chart;
pub mod radial;
pub mod scatter3d;
pub mod scatter_plot;
pub mod stacked_area;
pub mod stem_plot;
pub mod streamplot;
pub mod surface3d;
pub mod twin_axes;
pub mod vector_field;
pub mod violin_plot;
pub mod wireframe3d;
