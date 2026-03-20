#!/usr/bin/env bash
set -e

theme="${1:-light}"

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
    heatmap \
    hexbin \
    hist2d \
    histogram \
    inset \
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
    wireframe3d
do
    echo "=== $ex ==="
    cargo run --release --example "$ex" -- "$theme"
done
