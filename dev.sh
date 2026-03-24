#!/usr/bin/env bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════════════
# dev.sh — Development script for ratatui-plt
#
# Usage: ./dev.sh <command> [flags]
#
# Commands: examples, test, bench, build, lint, clean, doctor, info, help
# ══════════════════════════════════════════════════════════════════════════════

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# ── Colours ──────────────────────────────────────────────────────────────────

if [[ -t 1 ]]; then
    RED=$'\e[0;31m'
    GREEN=$'\e[0;32m'
    YELLOW=$'\e[0;33m'
    BLUE=$'\e[0;34m'
    MAGENTA=$'\e[0;35m'
    CYAN=$'\e[0;36m'
    BOLD=$'\e[1m'
    DIM=$'\e[2m'
    RESET=$'\e[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' MAGENTA='' CYAN='' BOLD='' DIM='' RESET=''
fi

# ── Helpers ──────────────────────────────────────────────────────────────────

log()  { echo -e "${CYAN}▸${RESET} $*"; }
ok()   { echo -e "${GREEN}✓${RESET} $*"; }
warn() { echo -e "${YELLOW}⚠${RESET} $*"; }
err()  { echo -e "${RED}✗${RESET} $*" >&2; }
hdr()  { echo -e "\n${BOLD}${BLUE}── $* ──${RESET}\n"; }

has_cmd() { command -v "$1" &>/dev/null; }

require_cmd() {
    if ! has_cmd "$1"; then
        err "$1 is not installed"
        if [[ -n "${2:-}" ]]; then
            echo -e "  ${DIM}install: $2${RESET}" >&2
        fi
        exit 1
    fi
}

# ── Example Groups ───────────────────────────────────────────────────────────

GROUP_2D=(line_plot scatter_plot heatmap histogram bar_chart contour image_plot pcolormesh collections pie_chart stacked_area stairs stem_plot event_plot hexbin hist2d band rug)
GROUP_3D=(surface3d wireframe3d scatter3d bar3d contour3d quiver3d vector_field)
GROUP_STATISTICAL=(box_plot boxen violin_plot ecdf error_bar joint_plot swarm strip statistics trendline)
GROUP_SHOWCASE=(showcase_basic_2d showcase_3d showcase_fill showcase_grid showcase_statistical showcase_tri showcase_unicode showcase_features)
GROUP_SPECIALIZED=(radial ternary network parallel_coords sankey sunburst treemap dendrogram streamplot tricolor triplot scientific_dashboard theme_config)
GROUP_LAYOUT=(multi_panel facet_grid twin_axes inset)
GROUP_INTERACTIVE=(crosshair picking interactive_legend span_selector)
GROUP_FINANCE=(candlestick waterfall funnel gauge gantt)
GROUP_EXPORT=(kitty_export sixel_export toml_theme)

ALL_GROUPS=(2d 3d statistical showcase specialized layout interactive finance export)

# Feature flags for examples that require them
declare -A EXAMPLE_FEATURES=(
    [statistics]="statistics"
    [trendline]="statistics"
    [kitty_export]="kitty"
    [sixel_export]="sixel"
    [toml_theme]="toml-themes"
    [showcase_unicode]="unicode-extended"
    [showcase_features]="statistics"
)

get_group_examples() {
    local group="$1"
    case "$group" in
        2d)           echo "${GROUP_2D[@]}" ;;
        3d)           echo "${GROUP_3D[@]}" ;;
        statistical)  echo "${GROUP_STATISTICAL[@]}" ;;
        showcase)     echo "${GROUP_SHOWCASE[@]}" ;;
        specialized)  echo "${GROUP_SPECIALIZED[@]}" ;;
        layout)       echo "${GROUP_LAYOUT[@]}" ;;
        interactive)  echo "${GROUP_INTERACTIVE[@]}" ;;
        finance)      echo "${GROUP_FINANCE[@]}" ;;
        export)       echo "${GROUP_EXPORT[@]}" ;;
        *)            err "unknown group: $group"; return 1 ;;
    esac
}

get_all_examples() {
    for group in "${ALL_GROUPS[@]}"; do
        get_group_examples "$group"
    done | tr '\n' ' '
}

# ══════════════════════════════════════════════════════════════════════════════
# Commands
# ══════════════════════════════════════════════════════════════════════════════

# ── examples ─────────────────────────────────────────────────────────────────

pick_theme() {
    echo -e "\n${BOLD}Select theme:${RESET}"
    echo -e "  ${CYAN} 0${RESET}) auto (detect light/dark terminal)"
    echo -e "  ${CYAN} 1${RESET}) dark"
    echo -e "  ${CYAN} 2${RESET}) light"
    echo -e "  ${CYAN} 3${RESET}) minimal"
    echo -e "  ${CYAN} 4${RESET}) publication"
    echo -e "  ${CYAN} 5${RESET}) solarized"

    local choice
    echo -ne "\n${BOLD}choice [0]:${RESET} "
    read -r choice
    case "${choice:-0}" in
        0|auto)        THEME="" ;;
        1|dark)        THEME="dark" ;;
        2|light)       THEME="light" ;;
        3|minimal)     THEME="minimal" ;;
        4|publication) THEME="publication" ;;
        5|solarized)   THEME="solarized" ;;
        *)             THEME="$choice" ;;
    esac
}

run_example() {
    local name="$1"
    local profile="${2:-release}"
    local cargo_args=(--"$profile" --example "$name")

    local feature="${EXAMPLE_FEATURES[$name]:-}"
    if [[ -n "$feature" ]]; then
        cargo_args+=(--features "$feature")
    fi

    hdr "$name${feature:+ (--features $feature)}"
    cargo run "${cargo_args[@]}" "${theme_arg[@]}"
    ok "$name"
}

wait_for_input() {
    echo -ne "\n${DIM}press enter to continue, or n to quit...${RESET} "
    read -r ans
    case "$ans" in
        n|N|no|NO) log "stopped."; exit 0 ;;
    esac
}

run_example_list() {
    local pause="$1"; shift
    local profile="$1"; shift
    local examples=("$@")
    local total=${#examples[@]}
    local i=0

    for ex in "${examples[@]}"; do
        i=$((i + 1))
        log "[$i/$total]"
        run_example "$ex" "$profile"
        if [[ "$pause" == "true" ]] && (( i < total )); then
            wait_for_input
        fi
    done

    ok "all $total examples complete"
}

cmd_examples() {
    local run_all=false
    local group=""
    local theme_set=false
    local no_pause=false
    local profile="release"
    local list=false
    local single=""

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --all)       run_all=true ;;
            --group)     shift; group="$1" ;;
            --theme)     shift; THEME="$1"; theme_set=true ;;
            --no-pause)  no_pause=true ;;
            --release)   profile="release" ;;
            --debug)     profile="debug" ;;
            --list)      list=true ;;
            --help|-h)   cmd_examples_help; return ;;
            -*)          err "unknown flag: $1"; cmd_examples_help; return 1 ;;
            *)           single="$1" ;;
        esac
        shift
    done

    # List mode
    if $list; then
        hdr "example groups"
        for g in "${ALL_GROUPS[@]}"; do
            local examples
            read -ra examples <<< "$(get_group_examples "$g")"
            printf "  ${BOLD}%-14s${RESET} %d examples\n" "$g" "${#examples[@]}"
            for ex in "${examples[@]}"; do
                local feature="${EXAMPLE_FEATURES[$ex]:-}"
                if [[ -n "$feature" ]]; then
                    echo -e "    ${DIM}$ex${RESET} ${YELLOW}(--features $feature)${RESET}"
                else
                    echo -e "    ${DIM}$ex${RESET}"
                fi
            done
            echo ""
        done
        return
    fi

    # Build theme argument
    THEME="${THEME:-}"
    if ! $theme_set && { $run_all || [[ -n "$group" ]]; }; then
        pick_theme
    fi

    theme_arg=()
    if [[ -n "${THEME:-}" ]]; then
        theme_arg=("--" "$THEME")
        log "theme: ${BOLD}$THEME${RESET}"
    else
        log "theme: ${BOLD}auto${RESET}"
    fi

    # Single example
    if [[ -n "$single" ]]; then
        run_example "$single" "$profile"
        return
    fi

    # Group
    if [[ -n "$group" ]]; then
        local examples
        read -ra examples <<< "$(get_group_examples "$group")" || return 1
        hdr "group: $group (${#examples[@]} examples)"
        local pause="true"
        $no_pause && pause="false"
        run_example_list "$pause" "$profile" "${examples[@]}"
        return
    fi

    # All
    if $run_all; then
        local examples
        read -ra examples <<< "$(get_all_examples)"
        hdr "all examples (${#examples[@]})"
        local pause="true"
        $no_pause && pause="false"
        run_example_list "$pause" "$profile" "${examples[@]}"
        return
    fi

    # No args — show help
    cmd_examples_help
}

cmd_examples_help() {
    cat <<'EOF'
Usage: ./dev.sh examples [name] [flags]

Run examples — all, a group, or a single one.

Flags:
  --all              run all examples
  --group <name>     run a group (2d, 3d, statistical, showcase, specialized,
                     layout, interactive, finance, export)
  --theme <name>     set theme (dark, light, minimal, publication, solarized)
  --no-pause         skip continue prompt between examples
  --release          release build (default)
  --debug            debug build
  --list             list all groups and examples

Examples:
  ./dev.sh examples line_plot              # single example
  ./dev.sh examples line_plot --theme dark # with theme
  ./dev.sh examples --group 3d            # all 3D examples
  ./dev.sh examples --all --theme dark    # all examples, dark theme
  ./dev.sh examples --list                # list groups
EOF
}

# ── test ─────────────────────────────────────────────────────────────────────

cmd_test() {
    local run_all=false
    local mode=""
    local filter=""
    local threads=""
    local extra_args=()

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --all)        run_all=true ;;
            --release)    mode="release" ;;
            --filter)     shift; filter="$1" ;;
            --threads)    shift; threads="$1" ;;
            --help|-h)    cmd_test_help; return ;;
            *)            extra_args+=("$1") ;;
        esac
        shift
    done

    local cargo_args=()
    [[ "$mode" == "release" ]] && cargo_args+=(--release)
    $run_all && cargo_args+=(--all-features)

    local test_args=(--)
    [[ -n "$threads" ]] && test_args+=(--test-threads="$threads")
    [[ -n "$filter" ]]  && test_args+=("$filter")
    test_args+=("${extra_args[@]}")

    hdr "test ratatui-plt${mode:+ ($mode)}${run_all:+ (all features)}"
    log "cargo test ${cargo_args[*]} ${test_args[*]}"
    cargo test "${cargo_args[@]}" "${test_args[@]}"
    ok "tests passed"
}

cmd_test_help() {
    cat <<'EOF'
Usage: ./dev.sh test [flags]

Flags:
  --all               test with --all-features
  --release           release mode
  --filter <pattern>  filter test names
  --threads <n>       override --test-threads
EOF
}

# ── bench ────────────────────────────────────────────────────────────────────

cmd_bench() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h) cmd_bench_help; return ;;
            *)         ;;
        esac
        shift
    done

    log "no benchmarks configured yet."
    echo -e "  ${DIM}add criterion benches to benches/ to enable.${RESET}"
}

cmd_bench_help() {
    cat <<'EOF'
Usage: ./dev.sh bench [flags]

Flags:
  --filter <pattern>  filter benchmark names
  --save <name>       save baseline (criterion --save-baseline)
  --compare <name>    compare to baseline (criterion --baseline)

Note: No benchmarks are configured yet. Add criterion benches to benches/
      and update this command to enable.
EOF
}

# ── build ────────────────────────────────────────────────────────────────────

cmd_build() {
    local profile=""
    local features=""
    local run_all=false
    local extra_args=()

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --release)   profile="release" ;;
            --features)  shift; features="$1" ;;
            --all)       run_all=true ;;
            --help|-h)   cmd_build_help; return ;;
            *)           extra_args+=("$1") ;;
        esac
        shift
    done

    local build_args=()
    [[ -n "$profile" ]]  && build_args+=(--release)
    [[ -n "$features" ]] && build_args+=(--features "$features")
    $run_all && build_args+=(--all-features)
    build_args+=("${extra_args[@]}")

    hdr "build ratatui-plt${profile:+ ($profile)}${run_all:+ (all features)}"
    log "cargo build ${build_args[*]}"
    cargo build "${build_args[@]}"
    ok "build complete"
}

cmd_build_help() {
    cat <<'EOF'
Usage: ./dev.sh build [flags]

Flags:
  --release       release profile
  --features <f>  comma-separated features
  --all           build with --all-features
EOF
}

# ── lint ─────────────────────────────────────────────────────────────────────

cmd_lint() {
    local fix=false
    local run_all=false

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --fix)     fix=true ;;
            --all)     run_all=true ;;
            --help|-h) cmd_lint_help; return ;;
        esac
        shift
    done

    local feature_args=()
    $run_all && feature_args+=(--all-features)

    hdr "lint ratatui-plt"

    if $fix; then
        log "cargo clippy --fix --allow-dirty ${feature_args[*]}"
        cargo clippy --fix --allow-dirty "${feature_args[@]}"
        log "cargo fmt"
        cargo fmt
    else
        log "cargo clippy ${feature_args[*]}"
        cargo clippy "${feature_args[@]}"
        log "cargo fmt --check"
        cargo fmt --check
    fi
    ok "lint passed"
}

cmd_lint_help() {
    cat <<'EOF'
Usage: ./dev.sh lint [flags]

Flags:
  --fix   apply clippy fixes and format code
  --all   lint with --all-features
EOF
}

# ── clean ────────────────────────────────────────────────────────────────────

cmd_clean() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h) cmd_clean_help; return ;;
        esac
        shift
    done

    hdr "clean ratatui-plt"
    log "cargo clean"
    cargo clean
    ok "clean complete"
}

cmd_clean_help() {
    cat <<'EOF'
Usage: ./dev.sh clean

Removes all build artifacts (target/).
EOF
}

# ── doctor ───────────────────────────────────────────────────────────────────

TOOL_REGISTRY=(
    "rustc:rustc:curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh:required"
    "cargo:cargo:(installed with rustc):required"
    "clippy:cargo-clippy:rustup component add clippy:required"
    "rustfmt:rustfmt:rustup component add rustfmt:required"
)

check_tool() {
    local name="$1" cmd="$2" install="$3" category="$4"
    if has_cmd "$cmd"; then
        echo -e "  ${GREEN}✓${RESET} $name"
        return 0
    else
        local tag
        case "$category" in
            required)    tag="${RED}REQUIRED${RESET}" ;;
            recommended) tag="${YELLOW}recommended${RESET}" ;;
            optional)    tag="${DIM}optional${RESET}" ;;
        esac
        echo -e "  ${RED}✗${RESET} $name  [$tag]"
        echo -e "    ${DIM}→ $install${RESET}"
        return 1
    fi
}

cmd_doctor() {
    hdr "doctor — checking prerequisites"

    local missing_required=0

    for entry in "${TOOL_REGISTRY[@]}"; do
        IFS=':' read -r name cmd install category <<< "$entry"
        if ! check_tool "$name" "$cmd" "$install" "$category"; then
            case "$category" in
                required) missing_required=$((missing_required + 1)) ;;
            esac
        fi
    done

    echo ""
    if [[ $missing_required -eq 0 ]]; then
        ok "all tools installed"
    else
        err "$missing_required required tool(s) missing"
        exit 1
    fi
}

# ── info ─────────────────────────────────────────────────────────────────────

cmd_info() {
    hdr "system info"

    echo -e "  ${BOLD}project${RESET}    ratatui-plt"
    echo -e "  ${BOLD}directory${RESET}  $(pwd)"

    echo ""
    if has_cmd rustc; then
        echo -e "  ${BOLD}rustc${RESET}      $(rustc --version)"
    fi
    if has_cmd cargo; then
        echo -e "  ${BOLD}cargo${RESET}      $(cargo --version)"
    fi

    hdr "cargo features"
    echo "  crossterm        (default) crossterm backend"
    echo "  statistics       KDE, regression, LOWESS, bootstrap CI, trendlines"
    echo "  export           PNG raster export"
    echo "  kitty            Kitty graphics protocol"
    echo "  sixel            Sixel graphics protocol"
    echo "  toml-themes      TOML theme loading"
    echo "  async            Animation, streaming data"
    echo "  chrono           Timestamp axis support"
    echo "  serde            Serialization"
    echo "  fft              Power spectral density, spectrograms"
    echo "  triangulation    Delaunay triangulation"
    echo "  unicode-extended Extended Unicode characters"

    hdr "examples"
    local total=0
    for g in "${ALL_GROUPS[@]}"; do
        local examples
        read -ra examples <<< "$(get_group_examples "$g")"
        total=$((total + ${#examples[@]}))
    done
    echo -e "  ${BOLD}$total${RESET} examples in ${BOLD}${#ALL_GROUPS[@]}${RESET} groups"
    echo -e "  ${DIM}(run ./dev.sh examples --list for details)${RESET}"
}

# ── help ─────────────────────────────────────────────────────────────────────

cmd_help() {
    cat <<EOF
${BOLD}dev.sh${RESET} — development script for ${BOLD}ratatui-plt${RESET}

${BOLD}Usage:${RESET} ./dev.sh <command> [flags]

${BOLD}Commands:${RESET}
  ${CYAN}doctor${RESET}     check prerequisites
  ${CYAN}examples${RESET}   run examples (all, group, or single)
               --all  --group <name>  --theme <name>  --no-pause  --list
  ${CYAN}test${RESET}       run tests
               --release  --all  --filter <pattern>  --threads <n>
  ${CYAN}bench${RESET}      run benchmarks (not yet configured)
  ${CYAN}build${RESET}      build library
               --release  --all  --features <f>
  ${CYAN}lint${RESET}       clippy + fmt
               --fix  --all
  ${CYAN}clean${RESET}      clean build artifacts
  ${CYAN}info${RESET}       show project info, features, examples
  ${CYAN}help${RESET}       this message

${BOLD}Examples:${RESET}
  ./dev.sh doctor                              # check all prerequisites
  ./dev.sh examples line_plot --theme dark      # run single example
  ./dev.sh examples --group 3d                  # run 3D examples
  ./dev.sh examples --all --theme dark          # run all, dark theme
  ./dev.sh examples --list                      # list groups and examples
  ./dev.sh test                                 # run tests
  ./dev.sh test --all --release                 # all features, release mode
  ./dev.sh build --all                          # build with all features
  ./dev.sh lint --fix                           # auto-fix lint issues
  ./dev.sh info                                 # project info
EOF
}

# ══════════════════════════════════════════════════════════════════════════════
# Dispatch
# ══════════════════════════════════════════════════════════════════════════════

if [[ $# -lt 1 ]]; then
    cmd_help
    exit 0
fi

command="$1"; shift

case "$command" in
    examples)  cmd_examples "$@" ;;
    test)      cmd_test "$@" ;;
    bench)     cmd_bench "$@" ;;
    build)     cmd_build "$@" ;;
    lint)      cmd_lint "$@" ;;
    clean)     cmd_clean "$@" ;;
    doctor)    cmd_doctor ;;
    info)      cmd_info ;;
    help|-h|--help) cmd_help ;;
    *)
        err "unknown command: $command"
        cmd_help
        exit 1
        ;;
esac
