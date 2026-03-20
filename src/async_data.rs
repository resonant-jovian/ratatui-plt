//! Async data streaming for real-time plot updates.
//!
//! Provides channel-based data sources that integrate with tokio for
//! pushing data from simulation/computation tasks to the rendering loop.

use ratatui::style::Color;
use tokio::sync::watch;

use crate::series::{GridData, Series};
use crate::style::LineStyle;

/// An async-capable data series backed by a tokio watch channel.
///
/// The rendering loop calls [`snapshot`](AsyncSeries::snapshot) each frame to
/// obtain a regular [`Series`] with the latest data, while a background task
/// pushes new data through the paired [`AsyncSeriesSender`].
///
/// # Example
///
/// ```rust,no_run
/// use ratatui_sim::async_data::{async_series, AsyncSeries};
///
/// # async fn example() {
/// let (tx, rx) = async_series("temperature");
/// // Background task pushes data:
/// tx.send(vec![(0.0, 20.0), (1.0, 22.5)]);
/// // Render loop reads snapshots:
/// let series = rx.snapshot();
/// # }
/// ```
pub struct AsyncSeries {
    /// Display name (used in legends).
    name: String,
    /// Line/marker color.
    color: Color,
    /// Line drawing style.
    line_style: LineStyle,
    /// Watch channel receiver carrying the latest data points.
    rx: watch::Receiver<Vec<(f64, f64)>>,
}

/// The sending half of an async series channel.
///
/// Clone-able so multiple producers can push data to the same series.
#[derive(Clone)]
pub struct AsyncSeriesSender {
    tx: watch::Sender<Vec<(f64, f64)>>,
}

/// Create a linked `(AsyncSeriesSender, AsyncSeries)` pair.
///
/// The sender pushes `Vec<(f64, f64)>` snapshots and the receiver converts
/// them into a [`Series`] on demand.
pub fn async_series(name: &str) -> (AsyncSeriesSender, AsyncSeries) {
    let (tx, rx) = watch::channel(Vec::new());
    let sender = AsyncSeriesSender { tx };
    let receiver = AsyncSeries {
        name: name.to_string(),
        color: Color::White,
        line_style: LineStyle::default(),
        rx,
    };
    (sender, receiver)
}

impl AsyncSeriesSender {
    /// Replace the current data with a new snapshot.
    pub fn send(&self, data: Vec<(f64, f64)>) {
        // Ignore the error — it just means the receiver was dropped.
        let _ = self.tx.send(data);
    }

    /// Append points to the existing data.
    pub fn append(&self, points: &[(f64, f64)]) {
        self.tx.send_modify(|data| {
            data.extend_from_slice(points);
        });
    }

    /// Clear all data.
    pub fn clear(&self) {
        let _ = self.tx.send(Vec::new());
    }
}

impl AsyncSeries {
    /// Set the display color (builder style).
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the line style (builder style).
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.line_style = style;
        self
    }

    /// Take a snapshot of the current data as a regular [`Series`].
    ///
    /// This is cheap — it clones the current `Vec` from the watch channel.
    pub fn snapshot(&self) -> Series {
        let data = self.rx.borrow().clone();
        Series::new(&self.name)
            .data(data)
            .color(self.color)
            .line_style(self.line_style.clone())
    }

    /// Wait for the data to change, then return a snapshot.
    ///
    /// This is useful for event-driven rendering where you only want to
    /// re-draw when new data arrives.
    pub async fn changed(&mut self) -> Option<Series> {
        self.rx.changed().await.ok()?;
        Some(self.snapshot())
    }

    /// Return a reference to the current name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

// ---------------------------------------------------------------------------
// AsyncGrid — same pattern for GridData
// ---------------------------------------------------------------------------

/// An async-capable grid data source backed by a tokio watch channel.
///
/// Background tasks push [`GridData`] snapshots through the paired
/// [`AsyncGridSender`]; the rendering loop calls
/// [`snapshot`](AsyncGrid::snapshot) to obtain the latest grid.
pub struct AsyncGrid {
    rx: watch::Receiver<GridData>,
}

/// The sending half of an async grid channel.
#[derive(Clone)]
pub struct AsyncGridSender {
    tx: watch::Sender<GridData>,
}

/// Create a linked `(AsyncGridSender, AsyncGrid)` pair.
///
/// `x` and `y` define the initial (possibly empty) coordinate vectors;
/// the value matrix starts as all zeros with dimensions `y.len() x x.len()`.
pub fn async_grid(x: Vec<f64>, y: Vec<f64>) -> (AsyncGridSender, AsyncGrid) {
    let nrows = y.len();
    let ncols = x.len();
    let values = vec![vec![0.0; ncols]; nrows];
    let initial = GridData::new(x, y, values);
    let (tx, rx) = watch::channel(initial);
    let sender = AsyncGridSender { tx };
    let receiver = AsyncGrid { rx };
    (sender, receiver)
}

impl AsyncGridSender {
    /// Replace the entire grid with new data.
    pub fn send(&self, grid: GridData) {
        let _ = self.tx.send(grid);
    }

    /// Update a single cell in-place.
    pub fn set_cell(&self, row: usize, col: usize, value: f64) {
        self.tx.send_modify(|grid| {
            if row < grid.values.len() && col < grid.values[row].len() {
                grid.values[row][col] = value;
            }
        });
    }

    /// Replace the entire value matrix, keeping existing x/y coordinates.
    pub fn set_values(&self, values: Vec<Vec<f64>>) {
        self.tx.send_modify(|grid| {
            grid.values = values;
        });
    }
}

impl AsyncGrid {
    /// Take a snapshot of the current grid data.
    pub fn snapshot(&self) -> GridData {
        self.rx.borrow().clone()
    }

    /// Wait for the grid data to change, then return a snapshot.
    pub async fn changed(&mut self) -> Option<GridData> {
        self.rx.changed().await.ok()?;
        Some(self.snapshot())
    }
}
