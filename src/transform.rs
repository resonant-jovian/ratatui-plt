//! Coordinate transforms, 3D projection, and camera configuration.
//!
//! Provides utilities for mapping data coordinates to screen coordinates,
//! 3D→2D projection (isometric and perspective), and interactive camera state.

use crate::axis::{AspectRatio, TERMINAL_CELL_ASPECT};

/// 3D camera configuration (immutable, for static views).
///
/// # Example
///
/// ```
/// use ratatui_plt::transform::Camera3D;
///
/// let cam = Camera3D::new()
///     .azimuth(45.0)
///     .elevation(30.0);
/// ```
#[derive(Clone, Debug)]
pub struct Camera3D {
    /// Horizontal rotation in degrees (0 = looking along +x).
    pub azimuth: f64,
    /// Vertical rotation in degrees (0 = horizon, 90 = top-down).
    pub elevation: f64,
    /// Distance from the origin (affects perspective).
    pub distance: f64,
    /// Projection mode.
    pub projection: Projection,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            azimuth: -60.0,
            elevation: 30.0,
            distance: 5.0,
            projection: Projection::Isometric,
        }
    }
}

impl Camera3D {
    /// Create a camera with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the horizontal rotation angle (degrees).
    pub fn azimuth(mut self, deg: f64) -> Self {
        self.azimuth = deg;
        self
    }

    /// Set the vertical elevation angle (degrees).
    pub fn elevation(mut self, deg: f64) -> Self {
        self.elevation = deg;
        self
    }

    /// Set the camera distance.
    pub fn distance(mut self, d: f64) -> Self {
        self.distance = d;
        self
    }

    /// Set the projection mode.
    pub fn projection(mut self, p: Projection) -> Self {
        self.projection = p;
        self
    }

    /// Project a 3D point to 2D screen coordinates.
    ///
    /// Returns (x, y, depth) where depth is used for sorting/cuing.
    pub fn project(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let az = self.azimuth.to_radians();
        let el = self.elevation.to_radians();

        let cos_az = az.cos();
        let sin_az = az.sin();
        let cos_el = el.cos();
        let sin_el = el.sin();

        // Rotate around vertical axis (azimuth)
        let rx = x * cos_az - y * sin_az;
        let ry = x * sin_az + y * cos_az;
        let rz = z;

        // Rotate around horizontal axis (elevation)
        let ry2 = ry * cos_el - rz * sin_el;
        let rz2 = ry * sin_el + rz * cos_el;

        match self.projection {
            Projection::Isometric => (rx, -rz2, ry2),
            Projection::Perspective => {
                let d = self.distance;
                let scale = d / (d + ry2);
                (rx * scale, -rz2 * scale, ry2)
            }
        }
    }
}

/// Mutable camera state for interactive 3D widgets (StatefulWidget).
///
/// # Example
///
/// ```
/// use ratatui_plt::transform::Camera3DState;
///
/// let mut state = Camera3DState::default();
/// state.rotate(5.0, 0.0);  // Rotate 5° horizontally
/// state.zoom(0.9);          // Zoom in slightly
/// ```
#[derive(Clone, Debug)]
pub struct Camera3DState {
    /// Current azimuth angle (degrees).
    pub azimuth: f64,
    /// Current elevation angle (degrees).
    pub elevation: f64,
    /// Current zoom level (1.0 = default).
    pub zoom: f64,
    /// Projection mode.
    pub projection: Projection,
}

impl Default for Camera3DState {
    fn default() -> Self {
        Self {
            azimuth: -60.0,
            elevation: 30.0,
            zoom: 1.0,
            projection: Projection::Isometric,
        }
    }
}

impl Camera3DState {
    /// Rotate the camera by the given delta angles (degrees).
    pub fn rotate(&mut self, d_azimuth: f64, d_elevation: f64) {
        self.azimuth += d_azimuth;
        self.elevation = (self.elevation + d_elevation).clamp(-89.0, 89.0);
    }

    /// Zoom by a factor (< 1.0 zooms in, > 1.0 zooms out).
    pub fn zoom(&mut self, factor: f64) {
        self.zoom = (self.zoom * factor).clamp(0.1, 10.0);
    }

    /// Convert to an immutable Camera3D for projection.
    pub fn to_camera(&self) -> Camera3D {
        Camera3D {
            azimuth: self.azimuth,
            elevation: self.elevation,
            distance: 5.0 / self.zoom,
            projection: self.projection.clone(),
        }
    }
}

/// Projection mode for 3D rendering.
#[derive(Clone, Debug, PartialEq)]
pub enum Projection {
    /// Isometric projection (no perspective distortion).
    Isometric,
    /// Perspective projection (closer objects appear larger).
    Perspective,
}

/// Map a data value to a screen coordinate within a pixel range.
///
/// `data_min` and `data_max` define the data range.
/// `screen_min` and `screen_max` define the screen range (in characters or sub-chars).
pub fn data_to_screen(
    value: f64,
    data_min: f64,
    data_max: f64,
    screen_min: f64,
    screen_max: f64,
) -> f64 {
    if data_max == data_min {
        return (screen_min + screen_max) / 2.0;
    }
    screen_min + (value - data_min) / (data_max - data_min) * (screen_max - screen_min)
}

/// Compute the effective drawing area considering aspect ratio.
///
/// Returns (x_offset, y_offset, width, height) of the actual drawing area
/// within the available area, adjusted for aspect ratio.
pub fn apply_aspect_ratio(
    aspect: &AspectRatio,
    data_x_range: f64,
    data_y_range: f64,
    area_width: u16,
    area_height: u16,
) -> (u16, u16, u16, u16) {
    match aspect {
        AspectRatio::Auto => (0, 0, area_width, area_height),
        AspectRatio::Equal => {
            compute_aspect_area(1.0, data_x_range, data_y_range, area_width, area_height)
        }
        AspectRatio::Fixed(ratio) => {
            compute_aspect_area(*ratio, data_x_range, data_y_range, area_width, area_height)
        }
    }
}

fn compute_aspect_area(
    data_aspect: f64,
    data_x_range: f64,
    data_y_range: f64,
    area_width: u16,
    area_height: u16,
) -> (u16, u16, u16, u16) {
    if data_x_range == 0.0 || data_y_range == 0.0 {
        return (0, 0, area_width, area_height);
    }

    // Data aspect ratio: how many x-units per y-unit
    let data_ratio = (data_x_range / data_y_range) * data_aspect;

    // Screen aspect ratio (accounting for terminal cells being ~2:1)
    let screen_ratio = (area_width as f64 * TERMINAL_CELL_ASPECT) / area_height as f64;

    if screen_ratio > data_ratio {
        // Too wide: shrink width
        let new_width = ((area_height as f64 * data_ratio / TERMINAL_CELL_ASPECT).round() as u16)
            .min(area_width);
        let x_offset = (area_width - new_width) / 2;
        (x_offset, 0, new_width, area_height)
    } else {
        // Too tall: shrink height
        let new_height = ((area_width as f64 * TERMINAL_CELL_ASPECT / data_ratio).round() as u16)
            .min(area_height);
        let y_offset = (area_height - new_height) / 2;
        (0, y_offset, area_width, new_height)
    }
}

/// Convert polar coordinates (r, theta) to cartesian (x, y).
pub fn polar_to_cartesian(r: f64, theta: f64) -> (f64, f64) {
    (r * theta.cos(), r * theta.sin())
}

/// Normalize a value to [0, 1] given a range.
pub fn normalize(value: f64, min: f64, max: f64) -> f64 {
    if max == min {
        0.5
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    }
}

/// Sort indices by depth (back-to-front) for painter's algorithm.
pub fn depth_sort(depths: &[f64]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..depths.len()).collect();
    indices.sort_by(|&a, &b| {
        depths[b]
            .partial_cmp(&depths[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    indices
}
