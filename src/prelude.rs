//! Convenient re-exports for common usage.
//!
//! ```rust
//! use ratatui_plt::prelude::*;
//! ```

pub use ratatui::style::Color;

pub use crate::annotation::Annotation;
pub use crate::axis::{AspectRatio, Axis, Bounds, GridConfig, Scale};
pub use crate::color_cycle::ColorCycle;
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
pub use crate::export::{
    buffer_to_ansi, buffer_to_svg, buffer_to_text, render_to_buffer, save_ansi, save_svg, save_text,
};
pub use crate::frame::{PlotArea, PlotFrame, RefLineDash, ReferenceLine};
pub use crate::legend::{Legend, LegendPosition};
pub use crate::norm::{
    AsinhNorm, BoundaryNorm, CenteredNorm, FuncNorm, LinearNorm, LogNorm, Normalize, PowerNorm,
    SymLogNorm, TwoSlopeNorm,
};
pub use crate::series::{GridData, Series, Series3D, VectorFieldData};
pub use crate::spines::Spines;
pub use crate::style::{FillStyle, LineStyle, MarkerShape, PlotStyle};
pub use crate::theme::Theme;
pub use crate::ticker::{
    AutoMinorLocator, CategoricalFormatter, CategoricalLocator, FixedLocator, FuncFormatter,
    LogFormatter, LogLocator, MaxNLocator, MultipleLocator, NullFormatter, NullLocator,
    PercentFormatter, ScalarFormatter, SiFormatter, TickFormatter, TickLocator,
};
pub use crate::transform::{Camera3D, Camera3DState, square_area};

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
pub use crate::widgets::heatmap::Heatmap;
pub use crate::widgets::hexbin::HexbinPlot;
pub use crate::widgets::hist2d::Hist2D;
pub use crate::widgets::histogram::Histogram;
pub use crate::widgets::line_plot::LinePlot;
pub use crate::widgets::parallel_coords::{ParallelAxis, ParallelCoords, ParallelRecord};
pub use crate::widgets::pcolormesh::Pcolormesh;
pub use crate::widgets::pie_chart::{PieChart, PieSlice};
pub use crate::widgets::rug::{RugDataset, RugPlot, RugSide};
pub use crate::widgets::scatter_plot::ScatterPlot;
pub use crate::widgets::stacked_area::StackedArea;
pub use crate::widgets::stairs::{StairsDataset, StairsPlot};
pub use crate::widgets::stem_plot::StemPlot;
pub use crate::widgets::streamplot::StreamPlot;
pub use crate::widgets::strip::{StripGroup, StripPlot};
pub use crate::widgets::swarm::SwarmPlot;
pub use crate::widgets::twin_axes::TwinAxes;
pub use crate::widgets::vector_field::VectorField;
pub use crate::widgets::violin_plot::{ViolinInner, ViolinPlot};

// 3D Plot Widgets
pub use crate::widgets::bar3d::Bar3D;
pub use crate::widgets::contour3d::Contour3D;
pub use crate::widgets::quiver3d::{Arrow3D, Quiver3D};
pub use crate::widgets::scatter3d::Scatter3D;
pub use crate::widgets::surface3d::Surface3D;
pub use crate::widgets::wireframe3d::Wireframe3D;

// Layout
pub use crate::widgets::multi_panel::{MosaicPanel, MultiPanel};
pub use crate::widgets::radial::RadialPlot;
