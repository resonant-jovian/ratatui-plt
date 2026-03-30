# Changelog

All notable changes to this project will be documented in this file.

The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-27

First stable API release.

### Added

- 80 plot widgets across 11 categories: core (8), statistical
  (14), scientific (13), financial (6), hierarchical/relational
  (9), polar/specialized (3), 3D (8), triangulation (3), layout
  (6), interaction (4), FFT (2)
- Pluggable `PlotBackend` trait with Unicode (default), Kitty,
  and Sixel rendering backends
- `PlotFrame` axis system with 7 scale types (Linear, Log,
  SymLog, Power, Asinh, Logit, FuncScale), 6 tick locators,
  and 7 tick formatters
- Theme system with 6 built-in presets (dark, light, minimal,
  publication, solarized, gruvbox) and thread-local defaults
- `CharSet`-based rendering: all glyphs centralized in the
  theme for consistent, customizable terminal output
- 55+ colormaps organized into 8 families (perceptual,
  sequential, diverging, cyclic, qualitative, cubehelix,
  terrain, miscellaneous)
- 9 normalization modes (Linear, Log, SymLog, Power, Asinh,
  TwoSlope, Boundary, Centered, Quantile)
- MathText support: Greek letters, superscripts, subscripts,
  scientific notation via Unicode
- Annotation system with configurable arrow styles
- Legend with 9 position options and automatic color cycle
- Export formats: text, ANSI, SVG, PNG, PDF, Sixel, Kitty
- 51 examples across 13 showcase galleries
- `series!`, `plot!`, `heatmap_widget!`, `subplot!`,
  `colormap_custom!` convenience macros
- `prelude` module for ergonomic imports
- Optional feature flags: `async`, `chrono`, `serde`, `fft`,
  `triangulation`, `export`, `kitty`, `sixel`, `toml-themes`,
  `statistics`, `unicode-extended`
- Smart backend auto-detection: probes Kitty/Sixel support via
  environment variables and escape sequences, with OnceLock
  caching and `RATATUI_PLT_BACKEND` env var override
- Headless export via `headless_export()`: set
  `RATATUI_PLT_EXPORT=1` to render showcases to SVG/PNG
  without a terminal
- `dev.sh screenshots` command for batch screenshot generation
  of all 13 showcases with configurable theme/format/size
- Expanded edge-case test coverage: NaN handling, empty data,
  single-point data, extreme values, Unicode labels, backend
  detection subprocess tests, tiny/huge rendering areas

### Changed

- **Breaking:** All widgets now render through `dyn PlotBackend`
  instead of `PlotBuffer` directly
- **Breaking:** Wireframe3D merged into Surface3D as
  `SurfaceRenderMode::Wireframe`
- **Breaking:** `StackedArea` renamed to `AreaChart` with
  `AreaMode` enum (Plain, Stacked, Normalized, StreamGraph)
- Consolidated examples from individual files into 13 thematic
  gallery showcases
- Reformatted codebase to 72-character line width

[0.1.0]: https://github.com/resonant-jovian/ratatui-plt/releases/tag/v0.1.0
