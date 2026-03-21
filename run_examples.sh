#!/usr/bin/env bash
set -e

if [ -n "$1" ]; then
    theme="$1"
else
    echo "Available themes: dark, light, minimal, publication, solarized"
    printf "Select theme [publication]: "
    read -r theme
    theme="${theme:-publication}"
fi

# Examples that need no feature flags
for ex in \
    band \
    bar3d \
    bar_chart \
    box_plot \
    boxen \
    candlestick \
    collections \
    contour \
    contour3d \
    crosshair \
    dendrogram \
    ecdf \
    error_bar \
    event_plot \
    facet_grid \
    funnel \
    gantt \
    gauge \
    heatmap \
    hexbin \
    hist2d \
    histogram \
    image_plot \
    inset \
    interactive_legend \
    joint_plot \
    line_plot \
    multi_panel \
    network \
    parallel_coords \
    pcolormesh \
    picking \
    pie_chart \
    quiver3d \
    radial \
    rug \
    sankey \
    scatter3d \
    scatter_plot \
    scientific_dashboard \
    showcase_3d \
    showcase_basic_2d \
    showcase_fill \
    showcase_grid \
    showcase_statistical \
    showcase_tri \
    span_selector \
    stacked_area \
    stairs \
    stem_plot \
    streamplot \
    strip \
    sunburst \
    surface3d \
    swarm \
    ternary \
    theme_config \
    treemap \
    tricolor \
    triplot \
    twin_axes \
    vector_field \
    violin_plot \
    waterfall \
    wireframe3d
do
    echo "=== $ex ==="
    cargo run --release --example "$ex" -- "$theme"
done

# Feature-gated examples
echo "=== statistics (--features statistics) ==="
cargo run --release --features statistics --example statistics -- "$theme"

echo "=== trendline (--features statistics) ==="
cargo run --release --features statistics --example trendline -- "$theme"

echo "=== kitty_export (--features kitty) ==="
cargo run --release --features kitty --example kitty_export

echo "=== sixel_export (--features sixel) ==="
cargo run --release --features sixel --example sixel_export

echo "=== toml_theme (--features toml-themes) ==="
cargo run --release --features toml-themes --example toml_theme
