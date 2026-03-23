//! Convenient re-exports for common usage.
//!
//! ```rust
//! use ratatui_plt::prelude::*;
//! ```

pub use ratatui::style::Color;

pub use crate::annotation::{Annotation, enclosed_number};
pub use crate::axis::{
    AspectRatio, Axis, Bounds, GridConfig, LabelPosition, LabelRotation, Scale, TickDirection,
    set_cell_aspect, terminal_cell_aspect,
};
pub use crate::brushing::{BrushState, SharedBrush, shared_brush};
pub use crate::collections::{LineCollection, PathCollection};
pub use crate::color_cycle::ColorCycle;
pub use crate::colormap::ColorbarExtend;
pub use crate::colormap::resample;
pub use crate::colormap::{
    // Qualitative
    Accent,
    Autumn,
    Blues,
    BrBG,
    BuGn,
    BuPu,
    Cividis,
    Colorbar,
    Colormap,
    Coolwarm,
    Dark2,
    GnBu,
    Grayscale,
    Greens,
    Greys,
    Hot,
    Hsv,
    Inferno,
    Jet,
    LinearSegmentedColormap,
    ListedColormap,
    Magma,
    OrRd,
    Oranges,
    PRGn,
    Paired,
    Pastel1,
    Pastel2,
    PiYG,
    Plasma,
    PuBu,
    PuBuGn,
    PuOr,
    PuRd,
    Purples,
    RdBu,
    RdGy,
    RdPu,
    RdYlBu,
    RdYlGn,
    Reds,
    Reversed,
    Seismic,
    Set1,
    Set2,
    Set3,
    Spectral,
    Spring,
    Summer,
    Tab20,
    Tab20b,
    Tab20c,
    Turbo,
    Twilight,
    Viridis,
    Winter,
    YlGn,
    YlGnBu,
    YlOrBr,
    YlOrRd,
    // Registry
    get_colormap,
};
pub use crate::config::{ConfigGuard, PlotConfig};
pub use crate::export::{
    buffer_to_ansi, buffer_to_svg, buffer_to_text, render_to_buffer, save_ansi, save_svg, save_text,
};
#[cfg(feature = "export")]
pub use crate::export::{ExportError, ExportOptions, buffer_to_png};
#[cfg(feature = "kitty")]
pub use crate::export::{buffer_to_kitty, print_kitty};
#[cfg(feature = "sixel")]
pub use crate::export::{buffer_to_sixel, print_sixel};
pub use crate::frame::{BorderStyle, DataBounds, PlotArea, PlotFrame, RefLineDash, ReferenceLine};
pub use crate::legend::{InteractiveLegend, Legend, LegendPosition, SharedLegendState, shared_legend_state};
pub use crate::linked_view::{SharedView, SharedViewState, shared_view};
pub use crate::norm::{
    AsinhNorm, BoundaryNorm, CenteredNorm, FuncNorm, LinearNorm, LogNorm, Normalize, PowerNorm,
    SymLogNorm, TwoSlopeNorm,
};
pub use crate::picking::{PickResult, pick_nearest};
pub use crate::series::{GridData, Series, Series3D, VectorFieldData, split_at_nan};
pub use crate::spines::Spines;
pub use crate::style::{DashPattern, FillStyle, HatchPattern, LineStyle, MarkerShape, PlotStyle};
pub use crate::theme::{Theme, ThemeGuard};
pub use crate::ticker::{
    AutoMinorLocator, CategoricalFormatter, CategoricalLocator, FixedLocator, FuncFormatter,
    LogFormatter, LogLocator, MaxNLocator, MultipleLocator, NullFormatter, NullLocator,
    PercentFormatter, ScalarFormatter, SiFormatter, TickFormatter, TickLocator,
};
pub use crate::plot_buffer::{
    PlotBuffer, Z_ANNOTATION, Z_BACKGROUND, Z_CHROME, Z_DATA, Z_FILL, Z_GRID, Z_MARKER,
};
pub use crate::transform::{Camera3D, Camera3DState, aspect_area, square_area};

// 2D Plot Widgets
pub use crate::widgets::band::{Band, BandPlot};
pub use crate::widgets::bar_chart::BarChart;
pub use crate::widgets::box_plot::BoxPlot;
pub use crate::widgets::boxen::BoxenPlot;
pub use crate::widgets::candlestick::{Candle, CandlestickChart};
pub use crate::widgets::contour::ContourPlot;
pub use crate::widgets::crosshair::Crosshair;
pub use crate::widgets::dendrogram::{DendroLink, DendroOrientation, Dendrogram};
pub use crate::widgets::ecdf::{EcdfDataset, EcdfPlot};
pub use crate::widgets::error_bar::ErrorBarPlot;
pub use crate::widgets::event_plot::{EventGroup, EventPlot};
pub use crate::widgets::facet_grid::{FacetData, FacetGrid, FacetRecord};
pub use crate::widgets::funnel::{FunnelChart, FunnelEntry};
pub use crate::widgets::gantt::{GanttChart, GanttTask};
pub use crate::widgets::gauge::{GaugeChart, GaugeSector};
pub use crate::widgets::heatmap::Heatmap;
pub use crate::widgets::hexbin::HexbinPlot;
pub use crate::widgets::image_plot::{ImageData, ImageOrigin, ImagePlot, Interpolation, matshow, spy};
pub use crate::widgets::hist2d::Hist2D;
pub use crate::widgets::histogram::Histogram;
pub use crate::widgets::joint_plot::{JointPlot, MarginalType};
pub use crate::widgets::line_plot::LinePlot;
pub use crate::widgets::network::{GraphEdge, GraphLayout, GraphNode, NetworkPlot};
pub use crate::widgets::parallel_coords::{ParallelAxis, ParallelCoords, ParallelRecord};
pub use crate::widgets::pcolormesh::Pcolormesh;
pub use crate::widgets::pie_chart::{PieChart, PieSlice};
pub use crate::widgets::rug::{RugDataset, RugPlot, RugSide};
pub use crate::widgets::sankey::{SankeyDiagram, SankeyFlow, SankeyNode};
pub use crate::widgets::rect_selector::RectangleSelector;
pub use crate::widgets::scatter_plot::ScatterPlot;
#[cfg(feature = "statistics")]
pub use crate::widgets::scatter_plot::TrendlineType;
pub use crate::widgets::span_selector::{SpanDirection, SpanSelector, SpanSelectorState, SharedSpanState, shared_span_state};
pub use crate::widgets::stacked_area::StackedArea;
pub use crate::widgets::stairs::{StairsDataset, StairsPlot};
pub use crate::widgets::stem_plot::StemPlot;
pub use crate::widgets::streamplot::StreamPlot;
pub use crate::widgets::strip::{StripGroup, StripPlot};
pub use crate::widgets::sunburst::{Sunburst, SunburstNode};
pub use crate::widgets::swarm::SwarmPlot;
pub use crate::widgets::ternary::{TernaryData, TernaryPlot};
pub use crate::widgets::treemap::{Treemap, TreemapNode};
pub use crate::widgets::twin_axes::TwinAxes;
pub use crate::widgets::vector_field::{ArrowCharSet, VectorField};
pub use crate::widgets::violin_plot::{ViolinInner, ViolinPlot};
pub use crate::widgets::waterfall::{WaterfallChart, WaterfallEntry};

// 3D Plot Widgets
pub use crate::widgets::bar3d::Bar3D;
pub use crate::widgets::contour3d::Contour3D;
pub use crate::widgets::quiver3d::{Arrow3D, Quiver3D};
pub use crate::widgets::scatter3d::Scatter3D;
pub use crate::widgets::surface3d::Surface3D;
pub use crate::widgets::wireframe3d::Wireframe3D;

// Triangulation Widgets
pub use crate::triangulation::Triangulation;
pub use crate::widgets::tricolor::TriColor;
pub use crate::widgets::tricontour::TriContour;
pub use crate::widgets::triplot::TriPlot;

// FFT Widgets (behind fft feature)
#[cfg(feature = "fft")]
pub use crate::fft::{hamming_window, hann_window, psd, stft};
#[cfg(feature = "fft")]
pub use crate::widgets::psd::PsdPlot;
#[cfg(feature = "fft")]
pub use crate::widgets::spectrogram::Spectrogram;

// Statistics (behind statistics feature)
#[cfg(feature = "statistics")]
pub use crate::statistics::{
    BandwidthMethod, BootstrapCI, EstimatorFn, HistNormExt, Kde, Kernel, LinearFitResult,
    LowessResult, PolyFitResult, bootstrap_ci, iqr, linear_regression, lowess, mean,
    mean_estimator, median, median_estimator, percentile, poly_fit, std_dev, variance,
};

// TOML themes
#[cfg(feature = "toml-themes")]
pub use crate::theme::{ThemeError, load_theme, theme_from_toml};

// Layout
pub use crate::widgets::inset::InsetAxes;
pub use crate::widgets::multi_panel::{MosaicPanel, MultiPanel};
pub use crate::widgets::radial::{PolarPlotType, RadialPlot, ThetaDirection};
