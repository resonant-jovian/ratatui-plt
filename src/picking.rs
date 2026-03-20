//! Data picking — find the nearest data point to a screen coordinate.
//!
//! Useful for interactive plots where the user moves a cursor/crosshair
//! and wants to see the closest data point.

use crate::frame::PlotArea;
use crate::series::Series;

/// Result of a pick operation.
#[derive(Clone, Debug)]
pub struct PickResult {
    /// Index of the series containing the nearest point.
    pub series_index: usize,
    /// Name of the series.
    pub series_name: String,
    /// Index of the point within the series.
    pub point_index: usize,
    /// X data coordinate of the nearest point.
    pub data_x: f64,
    /// Y data coordinate of the nearest point.
    pub data_y: f64,
    /// Screen-space distance to the nearest point.
    pub distance: f64,
}

/// Find the nearest data point across multiple series to a screen coordinate.
///
/// Returns `None` if all series are empty.
pub fn pick_nearest(
    screen_x: u16,
    screen_y: u16,
    series: &[Series],
    pa: &PlotArea,
) -> Option<PickResult> {
    let sx = screen_x as f64;
    let sy = screen_y as f64;
    let mut best: Option<PickResult> = None;

    for (si, s) in series.iter().enumerate() {
        for (pi, &(dx, dy)) in s.data.iter().enumerate() {
            if !dx.is_finite() || !dy.is_finite() {
                continue;
            }
            let px = pa.screen_x(dx);
            let py = pa.screen_y(dy);
            let dist = ((px - sx).powi(2) + (py - sy).powi(2)).sqrt();
            let dominated = best.as_ref().is_some_and(|b| dist >= b.distance);
            if !dominated {
                best = Some(PickResult {
                    series_index: si,
                    series_name: s.name.clone(),
                    point_index: pi,
                    data_x: dx,
                    data_y: dy,
                    distance: dist,
                });
            }
        }
    }

    best
}
