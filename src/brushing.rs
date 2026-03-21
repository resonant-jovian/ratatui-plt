//! Linked brushing for interactive data selection across multiple plots.
//!
//! Provides a shared selection state that can be used to highlight corresponding
//! data points across linked visualizations.

use std::cell::RefCell;
use std::rc::Rc;

/// Shared brushing state for linked selection across plots.
///
/// When a rectangular region is selected in one plot, all linked plots can
/// highlight the corresponding data points using the shared `selected_indices`.
///
/// # Example
///
/// ```
/// use ratatui_plt::brushing::BrushState;
/// use std::rc::Rc;
/// use std::cell::RefCell;
///
/// let brush = Rc::new(RefCell::new(BrushState::new()));
///
/// // Set a selection rectangle in data coordinates
/// brush.borrow_mut().set_selection(0.0, 0.0, 5.0, 10.0);
/// ```
#[derive(Clone, Debug, Default)]
pub struct BrushState {
    /// Selected rectangular region in data coordinates: (x_min, y_min, x_max, y_max).
    pub selection: Option<(f64, f64, f64, f64)>,
    /// Per-series selected point indices. Outer vec indexed by series index.
    pub selected_indices: Vec<Vec<usize>>,
}

impl BrushState {
    /// Create an empty brush state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the rectangular selection region in data coordinates.
    pub fn set_selection(&mut self, x_min: f64, y_min: f64, x_max: f64, y_max: f64) {
        self.selection = Some((
            x_min.min(x_max),
            y_min.min(y_max),
            x_min.max(x_max),
            y_max.max(y_min),
        ));
    }

    /// Clear the selection.
    pub fn clear(&mut self) {
        self.selection = None;
        self.selected_indices.clear();
    }

    /// Check if a data point falls within the current selection.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        if let Some((x_min, y_min, x_max, y_max)) = self.selection {
            x >= x_min && x <= x_max && y >= y_min && y <= y_max
        } else {
            false
        }
    }

    /// Update the selected indices for multiple series based on the current selection.
    ///
    /// Each inner slice is the (x, y) data for one series. After calling this,
    /// `selected_indices[i]` contains the indices of points in series `i` that
    /// fall within the selection rectangle.
    pub fn update_indices(&mut self, series_data: &[&[(f64, f64)]]) {
        self.selected_indices.clear();
        for &data in series_data {
            let mut indices = Vec::new();
            for (i, &(x, y)) in data.iter().enumerate() {
                if self.contains(x, y) {
                    indices.push(i);
                }
            }
            self.selected_indices.push(indices);
        }
    }
}

/// Convenience type for a shared brush state (single-threaded).
pub type SharedBrush = Rc<RefCell<BrushState>>;

/// Create a new shared brush state.
pub fn shared_brush() -> SharedBrush {
    Rc::new(RefCell::new(BrushState::new()))
}
