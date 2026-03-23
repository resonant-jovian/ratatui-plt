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
pub mod facet_grid;
pub mod funnel;
pub mod gantt;
pub mod gauge;
pub mod heatmap;
pub mod hexbin;
pub mod hist2d;
pub mod histogram;
pub mod image_plot;
pub mod inset;
pub mod joint_plot;
pub mod line_plot;
pub mod multi_panel;
pub mod network;
pub mod parallel_coords;
pub mod pcolormesh;
pub mod pie_chart;
pub mod quiver3d;
pub mod radial;
pub mod rect_selector;
pub mod rug;
pub mod sankey;
pub mod scatter3d;
pub mod scatter_plot;
pub mod span_selector;
pub mod stacked_area;
pub mod stairs;
pub mod stem_plot;
pub mod streamplot;
pub mod strip;
pub mod sunburst;
pub mod surface3d;
pub mod swarm;
pub mod ternary;
pub mod treemap;
pub mod tricolor;
pub mod tricontour;
pub mod triplot;
pub mod twin_axes;
pub mod vector_field;
pub mod violin_plot;
pub mod waterfall;
pub mod wireframe3d;

#[cfg(feature = "fft")]
pub mod psd;
#[cfg(feature = "fft")]
pub mod spectrogram;
