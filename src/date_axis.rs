//! Date/time axis support for time-series plots.
//!
//! Provides tick locators and formatters that work with `chrono::DateTime<Utc>`
//! for automatic date-aware axis labeling. Requires the `chrono` feature.
//!
//! # Example
//!
//! ```ignore
//! use ratatui_plt::date_axis::{DateLocator, DateFormatter, TimeSeries};
//! use chrono::Utc;
//!
//! let ts = TimeSeries::new("temperature")
//!     .push(Utc::now(), 22.5)
//!     .push(Utc::now(), 23.1);
//! ```

use chrono::{DateTime, Utc};

use crate::series::Series;
use crate::ticker::{TickFormatter, TickLocator};

/// A time series that stores DateTime<Utc> timestamps internally as f64
/// (seconds since Unix epoch) for compatibility with the Series type.
#[derive(Clone, Debug)]
pub struct TimeSeries {
    name: String,
    timestamps: Vec<DateTime<Utc>>,
    values: Vec<f64>,
    color: ratatui::style::Color,
}

impl TimeSeries {
    /// Create a new named time series.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timestamps: Vec::new(),
            values: Vec::new(),
            color: ratatui::style::Color::White,
        }
    }

    /// Add a data point.
    pub fn push(mut self, time: DateTime<Utc>, value: f64) -> Self {
        self.timestamps.push(time);
        self.values.push(value);
        self
    }

    /// Set the color.
    pub fn color(mut self, color: ratatui::style::Color) -> Self {
        self.color = color;
        self
    }

    /// Convert to a standard Series using seconds-since-epoch as x values.
    pub fn to_series(&self) -> Series {
        let data: Vec<(f64, f64)> = self
            .timestamps
            .iter()
            .zip(&self.values)
            .map(|(t, v)| (t.timestamp() as f64, *v))
            .collect();
        Series::new(&self.name).data(data).color(self.color)
    }

    /// Get the time range.
    pub fn time_bounds(&self) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
        if self.timestamps.is_empty() {
            return None;
        }
        let min = self.timestamps.iter().min().copied()?;
        let max = self.timestamps.iter().max().copied()?;
        Some((min, max))
    }
}

/// Convert a f64 (seconds since epoch) back to DateTime<Utc>.
fn epoch_to_datetime(secs: f64) -> DateTime<Utc> {
    DateTime::from_timestamp(secs as i64, 0)
        .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap())
}

/// Date/time tick resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
enum DateResolution {
    Seconds,
    Minutes,
    Hours,
    Days,
    Months,
    Years,
}

/// Automatic date tick locator. Picks appropriate time intervals based on range.
#[derive(Clone, Debug)]
pub struct DateLocator {
    /// Maximum number of ticks.
    pub n_ticks: usize,
}

impl DateLocator {
    pub fn new(n_ticks: usize) -> Self {
        Self { n_ticks }
    }

    fn pick_resolution(range_secs: f64) -> (DateResolution, f64) {
        let range = range_secs.abs();
        if range < 120.0 {
            (DateResolution::Seconds, 10.0)
        } else if range < 7200.0 {
            (DateResolution::Minutes, 60.0)
        } else if range < 172_800.0 {
            (DateResolution::Hours, 3600.0)
        } else if range < 5_184_000.0 {
            (DateResolution::Days, 86400.0)
        } else if range < 63_072_000.0 {
            (DateResolution::Months, 2_592_000.0) // ~30 days
        } else {
            (DateResolution::Years, 31_536_000.0) // ~365 days
        }
    }
}

impl TickLocator for DateLocator {
    fn tick_values(&self, vmin: f64, vmax: f64) -> Vec<f64> {
        if vmin >= vmax || self.n_ticks == 0 {
            return vec![];
        }
        let range = vmax - vmin;
        let (_res, base_step) = Self::pick_resolution(range);

        // Find nice step: multiples of base_step
        let rough_step = range / self.n_ticks as f64;
        let multiplier = (rough_step / base_step).ceil().max(1.0);
        let step = multiplier * base_step;

        let start = (vmin / step).ceil() * step;
        let mut ticks = Vec::new();
        let mut v = start;
        while v <= vmax + step * 0.001 {
            if v >= vmin && v <= vmax {
                ticks.push(v);
            }
            v += step;
        }
        ticks
    }

    fn box_clone(&self) -> Box<dyn TickLocator> {
        Box::new(self.clone())
    }
}

/// Date/time tick formatter using strftime-style format strings.
#[derive(Clone, Debug)]
pub struct DateFormatter {
    /// strftime format string. If None, auto-detected from range.
    pub format: Option<String>,
}

impl DateFormatter {
    /// Create a formatter with automatic format detection.
    pub fn auto() -> Self {
        Self { format: None }
    }

    /// Create a formatter with a specific strftime format.
    pub fn with_format(fmt: impl Into<String>) -> Self {
        Self {
            format: Some(fmt.into()),
        }
    }

    fn auto_format(value: f64) -> String {
        let dt = epoch_to_datetime(value);
        // Heuristic: if time component is midnight, show date only
        if dt.time().hour() == 0 && dt.time().minute() == 0 && dt.time().second() == 0 {
            dt.format("%Y-%m-%d").to_string()
        } else {
            dt.format("%H:%M:%S").to_string()
        }
    }
}

impl TickFormatter for DateFormatter {
    fn format(&self, value: f64) -> String {
        let dt = epoch_to_datetime(value);
        match &self.format {
            Some(fmt) => dt.format(fmt).to_string(),
            None => Self::auto_format(value),
        }
    }

    fn box_clone(&self) -> Box<dyn TickFormatter> {
        Box::new(self.clone())
    }
}

use chrono::Timelike;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_locator() {
        let loc = DateLocator::new(5);
        let ticks = loc.tick_values(0.0, 86400.0); // 1 day range
        assert!(!ticks.is_empty());
        assert!(ticks.len() <= 10);
    }

    #[test]
    fn test_date_formatter_auto() {
        let fmt = DateFormatter::auto();
        let label = fmt.format(0.0); // epoch
        assert!(!label.is_empty());
    }

    #[test]
    fn test_time_series() {
        let ts = TimeSeries::new("test")
            .push(DateTime::from_timestamp(1000, 0).unwrap(), 1.0)
            .push(DateTime::from_timestamp(2000, 0).unwrap(), 2.0);
        let series = ts.to_series();
        assert_eq!(series.data.len(), 2);
        assert_eq!(series.data[0].0, 1000.0);
    }
}
