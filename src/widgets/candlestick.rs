//! Candlestick (OHLC) chart widget for financial data.
//!
//! Renders Open-High-Low-Close bars commonly used in stock and financial charting.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::candlestick::Candle;
//!
//! let plot = CandlestickChart::new()
//!     .candles(vec![
//!         Candle::new(1.0, 100.0, 110.0, 95.0, 108.0),
//!         Candle::new(2.0, 108.0, 115.0, 105.0, 103.0),
//!         Candle::new(3.0, 103.0, 112.0, 100.0, 110.0),
//!     ])
//!     .title("Stock Price")
//!     .x_axis(Axis::new().label("Time"))
//!     .y_axis(Axis::new().label("Price"));
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::plot_buffer::{PlotBuffer, Z_DATA, Z_FILL, Z_MARKER};
use crate::spines::Spines;
use crate::theme::Theme;

/// A single OHLC candle.
#[derive(Clone, Debug)]
pub struct Candle {
    /// X position (timestamp index or numeric value).
    pub x: f64,
    /// Opening price.
    pub open: f64,
    /// Highest price.
    pub high: f64,
    /// Lowest price.
    pub low: f64,
    /// Closing price.
    pub close: f64,
}

impl Candle {
    /// Create a new candle with the given OHLC values.
    pub fn new(x: f64, open: f64, high: f64, low: f64, close: f64) -> Self {
        Self {
            x,
            open,
            high,
            low,
            close,
        }
    }

    /// Whether this candle is bullish (close >= open).
    pub fn is_bull(&self) -> bool {
        self.close >= self.open
    }
}

/// A candlestick (OHLC) chart widget.
///
/// Draws vertical wicks from low to high and filled bodies from open to close.
/// Bullish candles (close > open) are drawn with `bull_color`, bearish candles
/// with `bear_color`.
#[derive(Clone)]
pub struct CandlestickChart {
    candles: Vec<Candle>,
    x_axis: Axis,
    y_axis: Axis,
    title: Option<String>,
    bull_color: Color,
    bear_color: Color,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for CandlestickChart {
    fn default() -> Self {
        Self {
            candles: Vec::new(),
            x_axis: Axis::new(),
            y_axis: Axis::new(),
            title: None,
            bull_color: Color::Green,
            bear_color: Color::Red,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl CandlestickChart {
    /// Create an empty candlestick chart.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single candle.
    pub fn candle(mut self, c: Candle) -> Self {
        self.candles.push(c);
        self
    }

    /// Set all candles at once.
    pub fn candles(mut self, candles: Vec<Candle>) -> Self {
        self.candles = candles;
        self
    }

    /// Set the X axis.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the Y axis.
    pub fn y_axis(mut self, axis: Axis) -> Self {
        self.y_axis = axis;
        self
    }

    /// Set the plot title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the color for bullish candles (close > open).
    pub fn bull_color(mut self, color: Color) -> Self {
        self.bull_color = color;
        self
    }

    /// Set the color for bearish candles (close < open).
    pub fn bear_color(mut self, color: Color) -> Self {
        self.bear_color = color;
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set spine visibility.
    pub fn spines(mut self, spines: Spines) -> Self {
        self.spines = spines;
        self
    }

    /// Add a reference line.
    pub fn reference_line(mut self, line: ReferenceLine) -> Self {
        self.reference_lines.push(line);
        self
    }

    /// Set all reference lines.
    pub fn reference_lines(mut self, lines: Vec<ReferenceLine>) -> Self {
        self.reference_lines = lines;
        self
    }

    /// Add an annotation.
    pub fn annotation(mut self, ann: Annotation) -> Self {
        self.annotations.push(ann);
        self
    }
}

impl Widget for &CandlestickChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Compute data bounds
        let (data_x_min, data_x_max, data_y_min, data_y_max) = self.compute_bounds();

        let (x_lo, x_hi) = self.x_axis.resolve_bounds(data_x_min, data_x_max);
        let (y_lo, y_hi) = self.y_axis.resolve_bounds(data_y_min, data_y_max);

        let mut pb = PlotBuffer::new(area);

        // Create and render the plot frame
        let frame = PlotFrame::new(&self.x_axis, &self.y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render_to_pb(
            &mut pb,
            area,
            DataBounds {
                x_lo,
                x_hi,
                y_lo,
                y_hi,
            },
        ) else {
            return;
        };

        // Draw each candle
        for candle in &self.candles {
            if !candle.x.is_finite()
                || !candle.open.is_finite()
                || !candle.high.is_finite()
                || !candle.low.is_finite()
                || !candle.close.is_finite()
            {
                continue;
            }

            let sx = pa.screen_x(candle.x).round() as u16;
            if sx < pa.x || sx >= pa.x + pa.width {
                continue;
            }

            let is_bull = candle.is_bull();
            let color = if is_bull {
                self.bull_color
            } else {
                self.bear_color
            };

            // Compute screen positions
            let sy_high = pa.screen_y(candle.high).round() as u16;
            let sy_low = pa.screen_y(candle.low).round() as u16;
            let wick_top = sy_high.min(sy_low);
            let wick_bot = sy_high.max(sy_low);

            let sy_open = pa.screen_y(candle.open).round() as u16;
            let sy_close = pa.screen_y(candle.close).round() as u16;
            let body_top = sy_open.min(sy_close);
            let body_bot = sy_open.max(sy_close).max(body_top);

            // Body extends 1 column on each side of center (3 cols wide)
            let body_left = if sx > pa.x { sx - 1 } else { sx };
            let body_right = if sx + 1 < pa.x + pa.width { sx + 1 } else { sx };

            // 1. Clear the full candle area with a reset background
            for y in wick_top..=wick_bot {
                for x in body_left..=body_right {
                    if pa.contains(x, y) {
                        pb.set_bg(x, y, Color::Reset, Z_FILL);
                    }
                }
            }

            // 2. Draw body (solid filled block, 3 columns wide)
            for y in body_top..=body_bot {
                for x in body_left..=body_right {
                    if pa.contains(x, y) {
                        pb.set_char(x, y, '█', color, Z_DATA);
                    }
                }
            }

            // 3. Draw wicks on center column at Z_MARKER (on top of body fill)
            if wick_top < body_top {
                if pa.contains(sx, wick_top) {
                    pb.set_char(sx, wick_top, '┬', color, Z_MARKER);
                }
                for y in (wick_top + 1)..body_top {
                    if pa.contains(sx, y) {
                        pb.set_char(sx, y, '│', color, Z_MARKER);
                    }
                }
            }
            if wick_bot > body_bot {
                if wick_bot > body_bot + 1 {
                    for y in (body_bot + 1)..wick_bot {
                        if pa.contains(sx, y) {
                            pb.set_char(sx, y, '│', color, Z_MARKER);
                        }
                    }
                }
                if pa.contains(sx, wick_bot) {
                    pb.set_char(sx, wick_bot, '┴', color, Z_MARKER);
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations_pb(&pa, &self.annotations, &mut pb);

        // Composite
        pb.composite(buf);

        frame.draw_end_labels(buf, area, &pa);
    }
}

impl CandlestickChart {
    fn compute_bounds(&self) -> (f64, f64, f64, f64) {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for c in &self.candles {
            if c.x.is_finite() {
                x_min = x_min.min(c.x);
                x_max = x_max.max(c.x);
            }
            if c.low.is_finite() {
                y_min = y_min.min(c.low);
            }
            if c.high.is_finite() {
                y_max = y_max.max(c.high);
            }
        }

        // Add padding around x bounds so candles at edges aren't clipped
        if x_min.is_finite() && x_max.is_finite() {
            let x_pad = if (x_max - x_min).abs() < f64::EPSILON {
                1.0
            } else {
                (x_max - x_min) * 0.05
            };
            x_min -= x_pad;
            x_max += x_pad;
        } else {
            x_min = 0.0;
            x_max = 1.0;
        }

        if !y_min.is_finite() {
            y_min = 0.0;
            y_max = 1.0;
        } else {
            // Small padding on y axis
            let y_pad = (y_max - y_min) * 0.05;
            y_min -= y_pad;
            y_max += y_pad;
        }

        (x_min, x_max, y_min, y_max)
    }
}
