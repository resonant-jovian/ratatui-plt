#!/usr/bin/env bash
set -e

theme="${1:-light}"

for ex in \
    bar_chart \
    box_plot \
    contour \
    error_bar \
    event_plot \
    heatmap \
    hexbin \
    hist2d \
    histogram \
    line_plot \
    multi_panel \
    pie_chart \
    radial \
    scatter3d \
    scatter_plot \
    scientific_dashboard \
    stacked_area \
    stem_plot \
    streamplot \
    surface3d \
    twin_axes \
    vector_field \
    violin_plot \
    wireframe3d
do
    echo "=== $ex ==="
    cargo run --example "$ex" -- "$theme"
done
