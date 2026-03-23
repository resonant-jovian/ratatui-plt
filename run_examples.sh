#!/usr/bin/env bash
set -e

# Theme selection
if [ -n "$1" ]; then
    theme="$1"
else
    echo "Available themes:"
    echo "  [0] auto (detect light/dark terminal)"
    echo "  [1] dark"
    echo "  [2] light"
    echo "  [3] minimal"
    echo "  [4] publication"
    echo "  [5] solarized"
    printf "Select theme [0]: "
    read -r choice
    case "${choice:-0}" in
        0|auto)        theme="" ;;
        1|dark)        theme="dark" ;;
        2|light)       theme="light" ;;
        3|minimal)     theme="minimal" ;;
        4|publication) theme="publication" ;;
        5|solarized)   theme="solarized" ;;
        *)             theme="$choice" ;;
    esac
fi

if [ -z "$theme" ]; then
    echo "Using theme: auto (detecting terminal)"
else
    echo "Using theme: $theme"
fi
echo ""

# Build theme argument (empty for auto-detect)
theme_arg=()
if [ -n "$theme" ]; then
    theme_arg=("--" "$theme")
fi

# Prompt to continue or quit between examples
wait_for_input() {
    printf "\nPress Enter to continue, or n/Ctrl-C to quit... "
    read -r ans
    case "$ans" in
        n|N|no|NO) echo "Stopped."; exit 0 ;;
    esac
}

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
    cargo run --release --example "$ex" "${theme_arg[@]}"
    wait_for_input
done

# Feature-gated examples
echo "=== statistics (--features statistics) ==="
cargo run --release --features statistics --example statistics "${theme_arg[@]}"
wait_for_input

echo "=== trendline (--features statistics) ==="
cargo run --release --features statistics --example trendline "${theme_arg[@]}"
wait_for_input

echo "=== kitty_export (--features kitty) ==="
cargo run --release --features kitty --example kitty_export
wait_for_input

echo "=== sixel_export (--features sixel) ==="
cargo run --release --features sixel --example sixel_export
wait_for_input

echo "=== toml_theme (--features toml-themes) ==="
cargo run --release --features toml-themes --example toml_theme
wait_for_input

echo "=== showcase_unicode (--features unicode-extended) ==="
cargo run --release --features unicode-extended --example showcase_unicode "${theme_arg[@]}"
wait_for_input

echo "=== showcase_features (--features statistics) ==="
cargo run --release --features statistics --example showcase_features "${theme_arg[@]}"

echo ""
echo "All examples complete."
