//! Plot widgets for ratatui.
//!
//! Each widget implements `Widget for &WidgetName` following ratatui conventions.
//! All widgets use a builder pattern for configuration.

pub mod bar_chart;
pub mod box_plot;
pub mod contour;
pub mod error_bar;
pub mod heatmap;
pub mod hexbin;
pub mod histogram;
pub mod line_plot;
pub mod multi_panel;
pub mod radial;
pub mod scatter3d;
pub mod scatter_plot;
pub mod stem_plot;
pub mod surface3d;
pub mod vector_field;
pub mod wireframe3d;
