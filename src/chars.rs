//! Centralized character constants for all rendering glyphs.
//!
//! The [`CharSet`] struct groups every character used for rendering plots,
//! from half-block fills to box-drawing borders to arrow glyphs.
//! It is stored on [`Theme`](crate::theme::Theme) so users can customize
//! rendering characters globally.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::chars::CharSet;
//!
//! let chars = CharSet::default();
//! assert_eq!(chars.fill.solid, '█');
//! assert_eq!(chars.arrow.right, '→');
//! ```

/// All character constants used for rendering, configurable via [`Theme`](crate::theme::Theme).
#[derive(Clone, Debug)]
pub struct CharSet {
    /// Half-block and fill characters for area rendering.
    pub fill: FillChars,
    /// Box-drawing characters for borders and structural lines.
    pub border: BorderChars,
    /// Grid line characters.
    pub grid: GridChars,
    /// Tick mark characters on axes.
    pub tick: TickChars,
    /// Arrow characters for vector fields, annotations, streamlines.
    pub arrow: ArrowChars,
    /// Dashed/dotted line characters.
    pub dash: DashChars,
    /// Arc quadrant characters (pie chart edges).
    pub arc: ArcChars,
    /// Colorbar extend indicator characters.
    pub colorbar: ColorbarChars,
    /// Default marker characters used when no explicit marker is specified.
    pub marker: MarkerChars,
    /// Depth-shading characters for 3D bar rendering.
    pub depth: DepthChars,
}

impl Default for CharSet {
    fn default() -> Self {
        Self {
            fill: FillChars::default(),
            border: BorderChars::default(),
            grid: GridChars::default(),
            tick: TickChars::default(),
            arrow: ArrowChars::default(),
            dash: DashChars::default(),
            arc: ArcChars::default(),
            colorbar: ColorbarChars::default(),
            marker: MarkerChars::default(),
            depth: DepthChars::default(),
        }
    }
}

/// Half-block and fill characters for area rendering.
#[derive(Clone, Debug)]
pub struct FillChars {
    /// Upper half block: `▀`
    pub half_upper: char,
    /// Lower half block: `▄`
    pub half_lower: char,
    /// Solid/full block: `█`
    pub solid: char,
    /// Dense shade: `▓`
    pub dense: char,
    /// Medium shade: `▒`
    pub medium: char,
    /// Light shade: `░`
    pub light: char,
}

impl Default for FillChars {
    fn default() -> Self {
        Self {
            half_upper: '▀',
            half_lower: '▄',
            solid: '█',
            dense: '▓',
            medium: '▒',
            light: '░',
        }
    }
}

/// Box-drawing characters for borders and structural lines.
#[derive(Clone, Debug)]
pub struct BorderChars {
    /// Horizontal line: `─`
    pub horizontal: char,
    /// Vertical line: `│`
    pub vertical: char,
    /// Top-left corner: `┌`
    pub top_left: char,
    /// Top-right corner: `┐`
    pub top_right: char,
    /// Bottom-left corner: `└`
    pub bottom_left: char,
    /// Bottom-right corner: `┘`
    pub bottom_right: char,
    /// Cross/intersection: `┼`
    pub cross: char,
    /// T-down: `┬`
    pub tee_down: char,
    /// T-up: `┴`
    pub tee_up: char,
    /// T-right: `├`
    pub tee_right: char,
    /// T-left: `┤`
    pub tee_left: char,
}

impl Default for BorderChars {
    fn default() -> Self {
        Self {
            horizontal: '─',
            vertical: '│',
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
            cross: '┼',
            tee_down: '┬',
            tee_up: '┴',
            tee_right: '├',
            tee_left: '┤',
        }
    }
}

/// Grid line characters.
#[derive(Clone, Debug)]
pub struct GridChars {
    /// Major horizontal grid line: `─`
    pub major_h: char,
    /// Major vertical grid line: `│`
    pub major_v: char,
    /// Grid intersection: `┼`
    pub intersection: char,
    /// Minor horizontal grid line (dashed): `┄`
    pub minor_h: char,
    /// Minor vertical grid line (dashed): `┆`
    pub minor_v: char,
}

impl Default for GridChars {
    fn default() -> Self {
        Self {
            major_h: '─',
            major_v: '│',
            intersection: '┼',
            minor_h: '┄',
            minor_v: '┆',
        }
    }
}

/// Tick mark characters on axes.
#[derive(Clone, Debug)]
pub struct TickChars {
    /// Upward tick cap: `┬`
    pub cap_top: char,
    /// Downward tick cap: `┴`
    pub cap_bottom: char,
    /// Left tick cap: `├`
    pub cap_left: char,
    /// Right tick cap: `┤`
    pub cap_right: char,
}

impl Default for TickChars {
    fn default() -> Self {
        Self {
            cap_top: '┬',
            cap_bottom: '┴',
            cap_left: '├',
            cap_right: '┤',
        }
    }
}

/// Arrow characters for vector fields, annotations, and streamlines.
#[derive(Clone, Debug)]
pub struct ArrowChars {
    /// Left arrow: `←`
    pub left: char,
    /// Right arrow: `→`
    pub right: char,
    /// Up arrow: `↑`
    pub up: char,
    /// Down arrow: `↓`
    pub down: char,
    /// North-east arrow: `↗`
    pub ne: char,
    /// North-west arrow: `↖`
    pub nw: char,
    /// South-east arrow: `↘`
    pub se: char,
    /// South-west arrow: `↙`
    pub sw: char,
}

impl Default for ArrowChars {
    fn default() -> Self {
        Self {
            left: '←',
            right: '→',
            up: '↑',
            down: '↓',
            ne: '↗',
            nw: '↖',
            se: '↘',
            sw: '↙',
        }
    }
}

/// Dashed and dotted line characters.
#[derive(Clone, Debug)]
pub struct DashChars {
    /// Horizontal dash: `╌`
    pub h: char,
    /// Vertical dash: `╎`
    pub v: char,
    /// Bold/heavy horizontal line: `━`
    pub bold_h: char,
}

impl Default for DashChars {
    fn default() -> Self {
        Self {
            h: '╌',
            v: '╎',
            bold_h: '━',
        }
    }
}

/// Arc quadrant characters for pie chart edges.
#[derive(Clone, Debug)]
pub struct ArcChars {
    /// Top-left arc: `◜`
    pub top_left: char,
    /// Top-right arc: `◝`
    pub top_right: char,
    /// Bottom-right arc: `◞`
    pub bottom_right: char,
    /// Bottom-left arc: `◟`
    pub bottom_left: char,
}

impl Default for ArcChars {
    fn default() -> Self {
        Self {
            top_left: '◜',
            top_right: '◝',
            bottom_right: '◞',
            bottom_left: '◟',
        }
    }
}

/// Colorbar extend indicator characters.
#[derive(Clone, Debug)]
pub struct ColorbarChars {
    /// Upward triangle for max extend: `▲`
    pub extend_max: char,
    /// Downward triangle for min extend: `▼`
    pub extend_min: char,
}

impl Default for ColorbarChars {
    fn default() -> Self {
        Self {
            extend_max: '▲',
            extend_min: '▼',
        }
    }
}

/// Default marker characters used when no explicit marker shape is specified.
#[derive(Clone, Debug)]
pub struct MarkerChars {
    /// Default filled circle point: `●`
    pub default_point: char,
    /// Small bullet point: `•`
    pub small_point: char,
    /// Small filled square: `▪`
    pub center_dot: char,
    /// Legend line marker fallback: `━`
    pub legend_line: char,
    /// Gauge hub center: `●`
    pub gauge_hub: char,
    /// Box-plot whisker dash: `┆`
    pub whisker_dash: char,
}

impl Default for MarkerChars {
    fn default() -> Self {
        Self {
            default_point: '●',
            small_point: '•',
            center_dot: '▪',
            legend_line: '━',
            gauge_hub: '●',
            whisker_dash: '┆',
        }
    }
}

/// Depth-shading characters for 3D bar rendering.
#[derive(Clone, Debug)]
pub struct DepthChars {
    /// Front face (solid): `█`
    pub front: char,
    /// Side face near: `▓`
    pub side_near: char,
    /// Side face far: `▒`
    pub side_far: char,
    /// Top face near: `▓`
    pub top_near: char,
    /// Top face far: `▒`
    pub top_far: char,
}

impl Default for DepthChars {
    fn default() -> Self {
        Self {
            front: '█',
            side_near: '▓',
            side_far: '▒',
            top_near: '▓',
            top_far: '▒',
        }
    }
}
