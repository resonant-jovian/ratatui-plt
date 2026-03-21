//! Gantt chart widget for scheduling/timeline visualization.
//!
//! Renders horizontal bar segments for tasks, similar to matplotlib's `broken_barh`.
//! Each task occupies a row with one or more time segments.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::widgets::gantt::{GanttChart, GanttTask};
//! use ratatui::style::Color;
//!
//! let chart = GanttChart::new()
//!     .task(GanttTask::new("Design").segment(0.0, 3.0).color(Color::Cyan))
//!     .task(GanttTask::new("Develop").segment(2.0, 5.0).color(Color::Green))
//!     .task(GanttTask::new("Test").segment(6.0, 2.0).color(Color::Yellow))
//!     .title("Project Timeline");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::annotation::Annotation;
use crate::axis::Axis;
use crate::color_cycle::ColorCycle;
use crate::frame::{DataBounds, PlotFrame, ReferenceLine};
use crate::spines::Spines;
use crate::theme::Theme;
use crate::ticker::NullLocator;
use crate::transform::data_to_screen;

/// A single task in a Gantt chart with one or more time segments.
#[derive(Clone, Debug)]
pub struct GanttTask {
    /// Task label (displayed on the y-axis).
    pub label: String,
    /// Time segments as (start, duration) pairs.
    pub segments: Vec<(f64, f64)>,
    /// Bar color (if None, auto-assigned from color cycle).
    pub color: Option<Color>,
}

impl GanttTask {
    /// Create a new task with the given label and no segments.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            segments: Vec::new(),
            color: None,
        }
    }

    /// Add a time segment (start, duration).
    pub fn segment(mut self, start: f64, duration: f64) -> Self {
        self.segments.push((start, duration));
        self
    }

    /// Set the bar color.
    pub fn color(mut self, c: Color) -> Self {
        self.color = Some(c);
        self
    }
}

/// A Gantt chart widget for timeline/schedule visualization.
///
/// Each task occupies a horizontal row with colored bar segments
/// indicating time periods.
pub struct GanttChart {
    tasks: Vec<GanttTask>,
    x_axis: Axis,
    title: Option<String>,
    show_grid: bool,
    theme: Theme,
    spines: Spines,
    reference_lines: Vec<ReferenceLine>,
    annotations: Vec<Annotation>,
}

impl Default for GanttChart {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            x_axis: Axis::new(),
            title: None,
            show_grid: false,
            theme: Theme::get_default(),
            spines: Spines::default(),
            reference_lines: Vec::new(),
            annotations: Vec::new(),
        }
    }
}

impl GanttChart {
    /// Create a new empty Gantt chart.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a single task.
    pub fn task(mut self, task: GanttTask) -> Self {
        self.tasks.push(task);
        self
    }

    /// Add multiple tasks at once.
    pub fn tasks(mut self, tasks: Vec<GanttTask>) -> Self {
        self.tasks.extend(tasks);
        self
    }

    /// Set the x-axis configuration.
    pub fn x_axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self
    }

    /// Set the chart title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Enable or disable grid lines.
    pub fn show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    /// Set the visual theme.
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

    /// Add an annotation.
    pub fn annotation(mut self, ann: Annotation) -> Self {
        self.annotations.push(ann);
        self
    }
}

impl Widget for &GanttChart {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 || self.tasks.is_empty() {
            return;
        }

        let n_tasks = self.tasks.len();

        // Compute x bounds from all segments
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        for task in &self.tasks {
            for &(start, dur) in &task.segments {
                if start < x_min {
                    x_min = start;
                }
                let end = start + dur;
                if end > x_max {
                    x_max = end;
                }
            }
        }

        // Handle case with no segments
        if x_min > x_max {
            x_min = 0.0;
            x_max = 1.0;
        }

        // Add padding
        let x_range = x_max - x_min;
        let x_pad = if x_range == 0.0 {
            1.0
        } else {
            x_range * 0.05
        };
        let x_lo = x_min - x_pad;
        let x_hi = x_max + x_pad;

        let y_lo = -0.5;
        let y_hi = n_tasks as f64 - 0.5;

        // Use NullLocator for y-axis (task labels drawn manually)
        let y_axis = Axis::new().locator(NullLocator);

        // Enable grid on x-axis if requested
        let x_axis = if self.show_grid {
            self.x_axis.clone().grid(true)
        } else {
            self.x_axis.clone()
        };

        let frame = PlotFrame::new(&x_axis, &y_axis, &self.theme)
            .title(self.title.as_deref())
            .spines(self.spines.clone())
            .reference_lines(&self.reference_lines);

        let Some(pa) = frame.render(
            area,
            buf,
            DataBounds {
                x_lo,
                x_hi,
                y_lo,
                y_hi,
            },
        ) else {
            return;
        };

        let mut cycle = ColorCycle::default();

        for (i, task) in self.tasks.iter().enumerate() {
            let color = task.color.unwrap_or_else(|| cycle.next_color());
            if task.color.is_some() {
                let _ = cycle.next_color();
            }

            // Task row center in data coords: task 0 at y=0, task 1 at y=1, etc.
            // But y-axis is inverted on screen: top of screen = high y
            // We want task 0 at the top, so map task i to y = (n_tasks - 1 - i)
            let task_y = (n_tasks - 1 - i) as f64;
            let bar_half_height = 0.35; // fraction of row height

            let screen_y_top = data_to_screen(
                task_y + bar_half_height,
                y_lo,
                y_hi,
                (pa.y + pa.height - 1) as f64,
                pa.y as f64,
            )
            .round() as u16;
            let screen_y_bot = data_to_screen(
                task_y - bar_half_height,
                y_lo,
                y_hi,
                (pa.y + pa.height - 1) as f64,
                pa.y as f64,
            )
            .round() as u16;

            let y_top = screen_y_top.min(screen_y_bot);
            let y_bot = screen_y_top.max(screen_y_bot);

            for &(start, dur) in &task.segments {
                let screen_x_start = data_to_screen(
                    start,
                    x_lo,
                    x_hi,
                    pa.x as f64,
                    (pa.x + pa.width - 1) as f64,
                )
                .round() as u16;
                let screen_x_end = data_to_screen(
                    start + dur,
                    x_lo,
                    x_hi,
                    pa.x as f64,
                    (pa.x + pa.width - 1) as f64,
                )
                .round() as u16;

                let x_start = screen_x_start.min(screen_x_end);
                let x_end = screen_x_start.max(screen_x_end);

                for x in x_start..=x_end {
                    for y in y_top..=y_bot {
                        if pa.contains(x, y) {
                            buf[(x, y)].set_char('\u{2588}').set_fg(color);
                        }
                    }
                }
            }

            // Draw task label to the left of the plot area
            let label_y = (y_top + y_bot) / 2;
            if label_y >= area.y && label_y < area.y + area.height {
                let label = &task.label;
                let label_end = pa.x.saturating_sub(1);
                let label_len = label.len() as u16;
                let label_start = label_end.saturating_sub(label_len);
                for (j, ch) in label.chars().enumerate() {
                    let lx = label_start + j as u16;
                    if lx >= area.x && lx < label_end {
                        buf[(lx, label_y)]
                            .set_char(ch)
                            .set_fg(self.theme.axis_color);
                    }
                }
            }
        }

        // Draw annotations
        PlotFrame::draw_annotations(&pa, &self.annotations, buf);
    }
}
