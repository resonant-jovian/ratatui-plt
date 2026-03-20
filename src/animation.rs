//! Animation framework for time-stepped rendering loops.
//!
//! Provides [`AnimationConfig`] and [`run_animation`] for driving a
//! render callback at a fixed frame interval using tokio's async timer.

use std::time::Duration;

use tokio::time;

/// Configuration for an animation loop.
///
/// # Example
///
/// ```rust,no_run
/// use std::time::Duration;
/// use ratatui_sim::animation::AnimationConfig;
///
/// let config = AnimationConfig::new(Duration::from_millis(16))
///     .frame_count(120); // run for 120 frames (~2 s at 60 fps)
/// ```
#[derive(Clone, Debug)]
pub struct AnimationConfig {
    /// Time between consecutive frames.
    pub frame_interval: Duration,
    /// Total number of frames to render. `None` means run indefinitely.
    pub frame_count: Option<usize>,
}

impl AnimationConfig {
    /// Create a new config with the given frame interval.
    pub fn new(frame_interval: Duration) -> Self {
        Self {
            frame_interval,
            frame_count: None,
        }
    }

    /// Set the total number of frames (builder style).
    pub fn frame_count(mut self, n: usize) -> Self {
        self.frame_count = Some(n);
        self
    }

    /// Create a config targeting approximately `fps` frames per second.
    pub fn from_fps(fps: u32) -> Self {
        let interval = Duration::from_secs_f64(1.0 / fps.max(1) as f64);
        Self::new(interval)
    }
}

impl Default for AnimationConfig {
    fn default() -> Self {
        // 30 fps, run forever
        Self {
            frame_interval: Duration::from_millis(33),
            frame_count: None,
        }
    }
}

/// Run an animation loop, calling `render` on each frame.
///
/// The `render` callback receives the zero-based frame index and returns
/// `true` to continue or `false` to stop early.  The loop also respects
/// [`AnimationConfig::frame_count`] if set.
///
/// # Example
///
/// ```rust,no_run
/// use std::time::Duration;
/// use ratatui_sim::animation::{AnimationConfig, run_animation};
///
/// # async fn example() {
/// let config = AnimationConfig::from_fps(30).frame_count(90);
/// run_animation(config, |frame| {
///     // update data, render widgets, etc.
///     println!("frame {frame}");
///     true // keep going
/// }).await;
/// # }
/// ```
pub async fn run_animation<F>(config: AnimationConfig, mut render: F)
where
    F: FnMut(usize) -> bool,
{
    let mut interval = time::interval(config.frame_interval);
    let limit = config.frame_count.unwrap_or(usize::MAX);

    for frame in 0..limit {
        interval.tick().await;
        if !render(frame) {
            break;
        }
    }
}
