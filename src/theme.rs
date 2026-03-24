//! Themes and stylesheets for consistent plot styling.
//!
//! Themes control colors, fonts, grid visibility, and other visual properties
//! across all widgets. Inspired by matplotlib's `plt.style.use()`.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::theme::Theme;
//!
//! // Use the dark theme (good for most terminals)
//! let theme = Theme::dark();
//!
//! // Or set it as the global default
//! Theme::set_default(Theme::dark());
//! ```

use ratatui::style::Color;

use crate::color_cycle::ColorCycle;
use crate::style::DashPattern;

/// A theme controlling the visual appearance of all plot elements.
#[derive(Clone, Debug)]
pub struct Theme {
    /// Background color for the plot area.
    pub background: Color,
    /// Default text color (titles, labels, tick marks).
    pub foreground: Color,
    /// Color for grid lines.
    pub grid_color: Color,
    /// Color for minor grid lines (dimmer than major grid by default).
    pub minor_grid_color: Color,
    /// Color for axis borders/spines.
    pub axis_color: Color,
    /// Color cycle for auto-coloring series.
    pub color_cycle: ColorCycle,
    /// Whether to show grid lines by default.
    pub grid_visible: bool,
    /// Grid line dash pattern.
    pub grid_pattern: DashPattern,
    /// Whether titles should be bold.
    pub bold_title: bool,

    // --- Named palette ---

    /// Primary data color (main series, single-dataset widgets).
    pub primary: Color,
    /// Secondary data color (second series, comparisons).
    pub secondary: Color,
    /// Accent color (interactive selectors, emphasis).
    pub accent: Color,
    /// Highlight color (crosshair, callouts, annotations).
    pub highlight: Color,
    /// Muted color (outlines, low-priority chrome, decorative arcs).
    pub muted: Color,
    /// Surface color (panel/inset backgrounds, filled regions).
    pub surface: Color,

    // --- Semantic colors ---

    /// Positive-value color (waterfall gains, upward trends).
    pub positive_color: Color,
    /// Negative-value color (waterfall losses, downward trends).
    pub negative_color: Color,
    /// Neutral/total color (waterfall totals, baselines).
    pub neutral_color: Color,

    // --- 3D axis colors ---

    /// X-axis color in 3D plots.
    pub x_axis_3d_color: Color,
    /// Y-axis color in 3D plots.
    pub y_axis_3d_color: Color,
    /// Z-axis color in 3D plots.
    pub z_axis_3d_color: Color,

    // --- Chrome / decoration ---

    /// Color for disabled/hidden UI elements (e.g. hidden legend entries).
    pub disabled_color: Color,
    /// Color for bad/missing data cells (NaN in heatmaps).
    pub bad_data_color: Color,
    /// Default color for text annotations.
    pub annotation_color: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Dark theme — light text on dark background. Good for most terminals.
    pub fn dark() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::White,
            grid_color: Color::Rgb(75, 75, 75),
            minor_grid_color: Color::Rgb(50, 50, 50),
            axis_color: Color::Rgb(180, 180, 180),
            color_cycle: ColorCycle::default(),
            grid_visible: true,
            grid_pattern: DashPattern::Solid,
            bold_title: true,
            primary: Color::Rgb(31, 119, 180),
            secondary: Color::Rgb(255, 127, 14),
            accent: Color::Cyan,
            highlight: Color::Yellow,
            muted: Color::DarkGray,
            surface: Color::Rgb(40, 40, 40),
            positive_color: Color::Green,
            negative_color: Color::Red,
            neutral_color: Color::Blue,
            x_axis_3d_color: Color::Red,
            y_axis_3d_color: Color::Green,
            z_axis_3d_color: Color::Blue,
            disabled_color: Color::DarkGray,
            bad_data_color: Color::DarkGray,
            annotation_color: Color::White,
        }
    }

    /// Light theme — dark text, suitable for light terminal backgrounds.
    pub fn light() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::Black,
            grid_color: Color::Rgb(180, 180, 180),
            minor_grid_color: Color::Rgb(205, 205, 205),
            axis_color: Color::Rgb(60, 60, 60),
            color_cycle: ColorCycle::default(),
            grid_visible: true,
            grid_pattern: DashPattern::Solid,
            bold_title: true,
            primary: Color::Rgb(31, 119, 180),
            secondary: Color::Rgb(255, 127, 14),
            accent: Color::Rgb(0, 150, 150),
            highlight: Color::Rgb(180, 150, 0),
            muted: Color::Rgb(180, 180, 180),
            surface: Color::Rgb(240, 240, 240),
            positive_color: Color::Rgb(44, 160, 44),
            negative_color: Color::Rgb(214, 39, 40),
            neutral_color: Color::Rgb(31, 119, 180),
            x_axis_3d_color: Color::Rgb(214, 39, 40),
            y_axis_3d_color: Color::Rgb(44, 160, 44),
            z_axis_3d_color: Color::Rgb(31, 119, 180),
            disabled_color: Color::Rgb(180, 180, 180),
            bad_data_color: Color::Rgb(180, 180, 180),
            annotation_color: Color::Black,
        }
    }

    /// Minimal theme — reduced chrome, clean look.
    pub fn minimal() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::White,
            grid_color: Color::Rgb(80, 80, 80),
            minor_grid_color: Color::Rgb(50, 50, 50),
            axis_color: Color::Rgb(120, 120, 120),
            color_cycle: ColorCycle::default(),
            grid_visible: false,
            grid_pattern: DashPattern::Solid,
            bold_title: false,
            primary: Color::Rgb(31, 119, 180),
            secondary: Color::Rgb(255, 127, 14),
            accent: Color::Cyan,
            highlight: Color::Yellow,
            muted: Color::Rgb(80, 80, 80),
            surface: Color::Rgb(30, 30, 30),
            positive_color: Color::Green,
            negative_color: Color::Red,
            neutral_color: Color::Blue,
            x_axis_3d_color: Color::Red,
            y_axis_3d_color: Color::Green,
            z_axis_3d_color: Color::Blue,
            disabled_color: Color::Rgb(80, 80, 80),
            bad_data_color: Color::Rgb(80, 80, 80),
            annotation_color: Color::White,
        }
    }

    /// Publication theme — high contrast, monochrome-friendly.
    pub fn publication() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::White,
            grid_color: Color::Rgb(80, 80, 80),
            minor_grid_color: Color::Rgb(50, 50, 50),
            axis_color: Color::White,
            color_cycle: ColorCycle::new(vec![
                Color::White,
                Color::Rgb(200, 200, 200),
                Color::Rgb(150, 150, 150),
                Color::Rgb(100, 100, 100),
            ]),
            grid_visible: true,
            grid_pattern: DashPattern::Solid,
            bold_title: true,
            primary: Color::White,
            secondary: Color::Rgb(200, 200, 200),
            accent: Color::Rgb(150, 150, 150),
            highlight: Color::White,
            muted: Color::Rgb(100, 100, 100),
            surface: Color::Rgb(30, 30, 30),
            positive_color: Color::Rgb(200, 200, 200),
            negative_color: Color::Rgb(150, 150, 150),
            neutral_color: Color::White,
            x_axis_3d_color: Color::Rgb(200, 200, 200),
            y_axis_3d_color: Color::Rgb(150, 150, 150),
            z_axis_3d_color: Color::White,
            disabled_color: Color::Rgb(80, 80, 80),
            bad_data_color: Color::Rgb(80, 80, 80),
            annotation_color: Color::White,
        }
    }

    /// Solarized dark theme.
    pub fn solarized() -> Self {
        Self {
            background: Color::Rgb(0, 43, 54),
            foreground: Color::Rgb(131, 148, 150),
            grid_color: Color::Rgb(30, 70, 80),
            minor_grid_color: Color::Rgb(20, 55, 65),
            axis_color: Color::Rgb(88, 110, 117),
            color_cycle: ColorCycle::new(vec![
                Color::Rgb(38, 139, 210),  // blue
                Color::Rgb(211, 54, 130),  // magenta
                Color::Rgb(133, 153, 0),   // green
                Color::Rgb(203, 75, 22),   // orange
                Color::Rgb(108, 113, 196), // violet
                Color::Rgb(42, 161, 152),  // cyan
                Color::Rgb(181, 137, 0),   // yellow
                Color::Rgb(220, 50, 47),   // red
            ]),
            grid_visible: true,
            grid_pattern: DashPattern::Solid,
            bold_title: true,
            primary: Color::Rgb(38, 139, 210),   // blue
            secondary: Color::Rgb(211, 54, 130),  // magenta
            accent: Color::Rgb(42, 161, 152),     // cyan
            highlight: Color::Rgb(181, 137, 0),    // yellow
            muted: Color::Rgb(88, 110, 117),       // base01
            surface: Color::Rgb(7, 54, 66),        // base02
            positive_color: Color::Rgb(133, 153, 0),   // green
            negative_color: Color::Rgb(220, 50, 47),    // red
            neutral_color: Color::Rgb(38, 139, 210),    // blue
            x_axis_3d_color: Color::Rgb(220, 50, 47),   // red
            y_axis_3d_color: Color::Rgb(133, 153, 0),   // green
            z_axis_3d_color: Color::Rgb(38, 139, 210),  // blue
            disabled_color: Color::Rgb(88, 110, 117),    // base01
            bad_data_color: Color::Rgb(88, 110, 117),    // base01
            annotation_color: Color::Rgb(131, 148, 150), // base0
        }
    }

    /// Auto-detect whether the terminal has a light or dark background and
    /// return the appropriate theme.
    ///
    /// Detection checks (in order):
    /// 1. `COLORFGBG` env var (xterm, rxvt, and others)
    /// 2. `GTK_THEME` env var (`:dark` suffix → dark)
    /// 3. GNOME/freedesktop color-scheme via `gsettings` (Linux desktops)
    ///
    /// Falls back to [`Theme::dark()`] if detection fails.
    pub fn auto() -> Self {
        // 1. COLORFGBG — format "fg;bg", bg > 8 means light background.
        if let Ok(val) = std::env::var("COLORFGBG")
            && let Some(bg) = val.rsplit(';').next().and_then(|s| s.parse::<u8>().ok())
        {
            return if bg > 8 { Self::light() } else { Self::dark() };
        }

        // 2. GTK_THEME — e.g. "Adwaita:dark" or "Yaru-dark".
        if let Ok(gtk) = std::env::var("GTK_THEME") {
            let lower = gtk.to_lowercase();
            if lower.contains("dark") {
                return Self::dark();
            }
            if !lower.is_empty() {
                return Self::light();
            }
        }

        // 3. GNOME / freedesktop color-scheme preference.
        if let Ok(output) = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("prefer-dark") {
                return Self::dark();
            }
            if stdout.contains("default") || stdout.contains("prefer-light") {
                return Self::light();
            }
        }

        Self::dark()
    }

    /// Set the global default theme.
    pub fn set_default(theme: Theme) {
        DEFAULT_THEME.with(|t| {
            *t.borrow_mut() = theme;
        });
    }

    /// Get a clone of the current global default theme.
    pub fn get_default() -> Theme {
        DEFAULT_THEME.with(|t| t.borrow().clone())
    }
}

std::thread_local! {
    static DEFAULT_THEME: std::cell::RefCell<Theme> = std::cell::RefCell::new(Theme::dark());
}

/// RAII guard that restores the previous [`Theme`] when dropped.
///
/// Created by [`Theme::activate`].
///
/// # Example
///
/// ```
/// use ratatui_plt::theme::Theme;
///
/// {
///     let _guard = Theme::solarized().activate();
///     // All widgets constructed here use solarized theme
/// } // previous theme is restored
/// ```
pub struct ThemeGuard {
    previous: Theme,
}

impl Drop for ThemeGuard {
    fn drop(&mut self) {
        Theme::set_default(self.previous.clone());
    }
}

impl Theme {
    /// Activate this theme as the global default, returning a guard
    /// that restores the previous theme when dropped.
    pub fn activate(self) -> ThemeGuard {
        let previous = Theme::get_default();
        Theme::set_default(self);
        ThemeGuard { previous }
    }
}

// ---------------------------------------------------------------------------
// TOML theme loading (behind toml-themes feature)
// ---------------------------------------------------------------------------

/// Error type for TOML theme parsing and loading.
#[cfg(feature = "toml-themes")]
#[derive(Debug)]
pub enum ThemeError {
    /// I/O error reading a theme file.
    Io(std::io::Error),
    /// TOML parsing error.
    Parse(String),
    /// Invalid color string.
    InvalidColor(String),
}

#[cfg(feature = "toml-themes")]
impl std::fmt::Display for ThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Parse(e) => write!(f, "TOML parse error: {e}"),
            Self::InvalidColor(e) => write!(f, "Invalid color: {e}"),
        }
    }
}

#[cfg(feature = "toml-themes")]
impl std::error::Error for ThemeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(_) | Self::InvalidColor(_) => None,
        }
    }
}

#[cfg(feature = "toml-themes")]
impl From<std::io::Error> for ThemeError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(feature = "toml-themes")]
#[derive(serde::Deserialize)]
struct TomlTheme {
    colors: Option<TomlColors>,
    palette: Option<TomlPalette>,
    semantic: Option<TomlSemantic>,
    axes_3d: Option<TomlAxes3d>,
    chrome: Option<TomlChrome>,
    grid: Option<TomlGrid>,
    cycle: Option<TomlCycle>,
}

#[cfg(feature = "toml-themes")]
#[derive(serde::Deserialize)]
struct TomlColors {
    background: Option<String>,
    foreground: Option<String>,
    grid: Option<String>,
    minor_grid: Option<String>,
    axis: Option<String>,
}

#[cfg(feature = "toml-themes")]
#[derive(serde::Deserialize)]
struct TomlGrid {
    visible: Option<bool>,
    pattern: Option<String>,
    bold_title: Option<bool>,
}

#[cfg(feature = "toml-themes")]
#[derive(serde::Deserialize)]
struct TomlCycle {
    colors: Option<Vec<String>>,
}

#[cfg(feature = "toml-themes")]
#[derive(serde::Deserialize)]
struct TomlPalette {
    primary: Option<String>,
    secondary: Option<String>,
    accent: Option<String>,
    highlight: Option<String>,
    muted: Option<String>,
    surface: Option<String>,
}

#[cfg(feature = "toml-themes")]
#[derive(serde::Deserialize)]
struct TomlSemantic {
    positive: Option<String>,
    negative: Option<String>,
    neutral: Option<String>,
}

#[cfg(feature = "toml-themes")]
#[derive(serde::Deserialize)]
struct TomlAxes3d {
    x_axis: Option<String>,
    y_axis: Option<String>,
    z_axis: Option<String>,
}

#[cfg(feature = "toml-themes")]
#[derive(serde::Deserialize)]
struct TomlChrome {
    disabled: Option<String>,
    bad_data: Option<String>,
    annotation: Option<String>,
}

/// Parse a color string into a ratatui [`Color`].
///
/// Supported formats:
/// - `"#RRGGBB"` — hex RGB (e.g., `"#ff8000"`)
/// - `"reset"` — [`Color::Reset`]
/// - Named colors: `"red"`, `"blue"`, `"green"`, `"yellow"`, `"magenta"`,
///   `"cyan"`, `"white"`, `"black"`, `"gray"`, `"darkgray"`,
///   `"lightred"`, `"lightgreen"`, `"lightyellow"`, `"lightblue"`,
///   `"lightmagenta"`, `"lightcyan"`
#[cfg(feature = "toml-themes")]
fn parse_color(s: &str) -> Result<Color, ThemeError> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() != 6 {
            return Err(ThemeError::InvalidColor(format!(
                "expected 6 hex digits after '#', got '{s}'"
            )));
        }
        let r = u8::from_str_radix(&hex[0..2], 16)
            .map_err(|_| ThemeError::InvalidColor(format!("invalid hex color '{s}'")))?;
        let g = u8::from_str_radix(&hex[2..4], 16)
            .map_err(|_| ThemeError::InvalidColor(format!("invalid hex color '{s}'")))?;
        let b = u8::from_str_radix(&hex[4..6], 16)
            .map_err(|_| ThemeError::InvalidColor(format!("invalid hex color '{s}'")))?;
        return Ok(Color::Rgb(r, g, b));
    }

    match s.to_lowercase().as_str() {
        "reset" => Ok(Color::Reset),
        "black" => Ok(Color::Black),
        "red" => Ok(Color::Red),
        "green" => Ok(Color::Green),
        "yellow" => Ok(Color::Yellow),
        "blue" => Ok(Color::Blue),
        "magenta" => Ok(Color::Magenta),
        "cyan" => Ok(Color::Cyan),
        "gray" | "grey" => Ok(Color::Gray),
        "darkgray" | "darkgrey" => Ok(Color::DarkGray),
        "lightred" => Ok(Color::LightRed),
        "lightgreen" => Ok(Color::LightGreen),
        "lightyellow" => Ok(Color::LightYellow),
        "lightblue" => Ok(Color::LightBlue),
        "lightmagenta" => Ok(Color::LightMagenta),
        "lightcyan" => Ok(Color::LightCyan),
        "white" => Ok(Color::White),
        _ => Err(ThemeError::InvalidColor(format!("unknown color '{s}'"))),
    }
}

/// Parse a grid pattern string into a [`DashPattern`].
#[cfg(feature = "toml-themes")]
fn parse_pattern(s: &str) -> Result<DashPattern, ThemeError> {
    match s.trim().to_lowercase().as_str() {
        "solid" => Ok(DashPattern::Solid),
        "dashed" => Ok(DashPattern::Dashed),
        "dotted" => Ok(DashPattern::Dotted),
        "dashdot" | "dash-dot" => Ok(DashPattern::DashDot),
        _ => Err(ThemeError::Parse(format!("unknown grid pattern '{s}'"))),
    }
}

/// Parse a TOML string into a [`Theme`].
///
/// Starts with [`Theme::dark()`] as a base and overrides any fields that
/// are present in the TOML document.
///
/// # TOML Format
///
/// ```toml
/// [colors]
/// background = "#1a1a2e"
/// foreground = "#e0e0e0"
/// grid = "#333333"
/// minor_grid = "#222222"
/// axis = "gray"
///
/// [palette]
/// primary = "#1f77b4"
/// secondary = "#ff7f0e"
/// accent = "cyan"
/// highlight = "yellow"
/// muted = "darkgray"
/// surface = "#282828"
///
/// [semantic]
/// positive = "green"
/// negative = "red"
/// neutral = "blue"
///
/// [axes_3d]
/// x_axis = "red"
/// y_axis = "green"
/// z_axis = "blue"
///
/// [chrome]
/// disabled = "darkgray"
/// bad_data = "darkgray"
/// annotation = "white"
///
/// [grid]
/// visible = true
/// pattern = "dashed"    # "solid", "dashed", "dotted", "dashdot"
/// bold_title = true
///
/// [cycle]
/// colors = ["#e94560", "#0f3460", "#16c79a"]
/// ```
#[cfg(feature = "toml-themes")]
pub fn theme_from_toml(toml_str: &str) -> Result<Theme, ThemeError> {
    let parsed: TomlTheme =
        toml::from_str(toml_str).map_err(|e| ThemeError::Parse(e.to_string()))?;

    let mut theme = Theme::dark();

    if let Some(colors) = parsed.colors {
        if let Some(ref s) = colors.background {
            theme.background = parse_color(s)?;
        }
        if let Some(ref s) = colors.foreground {
            theme.foreground = parse_color(s)?;
        }
        if let Some(ref s) = colors.grid {
            theme.grid_color = parse_color(s)?;
        }
        if let Some(ref s) = colors.minor_grid {
            theme.minor_grid_color = parse_color(s)?;
        }
        if let Some(ref s) = colors.axis {
            theme.axis_color = parse_color(s)?;
        }
    }

    if let Some(palette) = parsed.palette {
        if let Some(ref s) = palette.primary {
            theme.primary = parse_color(s)?;
        }
        if let Some(ref s) = palette.secondary {
            theme.secondary = parse_color(s)?;
        }
        if let Some(ref s) = palette.accent {
            theme.accent = parse_color(s)?;
        }
        if let Some(ref s) = palette.highlight {
            theme.highlight = parse_color(s)?;
        }
        if let Some(ref s) = palette.muted {
            theme.muted = parse_color(s)?;
        }
        if let Some(ref s) = palette.surface {
            theme.surface = parse_color(s)?;
        }
    }

    if let Some(semantic) = parsed.semantic {
        if let Some(ref s) = semantic.positive {
            theme.positive_color = parse_color(s)?;
        }
        if let Some(ref s) = semantic.negative {
            theme.negative_color = parse_color(s)?;
        }
        if let Some(ref s) = semantic.neutral {
            theme.neutral_color = parse_color(s)?;
        }
    }

    if let Some(axes_3d) = parsed.axes_3d {
        if let Some(ref s) = axes_3d.x_axis {
            theme.x_axis_3d_color = parse_color(s)?;
        }
        if let Some(ref s) = axes_3d.y_axis {
            theme.y_axis_3d_color = parse_color(s)?;
        }
        if let Some(ref s) = axes_3d.z_axis {
            theme.z_axis_3d_color = parse_color(s)?;
        }
    }

    if let Some(chrome) = parsed.chrome {
        if let Some(ref s) = chrome.disabled {
            theme.disabled_color = parse_color(s)?;
        }
        if let Some(ref s) = chrome.bad_data {
            theme.bad_data_color = parse_color(s)?;
        }
        if let Some(ref s) = chrome.annotation {
            theme.annotation_color = parse_color(s)?;
        }
    }

    if let Some(grid) = parsed.grid {
        if let Some(v) = grid.visible {
            theme.grid_visible = v;
        }
        if let Some(ref s) = grid.pattern {
            theme.grid_pattern = parse_pattern(s)?;
        }
        if let Some(v) = grid.bold_title {
            theme.bold_title = v;
        }
    }

    if let Some(cycle) = parsed.cycle
        && let Some(color_strings) = cycle.colors
    {
        let mut colors = Vec::with_capacity(color_strings.len());
        for s in &color_strings {
            colors.push(parse_color(s)?);
        }
        theme.color_cycle = ColorCycle::new(colors);
    }

    Ok(theme)
}

/// Load a [`Theme`] from a TOML file at the given path.
///
/// Reads the file contents and delegates to [`theme_from_toml`].
#[cfg(feature = "toml-themes")]
pub fn load_theme(path: impl AsRef<std::path::Path>) -> Result<Theme, ThemeError> {
    let contents = std::fs::read_to_string(path)?;
    theme_from_toml(&contents)
}
