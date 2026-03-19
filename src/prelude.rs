//! Convenient re-exports for common usage.
//!
//! ```rust
//! use ratatui_sim::prelude::*;
//! ```

pub use ratatui::style::Color;

pub use crate::annotation::Annotation;
pub use crate::axis::{AspectRatio, Axis, Scale};
pub use crate::colormap::{
    Cividis, Colorbar, Colormap, Coolwarm, Grayscale, Hot, Inferno, Jet, ListedColormap, Magma,
    Plasma, RdBu, Seismic, Turbo, Viridis,
};
pub use crate::legend::{Legend, LegendPosition};
pub use crate::norm::{BoundaryNorm, LinearNorm, LogNorm, Normalize, PowerNorm, SymLogNorm, TwoSlopeNorm};
pub use crate::series::{GridData, Series, Series3D, VectorFieldData};
pub use crate::style::{FillStyle, LineStyle, MarkerShape, PlotStyle};
pub use crate::ticker::{
    FixedLocator, FuncFormatter, LogFormatter, LogLocator, MaxNLocator, MultipleLocator,
    ScalarFormatter, SiFormatter, TickFormatter, TickLocator,
};
pub use crate::transform::{Camera3D, Camera3DState};
pub use crate::widgets::bar_chart::BarChart;
pub use crate::widgets::box_plot::BoxPlot;
pub use crate::widgets::contour::ContourPlot;
pub use crate::widgets::error_bar::ErrorBarPlot;
pub use crate::widgets::heatmap::Heatmap;
pub use crate::widgets::hexbin::HexbinPlot;
pub use crate::widgets::histogram::Histogram;
pub use crate::widgets::line_plot::LinePlot;
pub use crate::widgets::multi_panel::MultiPanel;
pub use crate::widgets::radial::RadialPlot;
pub use crate::widgets::scatter3d::Scatter3D;
pub use crate::widgets::scatter_plot::ScatterPlot;
pub use crate::widgets::stem_plot::StemPlot;
pub use crate::widgets::surface3d::Surface3D;
pub use crate::widgets::vector_field::VectorField;
pub use crate::widgets::wireframe3d::Wireframe3D;
