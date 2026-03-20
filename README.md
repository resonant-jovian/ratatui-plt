# ratatui-plt

**Scientific visualization widgets for [ratatui](https://ratatui.rs/) — matplotlib for the terminal.**

`ratatui-plt` provides a comprehensive suite of configurable plot widgets, colormaps, axis systems, and layout tools designed for scientific computing and simulation monitoring. Built primarily for astrophysical applications (Vlasov-Poisson solvers, phase-space analysis), it works anywhere you need terminal-based scientific plots.

> **Note:** This library has not yet reached version 0.1.0. The API is unstable, features may be incomplete or change without notice, and it is not yet intended for general use.

## Features

### 2D Plot Widgets
- **LinePlot** — Multiple series, error bars, fill regions, step modes, dashed/dotted lines
- **ScatterPlot** — Point clouds with color-mapped third values, configurable markers
- **Heatmap** — Half-block rendering for 2x vertical resolution, colorbars, normalization
- **Histogram** — Binning with count/density/probability modes, cumulative support
- **BarChart** — Grouped and stacked modes, horizontal/vertical
- **ContourPlot** — Marching squares iso-lines, filled contours (contourf-style)
- **StemPlot** — Discrete event visualization with baseline
- **ErrorBarPlot** — Standalone symmetric/asymmetric error bars
- **BoxPlot** — Quartiles, whiskers (1.5xIQR), outlier detection
- **VectorField** — 2D arrow fields with magnitude coloring (quiver plots)
- **HexbinPlot** — Hexagonal binning for large datasets (10^4+ points)
- **PieChart** — Pie/donut charts with explode, percentages, labels
- **StackedArea** — Stacked filled areas with fill characters
- **EventPlot** — Event timing / spike raster plots
- **Hist2D** — 2D histogram rendered as heatmap
- **ViolinPlot** — Violin plots with KDE and quartiles

### 3D Plot Widgets
- **Surface3D** — Colored surface with painter's algorithm, interactive camera
- **Wireframe3D** — Depth-cued wireframe mesh
- **Scatter3D** — 3D point cloud with depth cuing

All 3D widgets support both static (`Widget`) and interactive (`StatefulWidget`) rendering with `Camera3DState` for keyboard-driven rotation and zoom.

### Layout / Multi-Panel
- **RadialPlot** — Polar coordinate rendering with circular grids and angular ticks
- **MultiPanel** — GridSpec-like subplot grid with `width_ratios` / `height_ratios`
- **TwinAxes** — Dual y-axis overlay with independent scales
- **StreamPlot** — Vector field streamlines via Runge-Kutta integration

### Axis System (matplotlib-inspired)
- **Scales**: Linear, Log, SymLog (symmetric log), Power
- **Aspect Ratio**: `Auto`, `Equal`, `Fixed(ratio)` — auto-compensates for terminal cell geometry
- **Tick Locators**: `MaxNLocator` (nice round numbers), `LogLocator`, `MultipleLocator`, `FixedLocator`, `CategoricalLocator`
- **Tick Formatters**: `ScalarFormatter`, `LogFormatter`, `SiFormatter` (engineering prefixes), `FuncFormatter`, `CategoricalFormatter`

### Normalization System
- `LinearNorm`, `LogNorm`, `SymLogNorm`, `PowerNorm`, `BoundaryNorm`, `TwoSlopeNorm`
- Essential for astrophysical data spanning many orders of magnitude

### Theme System
- 5 presets: `dark`, `light`, `minimal`, `publication`, `solarized`
- Global default via `Theme::set_default()`
- All examples accept `--theme` CLI argument

### Annotations & Legend
- `Annotation` with optional arrow styles (`Arrow`, `FancyArrow`, `Bracket`)
- `Legend` with position control (`TopLeft`, `TopRight`, `BottomLeft`, `BottomRight`, etc.)

### Async Features (feature-gated: `async`)
- `AnimationConfig` / `run_animation()` — animation loop framework
- `AsyncSeries` / `AsyncGrid` — tokio-based streaming data
- `compute_kde_async()` / `compute_histogram_async()` — background computation

### Scientific Colormaps
- **Sequential** (perceptually uniform): Viridis, Plasma, Inferno, Magma, Cividis
- **Diverging**: Coolwarm, RdBu, Seismic
- **Cyclic**: Hsv, Twilight
- **Seasonal**: Spring, Summer, Autumn, Winter
- **Miscellaneous**: Grayscale, Jet, Turbo, Hot
- **Custom**: `ListedColormap` from user-defined color stops
- Colorbar widget for value-to-color mapping display

### MathText
- Greek letters: `\alpha` → α, `\beta` → β, `\Sigma` → Σ
- Superscripts: `x^2` → x², `10^{-3}` → 10⁻³
- Subscripts: `x_0` → x₀
- Scientific notation formatting

### Convenience Macros
```rust
// Quick series creation
let s = series!("sin(x)", [(0.0, 0.0), (1.0, 0.84), (2.0, 0.91)]);
let s = series!("cos(x)", data_vec, color = Color::Red);

// Quick line plot
let p = plot!(series1, series2);
let p = plot!(title = "My Plot", series1, series2);

// Quick heatmap
let h = heatmap_widget!(grid_data, Plasma);

// Quick subplot grid
let panel = subplot!(2, 2, gap = 1);

// Custom colormap
let cmap = colormap_custom!("diverging", 0.0 => Color::Blue, 0.5 => Color::White, 1.0 => Color::Red);
```

## Quick Start

Add to your `Cargo.toml`:
```toml
[dependencies]
ratatui-plt = "0.0.1"
ratatui = "0.30"
```

Basic line plot:
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

Heatmap with forced aspect ratio:
```rust
use ratatui_plt::prelude::*;

let data = GridData::from_fn((-2.0, 2.0), (-2.0, 2.0), 50, 50, |x, y| {
    (-(x * x + y * y)).exp()
});

let heatmap = Heatmap::new(data)
    .colormap(Viridis)
    .title("2D Gaussian")
    .aspect_ratio(AspectRatio::Equal);

frame.render_widget(&heatmap, area);
```

Interactive 3D surface:
```rust
use ratatui_plt::prelude::*;

let data = GridData::from_fn((-3.0, 3.0), (-3.0, 3.0), 30, 30, |x, y| {
    (x * x + y * y).sqrt().sin()
});

let surface = Surface3D::new(data).title("3D Surface");
let mut camera = Camera3DState::default();

// In event loop, handle arrow keys:
camera.rotate(5.0, 0.0);  // Rotate azimuth
camera.rotate(0.0, 5.0);  // Rotate elevation
camera.zoom(0.9);          // Zoom in

frame.render_stateful_widget(&surface, area, &mut camera);
```

## Examples

Run any example with:
```bash
cargo run --example <name>
```

| Example | Description |
|---------|-------------|
| `line_plot` | Sine/cosine with error bands, legend, grid |
| `scatter_plot` | Random point cloud with color-mapped values |
| `heatmap` | 2D Gaussian with viridis colormap and colorbar |
| `histogram` | Normal distribution with density normalization |
| `contour` | 2D potential field with filled contours |
| `surface3d` | Interactive 3D surface with keyboard rotation |
| `stem_plot` | Discrete event sequence |
| `box_plot` | Distribution comparison across groups |
| `vector_field` | 2D velocity field arrows |
| `multi_panel` | 4-panel dashboard layout |
| `radial` | Polar coordinate radial profile |
| `scientific_dashboard` | Full simulation monitoring dashboard |

## Design Principles

- **Builder pattern** for all widgets: `LinePlot::new().series(s).title("Plot")`
- **Reference rendering**: implements `Widget for &WidgetName` for zero-copy reuse
- **Half-block characters** for heatmaps — doubles vertical resolution
- **Braille-ready** marker support for sub-character resolution
- **Terminal cell compensation**: aspect ratio system accounts for ~2:1 cell geometry
- **Theming**: consistent styling via 5 built-in themes or custom `Theme` structs
- **Annotations & legends**: first-class support for labeling and explaining plots
- **Minimal dependencies**: only `ratatui`, `palette`, and `ordered-float`

## License

GPL-3.0
