//! Data series types for plot widgets.
//!
//! Provides containers for 2D, 3D, grid, and vector field data
//! that all plot widgets consume.

use ratatui::style::Color;

use crate::style::{LineStyle, MarkerShape};

/// A 2D data series for line plots, scatter plots, and similar widgets.
///
/// # Example
///
/// ```
/// use ratatui_sim::series::Series;
/// use ratatui::style::Color;
///
/// let s = Series::new("Temperature")
///     .data(vec![(0.0, 20.0), (1.0, 22.5), (2.0, 21.0)])
///     .color(Color::Red);
/// ```
#[derive(Clone, Debug)]
pub struct Series {
    /// Display name (used in legends).
    pub name: String,
    /// (x, y) data points.
    pub data: Vec<(f64, f64)>,
    /// Line/marker color.
    pub color: Color,
    /// Line drawing style.
    pub line_style: LineStyle,
    /// Marker shape at data points (`None` for no markers).
    pub marker: Option<MarkerShape>,
    /// Lower error values (y - err_low). Same length as `data` if present.
    pub y_err_low: Option<Vec<f64>>,
    /// Upper error values (y + err_high). Same length as `data` if present.
    pub y_err_high: Option<Vec<f64>>,
    /// Fill to baseline or between series.
    pub fill_to: Option<FillTo>,
}

/// Where to fill from a series.
#[derive(Clone, Debug)]
pub enum FillTo {
    /// Fill down to y = value.
    Baseline(f64),
    /// Fill between this series and another (by index).
    Series(usize),
}

impl Series {
    /// Create a new named series with no data.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: Vec::new(),
            color: Color::White,
            line_style: LineStyle::default(),
            marker: None,
            y_err_low: None,
            y_err_high: None,
            fill_to: None,
        }
    }

    /// Set the (x, y) data points.
    pub fn data(mut self, data: Vec<(f64, f64)>) -> Self {
        self.data = data;
        self
    }

    /// Set the display color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the line style.
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.line_style = style;
        self
    }

    /// Set the marker shape.
    pub fn marker(mut self, shape: MarkerShape) -> Self {
        self.marker = Some(shape);
        self
    }

    /// Set symmetric error bars (±err at each point).
    pub fn y_err(mut self, err: Vec<f64>) -> Self {
        self.y_err_low = Some(err.clone());
        self.y_err_high = Some(err);
        self
    }

    /// Set asymmetric error bars.
    pub fn y_err_asymmetric(mut self, low: Vec<f64>, high: Vec<f64>) -> Self {
        self.y_err_low = Some(low);
        self.y_err_high = Some(high);
        self
    }

    /// Fill to a baseline value.
    pub fn fill_to_baseline(mut self, y: f64) -> Self {
        self.fill_to = Some(FillTo::Baseline(y));
        self
    }

    /// Compute the x-range of the data.
    pub fn x_bounds(&self) -> Option<(f64, f64)> {
        if self.data.is_empty() {
            return None;
        }
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for &(x, _) in &self.data {
            if x < min {
                min = x;
            }
            if x > max {
                max = x;
            }
        }
        Some((min, max))
    }

    /// Compute the y-range of the data (including error bars if present).
    pub fn y_bounds(&self) -> Option<(f64, f64)> {
        if self.data.is_empty() {
            return None;
        }
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for (i, &(_, y)) in self.data.iter().enumerate() {
            let lo = y - self.y_err_low.as_ref().map_or(0.0, |e| e[i]);
            let hi = y + self.y_err_high.as_ref().map_or(0.0, |e| e[i]);
            if lo < min {
                min = lo;
            }
            if hi > max {
                max = hi;
            }
        }
        Some((min, max))
    }
}

/// A 3D data series for surface, wireframe, and scatter3d widgets.
///
/// # Example
///
/// ```
/// use ratatui_sim::series::Series3D;
///
/// let s = Series3D::new("Particles")
///     .data(vec![(1.0, 2.0, 3.0), (4.0, 5.0, 6.0)]);
/// ```
#[derive(Clone, Debug)]
pub struct Series3D {
    /// Display name.
    pub name: String,
    /// (x, y, z) data points.
    pub data: Vec<(f64, f64, f64)>,
    /// Color for rendering.
    pub color: Color,
    /// Optional per-point value for color mapping.
    pub values: Option<Vec<f64>>,
}

impl Series3D {
    /// Create a new named 3D series.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: Vec::new(),
            color: Color::White,
            values: None,
        }
    }

    /// Set the (x, y, z) data points.
    pub fn data(mut self, data: Vec<(f64, f64, f64)>) -> Self {
        self.data = data;
        self
    }

    /// Set the display color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set per-point values for color mapping.
    pub fn values(mut self, values: Vec<f64>) -> Self {
        self.values = Some(values);
        self
    }
}

/// 2D grid data for heatmaps, contour plots, and surface plots.
///
/// Stores a rectangular grid of values with associated x and y coordinate vectors.
///
/// # Example
///
/// ```
/// use ratatui_sim::series::GridData;
///
/// let grid = GridData::new(
///     vec![0.0, 1.0, 2.0],        // x coordinates
///     vec![0.0, 1.0],              // y coordinates
///     vec![vec![1.0, 2.0, 3.0],   // row 0
///          vec![4.0, 5.0, 6.0]],  // row 1
/// );
/// ```
#[derive(Clone, Debug)]
pub struct GridData {
    /// X-axis coordinate values (length = ncols).
    pub x: Vec<f64>,
    /// Y-axis coordinate values (length = nrows).
    pub y: Vec<f64>,
    /// Grid values, indexed as `values[row][col]`.
    pub values: Vec<Vec<f64>>,
}

impl GridData {
    /// Create grid data from coordinate vectors and a value matrix.
    pub fn new(x: Vec<f64>, y: Vec<f64>, values: Vec<Vec<f64>>) -> Self {
        Self { x, y, values }
    }

    /// Create grid data from a function z = f(x, y) sampled on a regular grid.
    pub fn from_fn(
        x_range: (f64, f64),
        y_range: (f64, f64),
        nx: usize,
        ny: usize,
        f: impl Fn(f64, f64) -> f64,
    ) -> Self {
        let x: Vec<f64> = (0..nx)
            .map(|i| x_range.0 + (x_range.1 - x_range.0) * i as f64 / (nx - 1).max(1) as f64)
            .collect();
        let y: Vec<f64> = (0..ny)
            .map(|j| y_range.0 + (y_range.1 - y_range.0) * j as f64 / (ny - 1).max(1) as f64)
            .collect();
        let values: Vec<Vec<f64>> = y
            .iter()
            .map(|&yj| x.iter().map(|&xi| f(xi, yj)).collect())
            .collect();
        Self { x, y, values }
    }

    /// Number of rows.
    pub fn nrows(&self) -> usize {
        self.y.len()
    }

    /// Number of columns.
    pub fn ncols(&self) -> usize {
        self.x.len()
    }

    /// Get the min and max values in the grid.
    pub fn value_bounds(&self) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for row in &self.values {
            for &v in row {
                if v.is_finite() {
                    if v < min {
                        min = v;
                    }
                    if v > max {
                        max = v;
                    }
                }
            }
        }
        (min, max)
    }
}

/// Vector field data for quiver plots.
///
/// Each entry is (x, y, dx, dy) representing a vector (dx, dy) at position (x, y).
#[derive(Clone, Debug)]
pub struct VectorFieldData {
    /// Vector entries: (x, y, dx, dy).
    pub vectors: Vec<(f64, f64, f64, f64)>,
}

impl VectorFieldData {
    /// Create vector field data from a list of (x, y, dx, dy) tuples.
    pub fn new(vectors: Vec<(f64, f64, f64, f64)>) -> Self {
        Self { vectors }
    }

    /// Create from a function (dx, dy) = f(x, y) sampled on a grid.
    pub fn from_fn(
        x_range: (f64, f64),
        y_range: (f64, f64),
        nx: usize,
        ny: usize,
        f: impl Fn(f64, f64) -> (f64, f64),
    ) -> Self {
        let mut vectors = Vec::with_capacity(nx * ny);
        for j in 0..ny {
            let y = y_range.0 + (y_range.1 - y_range.0) * j as f64 / (ny - 1).max(1) as f64;
            for i in 0..nx {
                let x = x_range.0 + (x_range.1 - x_range.0) * i as f64 / (nx - 1).max(1) as f64;
                let (dx, dy) = f(x, y);
                vectors.push((x, y, dx, dy));
            }
        }
        Self { vectors }
    }

    /// Get the maximum vector magnitude.
    pub fn max_magnitude(&self) -> f64 {
        self.vectors
            .iter()
            .map(|&(_, _, dx, dy)| (dx * dx + dy * dy).sqrt())
            .fold(0.0_f64, f64::max)
    }
}
