# ratatui-plt

**Scientific visualization widgets for [ratatui](https://ratatui.rs/) — matplotlib for the terminal.**

[![Crates.io](https://img.shields.io/crates/v/ratatui-plt.svg)](https://crates.io/crates/ratatui-plt)
[![docs.rs](https://docs.rs/ratatui-plt/badge.svg)](https://docs.rs/ratatui-plt)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

---

`ratatui-plt` is a comprehensive plotting library for terminal UIs built on [ratatui](https://ratatui.rs/). It provides 30+ plot widgets, colormaps, axis systems, and layout tools for scientific computing, simulation monitoring, and data exploration — all rendered in the terminal using Unicode characters for sub-cell resolution.

> **Status (0.0.1):** This is an early release. Most widgets work well, but the following have known rendering quality issues and are still being improved: **BandPlot** (fill gap artifacts), **BoxPlot / BoxenPlot** (outline alignment), **CandlestickPlot** (outline mismatches), **Contour3D** (surface artifacts), **VectorField 3D** (low contrast/density), **TernaryPlot** (staircase grid lines). Expect breaking API changes before 0.1.0.

## Features

### 2D Plots
- **LinePlot** — multiple series, fill regions, step modes, dash patterns, markers
- **ScatterPlot** — color-mapped point clouds with configurable markers
- **Heatmap** — half-block rendering for 2x vertical resolution, colorbars
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
- **MultiPanel** — GridSpec-like subplot grid with `width_ratios` / `height_ratios`
- **TwinAxes** — dual y-axis overlay with independent scales
- **InsetPlot** — zoomed inset panels with highlighted source regions

### Axis System
- **Scales**: Linear, Log, SymLog (symmetric log), Power
- **Aspect Ratio**: `Auto`, `Equal`, `Fixed(ratio)` with terminal cell geometry compensation
- **Tick Locators**: `MaxNLocator`, `LogLocator`, `MultipleLocator`, `FixedLocator`, `CategoricalLocator`
- **Tick Formatters**: `ScalarFormatter`, `LogFormatter`, `SiFormatter`, `FuncFormatter`, `CategoricalFormatter`
- **Overlap detection**: x-axis labels are automatically skipped when they would collide

### Rendering
- **Braille sub-pixel lines** — 2x4 dots per cell for smooth curves and diagonals
- **Half-block characters** — `▀`/`▄` for 2x vertical resolution in heatmaps and surfaces
- **Unicode box-drawing** — clean axis borders and chart outlines
- **Depth sorting** — painter's algorithm for correct 3D occlusion

### Colormaps
- **Sequential**: Viridis, Plasma, Inferno, Magma, Cividis
- **Diverging**: Coolwarm, RdBu, Seismic
- **Cyclic**: Hsv, Twilight
- **Seasonal**: Spring, Summer, Autumn, Winter
- **Miscellaneous**: Grayscale, Jet, Turbo, Hot
- **Custom**: `ListedColormap` from user-defined color stops
- Colorbar widget for value-to-color mapping display

### Normalization
- `LinearNorm`, `LogNorm`, `SymLogNorm`, `PowerNorm`, `BoundaryNorm`, `TwoSlopeNorm`
- Trait-based: implement `Normalize` for custom mappings

### Themes
- 5 presets: `dark`, `light`, `minimal`, `publication`, `solarized`
- Global default via `Theme::set_default()`
- All examples accept a theme CLI argument

### Annotations & Legend
- `Annotation` with optional arrow styles (`Arrow`, `FancyArrow`, `Bracket`)
- `Legend` with configurable position
- Reference lines and spans (`axhline`, `axvline`, `axhspan`, `axvspan`)
- Spines control (show/hide individual axis borders)

### MathText
- Greek letters: `\alpha` → α, `\beta` → β, `\Sigma` → Σ
- Superscripts: `x^2` → x², `10^{-3}` → 10⁻³
- Subscripts: `x_0` → x₀
- Scientific notation formatting

### Optional Features
- **`async`** — tokio-based animation loop, streaming data, background KDE/histogram
- **`chrono`** — timestamp axis support
- **`serde`** — serialization for data types
- **`fft`** — power spectral density plots
- **`triangulation`** — Delaunay triangulation for unstructured data

## Quick Start

```toml
[dependencies]
ratatui-plt = "0.0.1"
ratatui = "0.30"
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

50 examples are included. Run any with:
```bash
cargo run --example <name>
# Pass a theme:
cargo run --example line_plot -- light
```

| Example | Description |
|---------|-------------|
| `line_plot` | Sine/cosine with fill, legend, grid |
| `scatter_plot` | Color-mapped point cloud |
| `heatmap` | Correlation matrix with Viridis colorbar |
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
| `scientific_dashboard` | Full 4-panel simulation monitor |
| `theme_config` | Built-in theme gallery |
| `pcolormesh` | Pseudocolor mesh plot |
| `triplot` | Triangulation mesh, faces, contours |
| `contour3d` | 3D contour surface |
| `quiver3d` | 3D vector arrows |
| `boxen` | Letter-value (boxen) plot |

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
