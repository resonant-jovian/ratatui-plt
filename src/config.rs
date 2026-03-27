//! Global plot configuration (rcParams-like).
//!
//! Provides a thread-local default configuration that widgets read
//! during construction, similar to matplotlib's `rcParams` system.
//!
//! # Rendering Backends
//!
//! The [`RenderBackend`] enum selects the rendering strategy.
//! The default is [`RenderBackend::Auto`], which auto-detects the
//! best available terminal graphics protocol:
//!
//! 1. **Kitty** (best quality) — pixel-level rendering via Kitty
//!    graphics protocol. Supported by Kitty, WezTerm, Ghostty.
//! 2. **Sixel** (broad support) — pixel-level rendering via Sixel
//!    protocol. Supported by foot, mlterm, contour, WezTerm,
//!    iTerm2, mintty, Rio.
//! 3. **Unicode** (universal) — Braille sub-pixel dots and
//!    half-block characters. Works in all terminals.
//!
//! # Detection Hierarchy
//!
//! Auto-detection uses two tiers:
//!
//! 1. **Environment variables** (fast, always checked first):
//!    - `RATATUI_PLT_BACKEND` — explicit override (`kitty`,
//!      `sixel`, or `unicode`)
//!    - `KITTY_WINDOW_ID` — set by the Kitty terminal itself
//!    - `TERM_PROGRAM` — checked against known terminal names
//!    - `SIXEL_SUPPORT` — explicit sixel opt-in
//! 2. **Escape sequence probing** (slow path, ~200ms):
//!    - Kitty graphics query sent to the terminal; a response
//!      indicates support
//!    - Only runs when env vars are inconclusive and the terminal
//!      is not yet in raw mode
//!
//! Detection results are cached per-process via [`OnceLock`].
//!
//! # Manual Override
//!
//! ```
//! use ratatui_plt::config::{PlotConfig, RenderBackend};
//!
//! let mut cfg = PlotConfig::get_default();
//! cfg.render_backend = RenderBackend::Kitty;
//! PlotConfig::set_default(cfg);
//! ```
//!
//! Or via environment variable before launch:
//! ```text
//! RATATUI_PLT_BACKEND=kitty cargo run --example showcase_basic_2d
//! ```

use std::io::IsTerminal;
use std::sync::{Once, OnceLock};
use std::time::Duration;

use crate::legend::LegendPosition;
use crate::style::MarkerShape;
use crate::theme::Theme;

/// Rendering backend selection for plot widgets.
///
/// The default is [`RenderBackend::Auto`], which auto-detects the
/// best available graphics protocol (Kitty > Sixel > Unicode).
///
/// # Example
///
/// ```
/// use ratatui_plt::config::{PlotConfig, RenderBackend};
///
/// let mut cfg = PlotConfig::get_default();
/// cfg.render_backend = RenderBackend::Kitty;
/// PlotConfig::set_default(cfg);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum RenderBackend {
    /// Auto-detect best available backend. Checks environment
    /// variables first, then probes via escape sequences.
    /// Falls back to Unicode if no graphics protocol is detected.
    #[default]
    Auto,
    /// Braille sub-pixel dots and half-block characters.
    /// Works in all terminals.
    Unicode,
    /// Pixel-level rendering via Kitty graphics protocol.
    /// Requires the `kitty` feature. Falls back to Unicode if
    /// the feature is not enabled.
    Kitty,
    /// Pixel-level rendering via Sixel protocol.
    /// Requires the `sixel` feature. Falls back to Unicode if
    /// the feature is not enabled.
    Sixel,
}

/// Cached detection result, computed at most once per process.
static DETECTED_BACKEND: OnceLock<RenderBackend> = OnceLock::new();

/// Ensures the Unicode fallback message is printed at most once.
static UNICODE_FALLBACK_MSG: Once = Once::new();

/// Detect the best available rendering backend.
///
/// Results are cached per-process. The first call performs
/// detection; subsequent calls return the cached result
/// instantly.
///
/// Detection order:
/// 1. `RATATUI_PLT_BACKEND` env var (explicit override)
/// 2. Kitty env-var indicators (`KITTY_WINDOW_ID`,
///    `TERM_PROGRAM`)
/// 3. Sixel env-var indicators (`SIXEL_SUPPORT`,
///    `TERM_PROGRAM`)
/// 4. Kitty escape sequence probe (~200ms timeout)
/// 5. Unicode fallback (prints one-time suggestion to stderr)
pub fn detect_backend() -> RenderBackend {
    DETECTED_BACKEND
        .get_or_init(|| {
            // Explicit override has highest priority
            if let Some(backend) = detect_override_env() {
                return backend;
            }

            // Fast path: env-var checks
            if detect_kitty_env() {
                return RenderBackend::Kitty;
            }
            if detect_sixel_env() {
                return RenderBackend::Sixel;
            }

            // Slow path: escape sequence probing
            if probe_kitty_graphics() {
                return RenderBackend::Kitty;
            }

            // Fallback: Unicode with one-time suggestion
            UNICODE_FALLBACK_MSG.call_once(|| {
                if std::io::stderr().is_terminal() {
                    eprintln!(
                        "ratatui-plt: using Unicode rendering. \
                         For pixel-perfect plots, use a terminal \
                         with Kitty or Sixel support \
                         (WezTerm, Kitty, foot, etc.) \
                         or set RATATUI_PLT_BACKEND=kitty|sixel"
                    );
                }
            });
            RenderBackend::Unicode
        })
        .clone()
}

/// Check for explicit backend override via `RATATUI_PLT_BACKEND`.
fn detect_override_env() -> Option<RenderBackend> {
    match std::env::var("RATATUI_PLT_BACKEND")
        .ok()
        .as_deref()
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("kitty") => Some(RenderBackend::Kitty),
        Some("sixel") => Some(RenderBackend::Sixel),
        Some("unicode") => Some(RenderBackend::Unicode),
        _ => None,
    }
}

/// Check for Kitty graphics support via environment variables.
///
/// Returns `true` if any of the following are set:
/// - `KITTY_WINDOW_ID` (set by the Kitty terminal itself)
/// - `TERM_PROGRAM` is one of: kitty, WezTerm, Ghostty
pub fn detect_kitty_env() -> bool {
    if std::env::var("KITTY_WINDOW_ID").is_ok() {
        return true;
    }
    matches!(
        std::env::var("TERM_PROGRAM").ok().as_deref(),
        Some("kitty") | Some("WezTerm") | Some("Ghostty")
    )
}

/// Check for Sixel support via environment variables.
///
/// Returns `true` if any of the following are set:
/// - `SIXEL_SUPPORT` environment variable (any value)
/// - `TERM_PROGRAM` is one of: foot, mlterm, contour, iTerm2,
///   mintty, Rio
pub fn detect_sixel_env() -> bool {
    if std::env::var("SIXEL_SUPPORT").is_ok() {
        return true;
    }
    matches!(
        std::env::var("TERM_PROGRAM").ok().as_deref(),
        Some("foot")
            | Some("mlterm")
            | Some("contour")
            | Some("iTerm2")
            | Some("mintty")
            | Some("Rio")
    )
}

/// Probe the terminal for Kitty graphics support via escape
/// sequences.
///
/// Sends a Kitty graphics query and checks if the terminal
/// responds within 200ms. Non-Kitty terminals silently ignore
/// APC sequences, so no response means no support.
///
/// Only runs if:
/// - stdout is a terminal (not piped/redirected)
/// - raw mode is not already active (avoids interfering with
///   an active TUI session)
fn probe_kitty_graphics() -> bool {
    use std::io::Write;

    // Only probe on a real terminal, before TUI setup
    if !std::io::stdout().is_terminal() {
        return false;
    }
    if crossterm::terminal::is_raw_mode_enabled()
        .unwrap_or(true)
    {
        return false;
    }

    // Enable raw mode temporarily for the probe
    if crossterm::terminal::enable_raw_mode().is_err() {
        return false;
    }

    // Send Kitty graphics query (ask terminal if it supports
    // graphics). The query uses action=query (a=q) with a
    // minimal 1x1 pixel payload.
    let query =
        b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\";
    let sent = std::io::stdout().write_all(query).is_ok()
        && std::io::stdout().flush().is_ok();

    let mut detected = false;
    if sent {
        // If the terminal supports Kitty graphics, it responds
        // with an APC sequence. Non-Kitty terminals silently
        // ignore APC, so poll() will time out.
        if let Ok(true) = crossterm::event::poll(
            Duration::from_millis(200),
        ) {
            detected = true;
            // Drain the response event to avoid leaving stale
            // data in the event queue.
            let _ = crossterm::event::read();
        }
    }

    let _ = crossterm::terminal::disable_raw_mode();
    detected
}

// Keep legacy names as public aliases for backward compat.
// Widgets that called detect_kitty/detect_sixel directly still
// compile.

/// Detect Kitty graphics support (env vars only).
///
/// Prefer [`detect_backend()`] which combines all detection
/// methods with caching.
pub fn detect_kitty() -> bool {
    detect_kitty_env()
}

/// Detect Sixel support (env vars only).
///
/// Prefer [`detect_backend()`] which combines all detection
/// methods with caching.
pub fn detect_sixel() -> bool {
    detect_sixel_env()
}

/// Global plot configuration controlling default widget behavior.
///
/// # Example
///
/// ```
/// use ratatui_plt::config::PlotConfig;
///
/// // Modify global defaults
/// let mut cfg = PlotConfig::get_default();
/// cfg.grid_visible = true;
/// cfg.legend_visible = false;
/// PlotConfig::set_default(cfg);
/// ```
#[derive(Clone, Debug)]
pub struct PlotConfig {
    /// Default theme for all widgets.
    pub theme: Theme,
    /// Default line width (maps to Thickness).
    pub default_line_width: u16,
    /// Default marker shape.
    pub default_marker: Option<MarkerShape>,
    /// Whether grid is visible by default.
    pub grid_visible: bool,
    /// Whether legend is visible by default.
    pub legend_visible: bool,
    /// Default legend position.
    pub legend_position: LegendPosition,
    /// Default colormap name.
    pub colormap: String,
    /// Rendering backend for plot widgets.
    pub render_backend: RenderBackend,
}

impl Default for PlotConfig {
    fn default() -> Self {
        Self {
            theme: Theme::get_default(),
            default_line_width: 1,
            default_marker: None,
            grid_visible: false,
            legend_visible: true,
            legend_position: LegendPosition::TopRight,
            colormap: "viridis".to_string(),
            render_backend: RenderBackend::default(),
        }
    }
}

impl PlotConfig {
    /// Set the global default configuration.
    pub fn set_default(config: PlotConfig) {
        DEFAULT_CONFIG.with(|c| {
            *c.borrow_mut() = config;
        });
    }

    /// Get a clone of the current global default configuration.
    pub fn get_default() -> PlotConfig {
        DEFAULT_CONFIG.with(|c| c.borrow().clone())
    }
}

std::thread_local! {
    static DEFAULT_CONFIG: std::cell::RefCell<PlotConfig> =
        std::cell::RefCell::new(PlotConfig::default());
}

/// RAII guard that restores the previous [`PlotConfig`] when
/// dropped.
///
/// Created by [`PlotConfig::activate`].
pub struct ConfigGuard {
    previous: PlotConfig,
}

impl Drop for ConfigGuard {
    fn drop(&mut self) {
        PlotConfig::set_default(self.previous.clone());
    }
}

impl PlotConfig {
    /// Activate this configuration as the global default,
    /// returning a guard that restores the previous
    /// configuration when dropped.
    pub fn activate(self) -> ConfigGuard {
        let previous = PlotConfig::get_default();
        PlotConfig::set_default(self);
        ConfigGuard { previous }
    }
}
