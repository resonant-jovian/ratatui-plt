//! Choropleth map widget for geographic data visualization.
//!
//! Renders a simplified tile-based map with regions colored by data values,
//! mapped through a colormap. Supports a built-in world region layout and
//! user-defined custom grid layouts.
//!
//! This is not a real geographic projection — it uses rectangular blocks in
//! a terminal-friendly grid, suitable for dashboards and data exploration.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::choropleth::{ChoroplethMap, ChoroplethRegion, MapType};
//!
//! let map = ChoroplethMap::new()
//!     .region(ChoroplethRegion::new("North America", 331.0))
//!     .region(ChoroplethRegion::new("Europe", 447.0))
//!     .region(ChoroplethRegion::new("Asia", 4641.0))
//!     .region(ChoroplethRegion::new("Africa", 1340.0))
//!     .region(ChoroplethRegion::new("South America", 423.0))
//!     .region(ChoroplethRegion::new("Oceania", 43.0))
//!     .map_type(MapType::World)
//!     .title("Population (millions)");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::colormap::{Colorbar, Colormap, Viridis};
use crate::drawing::contrasting_color;
use crate::norm::{LinearNorm, Normalize};
use crate::plot_buffer::{PlotBuffer, Z_CHROME, Z_DATA};
use crate::theme::Theme;

/// A named region with a data value.
#[derive(Clone, Debug)]
pub struct ChoroplethRegion {
    /// Region name (must match a map cell's `region_name`).
    pub name: String,
    /// Data value for this region.
    pub value: f64,
}

impl ChoroplethRegion {
    /// Create a new region.
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// A cell in a custom map layout.
#[derive(Clone, Debug)]
pub struct MapCell {
    /// Region name this cell belongs to.
    pub region_name: String,
    /// Row position in the grid (0-based).
    pub row: u16,
    /// Column position in the grid (0-based).
    pub col: u16,
    /// Width of the cell in grid units.
    pub width: u16,
    /// Height of the cell in grid units.
    pub height: u16,
}

impl MapCell {
    /// Create a new map cell.
    pub fn new(
        region_name: impl Into<String>,
        row: u16,
        col: u16,
        width: u16,
        height: u16,
    ) -> Self {
        Self {
            region_name: region_name.into(),
            row,
            col,
            width,
            height,
        }
    }
}

/// The type of map layout to use.
#[derive(Clone, Debug, Default)]
pub enum MapType {
    /// Simple world regions (continents) using a predefined ASCII layout.
    #[default]
    World,
    /// Custom region layout defined by user-specified cells.
    Custom(Vec<MapCell>),
}

/// A choropleth map widget for data-driven geographic coloring.
///
/// Colors rectangular regions based on data values mapped through a colormap.
/// Supports a built-in world continent layout and custom grid layouts.
pub struct ChoroplethMap {
    regions: Vec<ChoroplethRegion>,
    map_type: MapType,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    title: Option<String>,
    show_colorbar: bool,
    theme: Theme,
}

impl Default for ChoroplethMap {
    fn default() -> Self {
        Self {
            regions: Vec::new(),
            map_type: MapType::World,
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(0.0, 1.0)),
            title: None,
            show_colorbar: true,
            theme: Theme::get_default(),
        }
    }
}

impl ChoroplethMap {
    /// Create a new empty choropleth map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a region with a data value.
    pub fn region(mut self, region: ChoroplethRegion) -> Self {
        self.regions.push(region);
        self
    }

    /// Add multiple regions.
    pub fn regions(mut self, regions: Vec<ChoroplethRegion>) -> Self {
        self.regions.extend(regions);
        self
    }

    /// Set the map type (World or Custom layout).
    pub fn map_type(mut self, map_type: MapType) -> Self {
        self.map_type = map_type;
        self
    }

    /// Set the colormap.
    pub fn colormap(mut self, cmap: impl Colormap + 'static) -> Self {
        self.colormap = Box::new(cmap);
        self
    }

    /// Set the normalization.
    pub fn norm(mut self, norm: impl Normalize + 'static) -> Self {
        self.norm = Box::new(norm);
        self
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Show or hide the colorbar.
    pub fn show_colorbar(mut self, show: bool) -> Self {
        self.show_colorbar = show;
        self
    }

    /// Set the theme.
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }
}

/// The default world map layout: 6 continental regions arranged in a
/// rough geographic pattern using rectangular blocks.
fn world_map_cells() -> Vec<MapCell> {
    // Layout on a 30-column x 12-row grid:
    //
    //  Row 0-3:   Europe (col 12-18)        Asia (col 19-29)
    //  Row 0-4:   North America (col 0-11)
    //  Row 5-8:   Africa (col 12-19)
    //  Row 5-9:   South America (col 3-11)
    //  Row 6-9:                              Oceania (col 22-29)
    vec![
        MapCell::new("North America", 0, 0, 11, 5),
        MapCell::new("Europe", 0, 12, 7, 4),
        MapCell::new("Asia", 0, 20, 10, 5),
        MapCell::new("South America", 5, 3, 8, 5),
        MapCell::new("Africa", 4, 12, 8, 5),
        MapCell::new("Oceania", 6, 22, 8, 4),
    ]
}

impl Widget for &ChoroplethMap {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 15 || area.height < 8 {
            return;
        }

        // Update normalization from region values
        let (vmin, vmax) = region_value_bounds(&self.regions);

        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let colorbar_width: u16 = if self.show_colorbar { 10 } else { 0 };
        let margin: u16 = 1;

        let py = area.y + title_height + margin;
        let ph = area.height.saturating_sub(title_height + margin * 2);
        let px = area.x + margin;
        let pw = area.width.saturating_sub(margin * 2 + colorbar_width);

        if ph < 4 || pw < 10 {
            return;
        }

        let mut pb = PlotBuffer::new(area);

        // Draw title
        if let Some(ref title) = self.title {
            let start = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = start + i as u16;
                if x < area.x + area.width {
                    pb.set_char(x, area.y, ch, self.theme.foreground, Z_CHROME);
                }
            }
        }

        // Get the cells to render
        let cells = match &self.map_type {
            MapType::World => world_map_cells(),
            MapType::Custom(c) => c.clone(),
        };

        if cells.is_empty() {
            return;
        }

        // Compute the grid bounds from the cells
        let grid_cols = cells.iter().map(|c| c.col + c.width).max().unwrap_or(1);
        let grid_rows = cells.iter().map(|c| c.row + c.height).max().unwrap_or(1);

        if grid_cols == 0 || grid_rows == 0 {
            return;
        }

        // Map each grid unit to screen characters
        let cell_w = pw / grid_cols;
        let cell_h = ph / grid_rows;

        if cell_w < 2 || cell_h < 1 {
            return;
        }

        // Recalculate the norm with actual data bounds
        let norm = LinearNorm::new(vmin, vmax);

        // Draw each cell
        for cell in &cells {
            // Find matching region
            let value = self.regions.iter().find_map(|r| {
                if r.name.eq_ignore_ascii_case(&cell.region_name) {
                    Some(r.value)
                } else {
                    None
                }
            });

            // Screen coordinates for this cell
            let sx = px + cell.col * cell_w;
            let sy = py + cell.row * cell_h;
            let sw = cell.width * cell_w;
            let sh = cell.height * cell_h;

            // Determine color
            let (fill_color, has_data) = if let Some(v) = value {
                let t = if vmin < vmax { norm.normalize(v) } else { 0.5 };
                (self.colormap.color_at(t), true)
            } else {
                (self.theme.muted, false)
            };

            // Fill the cell with half-block chars for 2x vertical resolution
            for cy in sy..sy + sh {
                for cx in sx..sx + sw {
                    if cx < area.x + area.width && cy < area.y + area.height {
                        pb.set_cell(
                            cx,
                            cy,
                            self.theme.chars.fill.solid,
                            fill_color,
                            fill_color,
                            Z_DATA,
                        );
                    }
                }
            }

            // Draw border
            for cx in sx..sx + sw {
                if cx < area.x + area.width {
                    if sy < area.y + area.height {
                        pb.set_char(
                            cx,
                            sy,
                            self.theme.chars.border.horizontal,
                            self.theme.axis_color,
                            Z_DATA + 1,
                        );
                    }
                    let bottom = sy + sh.saturating_sub(1);
                    if bottom < area.y + area.height {
                        pb.set_char(
                            cx,
                            bottom,
                            self.theme.chars.border.horizontal,
                            self.theme.axis_color,
                            Z_DATA + 1,
                        );
                    }
                }
            }
            for cy in sy..sy + sh {
                if cy < area.y + area.height {
                    if sx < area.x + area.width {
                        pb.set_char(
                            sx,
                            cy,
                            self.theme.chars.border.vertical,
                            self.theme.axis_color,
                            Z_DATA + 1,
                        );
                    }
                    let right = sx + sw.saturating_sub(1);
                    if right < area.x + area.width {
                        pb.set_char(
                            right,
                            cy,
                            self.theme.chars.border.vertical,
                            self.theme.axis_color,
                            Z_DATA + 1,
                        );
                    }
                }
            }

            // Center the region label inside the cell
            let label = &cell.region_name;
            // Truncate label to fit
            let max_label_len = sw.saturating_sub(2) as usize;
            let display_label: String = if label.len() > max_label_len {
                label.chars().take(max_label_len).collect()
            } else {
                label.clone()
            };

            if !display_label.is_empty() && sh >= 2 {
                let label_y = sy + sh / 2;
                let label_x = sx + (sw.saturating_sub(display_label.len() as u16)) / 2;
                let text_color = if has_data {
                    contrasting_color(fill_color)
                } else {
                    self.theme.foreground
                };
                for (j, ch) in display_label.chars().enumerate() {
                    let x = label_x + j as u16;
                    if x < area.x + area.width && label_y < area.y + area.height {
                        pb.set_char(x, label_y, ch, text_color, Z_CHROME);
                    }
                }
            }

            // If we have data, show the value below the label
            if has_data
                && sh >= 3
                && let Some(v) = value
            {
                let val_str = format_value(v);
                let val_display: String = if val_str.len() > max_label_len {
                    val_str.chars().take(max_label_len).collect()
                } else {
                    val_str
                };
                let val_y = sy + sh / 2 + 1;
                let val_x = sx + (sw.saturating_sub(val_display.len() as u16)) / 2;
                let text_color = contrasting_color(fill_color);
                for (j, ch) in val_display.chars().enumerate() {
                    let x = val_x + j as u16;
                    if x < area.x + area.width && val_y < area.y + area.height {
                        pb.set_char(x, val_y, ch, text_color, Z_CHROME);
                    }
                }
            }
        }

        pb.composite(buf);

        // Draw colorbar
        if self.show_colorbar && !self.regions.is_empty() {
            let cb = Colorbar::new(self.colormap.as_ref(), vmin, vmax)
                .label_color(self.theme.foreground);
            let cb_x = px + pw + 2;
            let cb_w = colorbar_width.min(area.x.saturating_add(area.width).saturating_sub(cb_x));
            let cb_area = Rect::new(cb_x, py, cb_w, ph);
            if cb_area.x + cb_area.width <= area.x + area.width {
                (&cb).render(cb_area, buf);
            }
        }
    }
}

/// Compute the min and max values across all regions.
fn region_value_bounds(regions: &[ChoroplethRegion]) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for r in regions {
        if r.value.is_finite() {
            min = min.min(r.value);
            max = max.max(r.value);
        }
    }
    if min.is_infinite() {
        (0.0, 1.0)
    } else if (max - min).abs() < 1e-15 {
        (min - 1.0, max + 1.0)
    } else {
        (min, max)
    }
}

/// Format a value for display inside a map cell.
fn format_value(v: f64) -> String {
    let abs = v.abs();
    if abs >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.1}K", v / 1_000.0)
    } else if abs >= 10.0 {
        format!("{:.0}", v)
    } else if abs >= 1.0 {
        format!("{:.1}", v)
    } else {
        format!("{:.2}", v)
    }
}
