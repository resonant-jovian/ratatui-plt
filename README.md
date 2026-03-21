# ratatui-plt

**Scientific visualization widgets for [ratatui](https://ratatui.rs/) — matplotlib for the terminal.**

[![Crates.io](https://img.shields.io/crates/v/ratatui-plt.svg)](https://crates.io/crates/ratatui-plt)
[![docs.rs](https://docs.rs/ratatui-plt/badge.svg)](https://docs.rs/ratatui-plt)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

---

`ratatui-plt` is a comprehensive plotting library for terminal UIs built on [ratatui](https://ratatui.rs/). It provides 50+ plot widgets, colormaps, axis systems, and layout tools for scientific computing, simulation monitoring, and data exploration — all rendered in the terminal using Unicode characters for sub-cell resolution.

> **Status (0.0.2):** Early release. Most widgets work well, but the following have known rendering quality issues: **BandPlot** (fill gap artifacts), **BoxPlot / BoxenPlot** (outline alignment), **CandlestickPlot** (outline mismatches), **Contour3D** (surface artifacts), **VectorField 3D** (low contrast/density), **TernaryPlot** (staircase grid lines). Expect breaking API changes before 0.1.0.

## Features

### 2D Plots
- **LinePlot** — multiple series, fill regions, step modes, dash patterns, markers
- **ScatterPlot** — color-mapped point clouds, configurable markers, trendline overlays (linear/polynomial/LOWESS)
- **Heatmap** — half-block rendering for 2x vertical resolution, colorbars
- **ImagePlot** — matrix/image display with `imshow`, `spy()`, `matshow()` convenience functions
- **Histogram** — count/density/probability modes, stacked, cumulative
- **BarChart** — grouped and stacked, horizontal/vertical
- **ContourPlot** — filled contours and iso-lines via marching squares
- **BoxPlot** — quartiles, whiskers, outliers, notched and bootstrap CI variants
- **ViolinPlot** — KDE-based distribution shape with quartile markers
- **StairsPlot** — step functions with fill-to-baseline
- **StemPlot** — discrete event / impulse visualization
- **ErrorBarPlot** — symmetric/asymmetric error bars
- **StackedArea** — cumulative filled area charts
- **EventPlot** — spike raster / event timing plots
- **Hist2D** — 2D histogram rendered as heatmap
- **HexbinPlot** — hexagonal binning for large datasets
- **PieChart** — pie/donut charts with explode and labels
- **BandPlot** — uncertainty bands / confidence intervals
- **SwarmPlot** — beeswarm plots with jitter
- **StripPlot** — categorical strip/dot plots
- **CandlestickPlot** — OHLC financial charts
- **ECDF** — empirical cumulative distribution functions
- **RugPlot** — marginal tick marks
- **JointPlot** — scatter with marginal histograms/KDE/rug distributions
- **WaterfallChart** — cumulative positive/negative value bars for financial analysis
- **FunnelChart** — centered decreasing-width bars for conversion funnels
- **GaugeChart** — semicircular gauge with needle indicator for KPI dashboards
- **GanttChart** — horizontal bar segments for scheduling/timeline visualization

### 3D Plots
- **Surface3D** — colored surface with half-block shading and wireframe
- **Wireframe3D** — depth-cued wireframe mesh with Braille lines
- **Scatter3D** — 3D point cloud with axis lines
- **Bar3D** — 3D bar chart with depth sorting
- **Contour3D** — filled contour surfaces in 3D
- **Quiver3D** — 3D vector field arrows

All 3D widgets support interactive camera control via `Camera3DState` (arrow keys to rotate, +/- to zoom).

### Specialized Plots
- **RadialPlot** — polar coordinates: line, scatter, bar, fill-between
- **TernaryPlot** — ternary/triangle diagrams with percentage labels
- **NetworkGraph** — force-directed or manual-layout graph visualization
- **ParallelCoords** — parallel coordinates for multivariate data
- **SankeyDiagram** — flow diagrams with node-to-node bands
- **SunburstChart** — hierarchical nested ring charts
- **TreemapChart** — area-proportional hierarchical rectangles
- **DendrogramPlot** — hierarchical clustering trees
- **StreamPlot** — vector field streamlines via Runge-Kutta integration

### Layout
- **MultiPanel** — GridSpec-like subplot grid with `width_ratios` / `height_ratios`, mosaic syntax, shared axes
- **FacetGrid** — seaborn-style automatic small multiples from grouped data
- **TwinAxes** — dual y-axis overlay with independent scales
- **InsetPlot** — zoomed inset panels with highlighted source regions

### Axis System
- **Scales**: Linear, Log, SymLog, Power, Logit, Asinh, Function (custom)
- **Aspect Ratio**: `Auto`, `Equal`, `Fixed(ratio)` with terminal cell geometry compensation
- **Tick Locators**: `MaxNLocator`, `LogLocator`, `MultipleLocator`, `FixedLocator`, `CategoricalLocator`, `AutoMinorLocator`, `NullLocator`
- **Tick Formatters**: `ScalarFormatter`, `LogFormatter`, `SiFormatter`, `PercentFormatter`, `FuncFormatter`, `CategoricalFormatter`, `NullFormatter`
- **Overlap detection**: x-axis labels are automatically skipped when they would collide

### Interactivity
- **Crosshair** — cursor overlay with coordinate readout
- **Data Picking** — nearest-point detection for hover tooltips
- **Brushing** — rectangular selection with `SharedBrush` for linked plots
- **InteractiveLegend** — click-to-toggle series visibility with `SharedLegendState`
- **SpanSelector** — horizontal/vertical range selection overlay
- **RectangleSelector** — 2D rectangular selection overlay
- **LinkedView** — synchronized pan/zoom bounds across multiple panels with `SharedView`

### Rendering
- **Braille sub-pixel lines** — 2x4 dots per cell for smooth curves and diagonals
- **Half-block characters** — `▀`/`▄` for 2x vertical resolution in heatmaps and surfaces
- **Unicode box-drawing** — clean axis borders and chart outlines
- **Depth sorting** — painter's algorithm for correct 3D occlusion

### Colormaps
- **Sequential**: Viridis, Plasma, Inferno, Magma, Cividis
- **Diverging**: Coolwarm, RdBu, Seismic, RdYlBu, RdYlGn, BrBG, PiYG, PRGn, PuOr, RdGy, Spectral
- **Cyclic**: Hsv, Twilight
- **Seasonal**: Spring, Summer, Autumn, Winter
- **Monotone**: Blues, Greens, Greys, Oranges, Reds, Purples + multi-hue sequentials
- **Qualitative**: Paired, Set1, Set2, Set3, Accent, Dark2, Pastel1, Pastel2, Tab20, Tab20b, Tab20c
- **Miscellaneous**: Grayscale, Jet, Turbo, Hot
- **Custom**: `ListedColormap` and `LinearSegmentedColormap` from user-defined color stops
- Colorbar widget with extend modes for out-of-range values

### Normalization
- `LinearNorm`, `LogNorm`, `SymLogNorm`, `PowerNorm`, `BoundaryNorm`, `TwoSlopeNorm`, `CenteredNorm`, `AsinhNorm`, `FuncNorm`
- Trait-based: implement `Normalize` for custom mappings

### Themes
- 5 presets: `dark`, `light`, `minimal`, `publication`, `solarized`
- Global default via `Theme::set_default()` with RAII guard via `Theme::activate()`
- TOML file loading (with `toml-themes` feature)
- All interactive examples accept a theme CLI argument

### Annotations & Legend
- `Annotation` with optional arrow styles (`Arrow`, `Simple`)
- `Legend` with configurable position and multi-column layout
- `InteractiveLegend` with toggle visibility
- Reference lines and spans (`axhline`, `axvline`, `axhspan`, `axvspan`)
- Spines control (show/hide individual axis borders)

### MathText
- Greek letters: `\alpha` -> a, `\beta` -> b, `\Sigma` -> S
- Superscripts: `x^2` -> x2, `10^{-3}` -> 10-3
- Subscripts: `x_0` -> x0
- Scientific notation formatting

### Export
- **Text** — plain Unicode (no color)
- **ANSI** — 24-bit true color terminal escape sequences
- **SVG** — monospace font rendering with cell-based layout
- **PNG** — raster export via the `image` crate (requires `export` feature)
- **Sixel** — inline terminal graphics for Sixel-compatible terminals (requires `sixel` feature)
- **Kitty** — inline terminal graphics for Kitty-compatible terminals (requires `kitty` feature)

> **Terminal compatibility for image export:**
>
> | Protocol | Supported terminals |
> |----------|-------------------|
> | **Kitty** | Kitty, WezTerm, Ghostty |
> | **Sixel** | foot, WezTerm, mlterm, xterm (`-ti vt340`), contour |
> | **Both** | WezTerm |
>
> GNOME Terminal, Alacritty, and most VTE-based terminals do **not** support Sixel or Kitty graphics. PNG export works everywhere (saves to file). Text/ANSI/SVG export requires no feature flags and works in any terminal.

## Optional Features

| Feature | Dependencies | Description |
|---------|-------------|-------------|
| `export` | `resvg`, `usvg`, `tiny-skia`, `svg2pdf`, `image` | PNG raster export |
| `kitty` | (implies `export`) | Kitty graphics protocol inline output |
| `sixel` | `image` (implies `export`) | Sixel graphics protocol inline output |
| `statistics` | (none, pure Rust) | KDE, linear/polynomial regression, LOWESS, bootstrap CI, trendlines |
| `toml-themes` | `toml` (implies `serde`) | Load themes from TOML files |
| `async` | `tokio` | Animation loop, streaming data, background computation |
| `chrono` | `chrono` | Timestamp axis support |
| `serde` | `serde` | Serialization for data types |
| `fft` | `rustfft` | Power spectral density and spectrogram plots |
| `triangulation` | `delaunator` | Delaunay triangulation for unstructured data |

## Quick Start

```toml
[dependencies]
ratatui-plt = "0.0.2"
ratatui = "0.30"

# Optional features:
# ratatui-plt = { version = "0.0.2", features = ["statistics", "export"] }
```

```rust
use ratatui_plt::prelude::*;

let series = Series::new("sin(x)")
    .data((0..100).map(|i| {
        let x = i as f64 * 0.1;
        (x, x.sin())
    }).collect())
    .color(Color::Cyan);

let plot = LinePlot::new()
    .series(series)
    .title("Sine Wave")
    .x_axis(Axis::new().label("x").grid(true))
    .y_axis(Axis::new().label("y"));

// In your ratatui draw callback:
frame.render_widget(&plot, area);
```

## Examples

70+ examples are included. Run any interactive example with:
```bash
cargo run --example <name>
# Pass a theme:
cargo run --example line_plot -- light
```

Feature-gated examples require the feature flag:
```bash
cargo run --example statistics --features statistics
cargo run --example trendline --features statistics
cargo run --example kitty_export --features kitty
cargo run --example sixel_export --features sixel
cargo run --example toml_theme --features toml-themes
```

Run all examples in sequence:
```bash
./run_examples.sh        # default light theme
./run_examples.sh dark   # dark theme
```

### Showcase Examples (matplotlib reference replicas)

Six showcase examples replicate matplotlib's reference plot gallery using `MultiPanel` grids:

| Example | Plots |
|---------|-------|
| `showcase_basic_2d` | LinePlot, ScatterPlot, BarChart, Histogram, PieChart, StairsPlot, StemPlot |
| `showcase_statistical` | BoxPlot, ViolinPlot, ErrorBarPlot, EcdfPlot, EventPlot |
| `showcase_grid` | ContourPlot (unfilled + filled), Heatmap, Pcolormesh, HexbinPlot, Hist2D, VectorField, StreamPlot |
| `showcase_fill` | BandPlot (fill_between), StackedArea |
| `showcase_tri` | TriPlot, TriContour (unfilled + filled), TriColor |
| `showcase_3d` | Surface3D, Wireframe3D, Scatter3D, Bar3D, Quiver3D |

### All Examples

| Example | Description |
|---------|-------------|
| `line_plot` | Sine/cosine with fill, legend, grid |
| `scatter_plot` | Color-mapped point cloud |
| `heatmap` | Correlation matrix with Viridis colorbar |
| `image_plot` | Matrix display with `matshow`, `spy`, bilinear interpolation |
| `histogram` | Stacked distributions |
| `contour` | Filled 2D potential field |
| `surface3d` | Interactive 3D surface with camera |
| `wireframe3d` | Depth-cued 3D wireframe |
| `scatter3d` | 3D point cloud |
| `bar3d` | 3D bar chart with axis lines |
| `box_plot` | Standard, notched, and bootstrap CI |
| `violin_plot` | KDE distribution shapes |
| `candlestick` | OHLC financial price action |
| `ecdf` | Empirical CDFs for 3 distributions |
| `stairs` | Step function plot |
| `band` | Uncertainty bands (confidence intervals) |
| `rug` | Histogram + KDE + rug marks |
| `swarm` | Beeswarm by browser |
| `strip` | Gene expression by cell type |
| `stem_plot` | Discrete impulse events |
| `error_bar` | Symmetric/asymmetric error bars |
| `stacked_area` | Cumulative filled areas |
| `bar_chart` | Grouped/stacked bars |
| `pie_chart` | Pie/donut chart |
| `hexbin` | Hexagonal binning |
| `hist2d` | 2D histogram |
| `event_plot` | Spike raster |
| `joint_plot` | Scatter with marginal histograms and KDE |
| `waterfall` | Financial waterfall (P&L) chart |
| `funnel` | Sales conversion funnel |
| `gauge` | KPI gauge with colored sectors |
| `gantt` | Project timeline Gantt chart |
| `facet_grid` | Seaborn-style faceted scatter |
| `radial` | Polar line, scatter, bar, fill |
| `ternary` | Soil texture triangle |
| `network` | Social network graph |
| `parallel_coords` | Iris dataset parallel coordinates |
| `sankey` | Energy flow Sankey diagram |
| `sunburst` | World population sunburst |
| `treemap` | Hierarchical treemap |
| `dendrogram` | Clustering tree |
| `streamplot` | Circular flow field |
| `vector_field` | 3D vector field dipole |
| `collections` | LineCollection / PathCollection |
| `multi_panel` | 4-panel subplot grid |
| `twin_axes` | Dual y-axis overlay |
| `inset` | Damped sine with zoomed inset |
| `crosshair` | Interactive crosshair |
| `picking` | Nearest-point data picking |
| `interactive_legend` | Click-to-toggle series visibility |
| `span_selector` | Horizontal range selection |
| `scientific_dashboard` | Full 4-panel simulation monitor |
| `theme_config` | Built-in theme gallery |
| `pcolormesh` | Pseudocolor mesh plot |
| `triplot` | Triangulation mesh |
| `tricolor` | Triangulated color map |
| `contour3d` | 3D contour surface |
| `quiver3d` | 3D vector arrows |
| `boxen` | Letter-value (boxen) plot |
| `statistics` | Regression + KDE + LOWESS overlays (requires `statistics`) |
| `trendline` | Scatter with polynomial trendline (requires `statistics`) |
| `kitty_export` | Inline Kitty image output (requires `kitty`) |
| `sixel_export` | Inline Sixel image output (requires `sixel`) |
| `toml_theme` | Load theme from TOML string (requires `toml-themes`) |

## Convenience Macros

```rust
let s = series!("sin(x)", [(0.0, 0.0), (1.0, 0.84), (2.0, 0.91)]);
let p = plot!(title = "My Plot", series1, series2);
let h = heatmap_widget!(grid_data, Plasma);
let panel = subplot!(2, 2, gap = 1);
let cmap = colormap_custom!("div", 0.0 => Color::Blue, 0.5 => Color::White, 1.0 => Color::Red);
```

## License

GPL-3.0
