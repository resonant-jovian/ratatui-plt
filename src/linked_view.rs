//! Linked view state for synchronizing axis bounds across multiple plots.
//!
//! Provides a shared view state that can be used to link pan/zoom across
//! multiple plot widgets so they always display the same data region.

use std::cell::RefCell;
use std::rc::Rc;

/// Shared view state containing optional axis bounds overrides.
///
/// When set, the x and/or y bounds override any auto-computed bounds
/// in linked plot widgets.
///
/// # Example
///
/// ```
/// use ratatui_plt::linked_view::{SharedViewState, shared_view};
///
/// let sv = shared_view();
/// sv.borrow_mut().x_bounds = Some((0.0, 10.0));
/// sv.borrow_mut().y_bounds = Some((-1.0, 1.0));
/// ```
#[derive(Clone, Debug, Default)]
pub struct SharedViewState {
    /// Optional override for x-axis bounds (min, max).
    pub x_bounds: Option<(f64, f64)>,
    /// Optional override for y-axis bounds (min, max).
    pub y_bounds: Option<(f64, f64)>,
}

/// Convenience type for a shared view state (single-threaded).
pub type SharedView = Rc<RefCell<SharedViewState>>;

/// Create a new shared view state.
pub fn shared_view() -> SharedView {
    Rc::new(RefCell::new(SharedViewState::default()))
}
