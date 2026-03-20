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
    line_plot \
    multi_panel \
    parallel_coords \
    pcolormesh \
    pie_chart \
    quiver3d \
    radial \
    rug \
    scatter3d \
    scatter_plot \
    scientific_dashboard \
    stacked_area \
    stairs \
    stem_plot \
    streamplot \
    strip \
    surface3d \
    swarm \
    twin_axes \
    vector_field \
    violin_plot \
    wireframe3d
do
    echo "=== $ex ==="
    cargo run --release --example "$ex" -- "$theme"
done
