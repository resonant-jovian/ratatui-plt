//! Plot widgets for ratatui.
//!
//! Each widget implements `Widget for &WidgetName` following ratatui conventions.
//! All widgets use a builder pattern for configuration.
//!
//! ## Widget Categories
//!
//! - **Core** — LinePlot, ScatterPlot, BarChart, AreaChart, Histogram, PieChart, Heatmap, ImagePlot
//! - **Statistical** — BoxPlot, BoxenPlot, ViolinPlot, ErrorBarPlot, BandPlot, EcdfPlot, StripPlot, SwarmPlot, RugPlot
//! - **Scientific** — ContourPlot, StreamPlot, VectorField, StemPlot, StairsPlot, EventPlot, HexbinPlot, Hist2D, Pcolormesh
//! - **Financial** — CandlestickChart, WaterfallChart, GanttChart, GaugeChart
//! - **Hierarchical & Relational** — Dendrogram, Treemap, Sunburst, SankeyDiagram, FunnelChart, NetworkPlot, ParallelCoords
//! - **Polar & Specialized** — RadialPlot (also RadarPlot), TernaryPlot
//! - **3D** — Surface3D, Scatter3D, Bar3D, Contour3D, Quiver3D
//! - **Triangulation** — TriPlot, TriContour, TriColor
//! - **Layout** — MultiPanel, FacetGrid, TwinAxes, InsetAxes, JointPlot
//! - **Interaction** — Crosshair, SpanSelector, RectangleSelector

// ── Core ────────────────────────────────────────────────────────────────────
pub mod area_chart;
pub mod bar_chart;
pub mod data_table;
pub mod heatmap;
pub mod histogram;
pub mod image_plot;
pub mod line_plot;
pub mod pie_chart;
pub mod scatter_plot;

// ── Statistical ─────────────────────────────────────────────────────────────
pub mod band;
pub mod box_plot;
pub mod boxen;
pub mod ecdf;
pub mod error_bar;
#[cfg(feature = "statistics")]
pub mod kde_plot;
#[cfg(feature = "statistics")]
pub mod qq_plot;
#[cfg(feature = "statistics")]
pub mod regression_plot;
#[cfg(feature = "statistics")]
pub mod ridgeline;
pub mod dot_plot;
pub mod rug;
pub mod strip;
pub mod swarm;
pub mod violin_plot;

// ── Scientific ──────────────────────────────────────────────────────────────
pub mod contour;
pub mod event_plot;
pub mod hexbin;
pub mod horizon;
pub mod hist2d;
pub mod pcolormesh;
pub mod stairs;
pub mod stem_plot;
pub mod streamplot;
pub mod vector_field;

// ── Financial ───────────────────────────────────────────────────────────────
pub mod candlestick;
pub mod gantt;
pub mod gauge;
pub mod waterfall;

// ── Hierarchical & Relational ───────────────────────────────────────────────
pub mod clustermap;
pub mod dendrogram;
pub mod funnel;
pub mod icicle;
pub mod network;
pub mod parallel_categories;
pub mod parallel_coords;
pub mod sankey;
pub mod sunburst;
pub mod treemap;

// ── Polar & Specialized ─────────────────────────────────────────────────────
pub mod radial;
pub mod ternary;

// ── 3D ──────────────────────────────────────────────────────────────────────
pub mod bar3d;
pub mod contour3d;
pub mod line3d;
pub mod mesh3d;
pub mod quiver3d;
pub mod scatter3d;
pub mod surface3d;

// ── Triangulation ───────────────────────────────────────────────────────────
pub mod tricolor;
pub mod tricontour;
pub mod triplot;

// ── Layout ──────────────────────────────────────────────────────────────────
pub mod facet_grid;
pub mod inset;
pub mod joint_plot;
pub mod multi_panel;
#[cfg(feature = "statistics")]
pub mod pair_plot;
pub mod twin_axes;

// ── Interaction ─────────────────────────────────────────────────────────────
pub mod crosshair;
pub mod rect_selector;
pub mod span_selector;

// ── FFT (feature-gated) ────────────────────────────────────────────────────
#[cfg(feature = "fft")]
pub mod psd;
#[cfg(feature = "fft")]
pub mod spectrogram;
