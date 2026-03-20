# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

ratatui-plt — scientific visualization widgets for ratatui (matplotlib for the terminal). Single-crate Rust library (edition 2024) providing plot widgets, colormaps, axis systems, and layout tools for terminal-based scientific plots. Licensed GPL-3.0.

## Commands

```bash
cargo build                          # Build library
cargo build --all-features           # Build with async/chrono/serde
cargo test                           # Run all tests
cargo test <test_name>               # Run a single test
cargo run --example <name>           # Run an example (e.g. line_plot, surface3d, scientific_dashboard)
cargo clippy                         # Lint
cargo doc --open                     # Build and view docs
```

Optional features: `async` (tokio streaming), `chrono` (timestamps), `serde` (serialization).

## Architecture

### Data flow

Raw data → `Series`/`GridData`/`VectorFieldData` → Widget (builder pattern) → ratatui `Widget`/`StatefulWidget` trait → terminal rendering.

### Core modules (src/)

- **series.rs** — Data containers: `Series` (2D), `Series3D`, `GridData` (2D matrix), `VectorFieldData`. All plot widgets consume these.
- **axis.rs** — `Axis` config, `Scale` enum (Linear/Log/SymLog/Power), `AspectRatio`, `Bounds`. `TERMINAL_CELL_ASPECT` (~0.5) compensates for ~2:1 terminal cell geometry.
- **norm.rs** — `Normalize` trait maps values to [0,1] for color mapping. Implementations: Linear, Log, SymLog, Power, Boundary, TwoSlope. Uses `box_clone()` for cloneable trait objects.
- **colormap.rs** — `Colormap` trait maps [0,1] → ratatui `Color`. 18 built-in colormaps. `ListedColormap` for custom color stops.
- **ticker.rs** — `TickLocator` trait (MaxNLocator, LogLocator, MultipleLocator, FixedLocator, CategoricalLocator) and `TickFormatter` trait (ScalarFormatter, LogFormatter, SiFormatter, FuncFormatter, CategoricalFormatter).
- **transform.rs** — 3D camera system: `Camera3D` (immutable config) and `Camera3DState` (mutable, for interactive use). `data_to_screen()`, `depth_sort()` (painter's algorithm).
- **mathtext.rs** — Unicode-based math rendering: Greek letters (`\alpha`→α), superscripts (`x^2`→x²), subscripts (`x_0`→x₀), `scientific_notation()`.
- **theme.rs** — `Theme` struct with presets (dark, light, minimal, publication, solarized). Thread-local global default.
- **style.rs** — `LineStyle`, `MarkerShape`, `FillStyle`, `PlotStyle`.
- **annotation.rs** — `Annotation`, `ArrowStyle` for text annotations with arrows.
- **legend.rs** — `Legend`, `LegendEntry`, `LegendPosition`.
- **spines.rs** — `Spines` for axis border visibility control.
- **color_cycle.rs** — `ColorCycle` for automatic color assignment (tab10 palette).
- **animation.rs** — `AnimationConfig`, `run_animation()` (async feature).
- **async_data.rs** — `AsyncSeries`, `AsyncGrid`, async data streaming (async feature).
- **compute.rs** — Background KDE/histogram computation (async feature).

### Widgets (src/widgets/)

23 widget files. Key ones: `line_plot`, `scatter_plot`, `heatmap`, `histogram`, `bar_chart`, `contour`, `surface3d`, `wireframe3d`, `scatter3d`, `multi_panel`, `radial`, `twin_axes`, `pie_chart`, `stacked_area`, `event_plot`, `hist2d`, `violin_plot`, `streamplot`.

### Key conventions

- **Builder pattern everywhere** — all widgets: `LinePlot::new().series(s).title("Plot").x_axis(...)`.
- **Reference rendering** — widgets implement `Widget for &WidgetName` for zero-copy reuse.
- **3D uses StatefulWidget** — 3D widgets use `Camera3DState` for interactive camera control via `StatefulWidget`.
- **Half-block characters** — `Heatmap` uses `▀`/`▄` for 2× vertical resolution.
- **Convenience macros** — `series!`, `plot!`, `heatmap_widget!`, `subplot!`, `colormap_custom!` in `macros.rs`.
- **Prelude** — `use ratatui_plt::prelude::*` imports all commonly needed types.
- **Trait extensibility** — `Normalize`, `Colormap`, `TickLocator`, `TickFormatter` are all public traits users can implement.

### Tests

Integration tests only, in `tests/integration.rs`. Tests cover data types, normalization, ticks, colormaps, mathtext, 3D transforms, and widget rendering (render-without-panic).
