//! Triangulation data structure for triangle mesh plots.
//!
//! Supports both explicit triangulations (no dependencies) and Delaunay
//! triangulation via the `triangulation` feature (uses `delaunator`).

/// A triangulation of 2D points.
///
/// Stores vertices as (x, y) pairs and triangles as triples of vertex indices.
///
/// # Example
///
/// ```
/// use ratatui_plt::triangulation::Triangulation;
///
/// let tri = Triangulation::from_explicit(
///     vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)],
///     vec![(0, 1, 2)],
/// );
/// assert_eq!(tri.edges().len(), 3);
/// ```
pub struct Triangulation {
    /// Vertex coordinates as (x, y) pairs.
    pub vertices: Vec<(f64, f64)>,
    /// Triangle connectivity as triples of vertex indices.
    pub triangles: Vec<(usize, usize, usize)>,
}

impl Triangulation {
    /// Create a triangulation from explicit vertices and triangle indices.
    ///
    /// Always available (no feature gate required).
    pub fn from_explicit(vertices: Vec<(f64, f64)>, triangles: Vec<(usize, usize, usize)>) -> Self {
        Self {
            vertices,
            triangles,
        }
    }

    /// Compute a Delaunay triangulation from a set of points.
    ///
    /// Requires the `triangulation` feature (uses `delaunator` crate).
    #[cfg(feature = "triangulation")]
    pub fn from_points(points: &[(f64, f64)]) -> Self {
        let coords: Vec<delaunator::Point> = points
            .iter()
            .map(|&(x, y)| delaunator::Point { x, y })
            .collect();

        let triangles = if let Some(result) = delaunator::triangulate(&coords) {
            result
                .triangles
                .chunks(3)
                .map(|tri| (tri[0], tri[1], tri[2]))
                .collect()
        } else {
            Vec::new()
        };

        let vertices = points.to_vec();

        Self {
            vertices,
            triangles,
        }
    }

    /// Return unique edges from the triangulation.
    ///
    /// Each edge is a pair of vertex indices (i, j) where i < j.
    pub fn edges(&self) -> Vec<(usize, usize)> {
        let mut edge_set = std::collections::BTreeSet::new();
        for &(a, b, c) in &self.triangles {
            edge_set.insert((a.min(b), a.max(b)));
            edge_set.insert((b.min(c), b.max(c)));
            edge_set.insert((a.min(c), a.max(c)));
        }
        edge_set.into_iter().collect()
    }

    /// Compute axis-aligned bounding box of vertices: (x_min, x_max, y_min, y_max).
    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for &(x, y) in &self.vertices {
            if x < x_min {
                x_min = x;
            }
            if x > x_max {
                x_max = x;
            }
            if y < y_min {
                y_min = y;
            }
            if y > y_max {
                y_max = y;
            }
        }
        if x_min.is_infinite() {
            (0.0, 1.0, 0.0, 1.0)
        } else {
            (x_min, x_max, y_min, y_max)
        }
    }
}
