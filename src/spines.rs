//! Spine (axis border) visibility control.
//!
//! Controls which borders of the plot area are visible, similar to
//! matplotlib's `ax.spines['top'].set_visible(False)`.

/// Controls visibility of the four plot border edges (spines).
///
/// # Example
///
/// ```
/// use ratatui_sim::spines::Spines;
///
/// // Hide the top and right spines for a cleaner look
/// let spines = Spines::new().top(false).right(false);
/// ```
#[derive(Clone, Debug)]
pub struct Spines {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl Default for Spines {
    fn default() -> Self {
        Self {
            top: false,
            bottom: true,
            left: true,
            right: false,
        }
    }
}

impl Spines {
    /// Create default spines (bottom and left visible).
    pub fn new() -> Self {
        Self::default()
    }

    /// Show or hide all spines.
    pub fn all(visible: bool) -> Self {
        Self {
            top: visible,
            bottom: visible,
            left: visible,
            right: visible,
        }
    }

    /// Set top spine visibility.
    pub fn top(mut self, visible: bool) -> Self {
        self.top = visible;
        self
    }

    /// Set bottom spine visibility.
    pub fn bottom(mut self, visible: bool) -> Self {
        self.bottom = visible;
        self
    }

    /// Set left spine visibility.
    pub fn left(mut self, visible: bool) -> Self {
        self.left = visible;
        self
    }

    /// Set right spine visibility.
    pub fn right(mut self, visible: bool) -> Self {
        self.right = visible;
        self
    }
}
