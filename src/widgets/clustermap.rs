//! Clustered heatmap widget combining a heatmap with row/column dendrograms.
//!
//! Similar to seaborn's `clustermap`, this widget displays a heatmap with
//! optional hierarchical clustering dendrograms on the row and column axes.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::prelude::*;
//! use ratatui_plt::widgets::dendrogram::DendroLink;
//! use ratatui_plt::widgets::clustermap::ClusterMap;
//!
//! let data = GridData::new(
//!     vec![0.0, 1.0, 2.0],
//!     vec![0.0, 1.0, 2.0],
//!     vec![
//!         vec![1.0, 0.5, 0.2],
//!         vec![0.5, 1.0, 0.8],
//!         vec![0.2, 0.8, 1.0],
//!     ],
//! );
//!
//! let row_links = vec![
//!     DendroLink::new(0, 1, 0.5),
//!     DendroLink::new(3, 2, 1.2),
//! ];
//!
//! let plot = ClusterMap::new(data)
//!     .row_links(row_links.clone())
//!     .col_links(row_links)
//!     .title("Cluster Map");
//! ```

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::colormap::{Colorbar, Colormap, Viridis};
use crate::norm::{LinearNorm, Normalize};
use crate::series::GridData;
use crate::spines::Spines;
use crate::theme::Theme;
use crate::widgets::dendrogram::DendroLink;

/// A clustered heatmap widget with optional row and column dendrograms.
///
/// Renders a heatmap in the center, with hierarchical clustering trees
/// drawn along the left (row) and top (column) edges. An optional colorbar
/// is shown on the right.
pub struct ClusterMap {
    data: GridData,
    row_links: Option<Vec<DendroLink>>,
    col_links: Option<Vec<DendroLink>>,
    row_order: Option<Vec<usize>>,
    col_order: Option<Vec<usize>>,
    colormap: Box<dyn Colormap>,
    norm: Box<dyn Normalize>,
    title: Option<String>,
    show_values: bool,
    show_colorbar: bool,
    dendrogram_ratio: f64,
    theme: Theme,
    spines: Spines,
}

impl ClusterMap {
    /// Create a new clustered heatmap from grid data.
    pub fn new(data: GridData) -> Self {
        let (vmin, vmax) = data.value_bounds();
        Self {
            data,
            row_links: None,
            col_links: None,
            row_order: None,
            col_order: None,
            colormap: Box::new(Viridis),
            norm: Box::new(LinearNorm::new(vmin, vmax)),
            title: None,
            show_values: false,
            show_colorbar: true,
            dendrogram_ratio: 0.15,
            theme: Theme::get_default(),
            spines: Spines::default(),
        }
    }

    /// Set the row dendrogram linkage.
    pub fn row_links(mut self, links: Vec<DendroLink>) -> Self {
        self.row_links = Some(links);
        self
    }

    /// Set the column dendrogram linkage.
    pub fn col_links(mut self, links: Vec<DendroLink>) -> Self {
        self.col_links = Some(links);
        self
    }

    /// Set the row reordering indices.
    pub fn row_order(mut self, order: Vec<usize>) -> Self {
        self.row_order = Some(order);
        self
    }

    /// Set the column reordering indices.
    pub fn col_order(mut self, order: Vec<usize>) -> Self {
        self.col_order = Some(order);
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

    /// Show or hide cell values.
    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Show or hide the colorbar.
    pub fn show_colorbar(mut self, show: bool) -> Self {
        self.show_colorbar = show;
        self
    }

    /// Set the fraction of space allocated to dendrograms (default 0.15).
    pub fn dendrogram_ratio(mut self, ratio: f64) -> Self {
        self.dendrogram_ratio = ratio;
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
}

/// Reorder grid data by the given row and column permutations.
fn reorder_grid(
    data: &GridData,
    row_order: Option<&[usize]>,
    col_order: Option<&[usize]>,
) -> GridData {
    let nrows = data.nrows();
    let ncols = data.ncols();

    let effective_rows: Vec<usize> = match row_order {
        Some(order) => order.iter().copied().filter(|&i| i < nrows).collect(),
        None => (0..nrows).collect(),
    };
    let effective_cols: Vec<usize> = match col_order {
        Some(order) => order.iter().copied().filter(|&i| i < ncols).collect(),
        None => (0..ncols).collect(),
    };

    let new_y: Vec<f64> = effective_rows
        .iter()
        .filter_map(|&i| data.y.get(i).copied())
        .collect();
    let new_x: Vec<f64> = effective_cols
        .iter()
        .filter_map(|&i| data.x.get(i).copied())
        .collect();

    let new_values: Vec<Vec<f64>> = effective_rows
        .iter()
        .map(|&r| {
            effective_cols
                .iter()
                .map(|&c| {
                    data.values
                        .get(r)
                        .and_then(|row| row.get(c).copied())
                        .unwrap_or(0.0)
                })
                .collect()
        })
        .collect();

    GridData::new(new_x, new_y, new_values)
}

/// Compute positions for each node in the linkage tree.
///
/// Leaves are placed at 0.5, 1.5, ... (centered in cells). Merged clusters
/// are placed at the average of their children.
fn compute_positions(links: &[DendroLink], n_leaves: usize) -> Vec<f64> {
    let total = n_leaves + links.len();
    let mut positions = vec![0.0; total];

    for (i, pos) in positions.iter_mut().enumerate().take(n_leaves) {
        *pos = i as f64 + 0.5;
    }

    for (i, link) in links.iter().enumerate() {
        let left_pos = if link.left < total {
            positions[link.left]
        } else {
            0.0
        };
        let right_pos = if link.right < total {
            positions[link.right]
        } else {
            0.0
        };
        positions[n_leaves + i] = (left_pos + right_pos) / 2.0;
    }

    positions
}

/// Compute the merge height for each node.
fn compute_heights(links: &[DendroLink], n_leaves: usize) -> Vec<f64> {
    let total = n_leaves + links.len();
    let mut heights = vec![0.0; total];

    for (i, link) in links.iter().enumerate() {
        heights[n_leaves + i] = link.distance;
    }

    heights
}

/// Find the maximum merge distance in a set of links.
fn max_distance(links: &[DendroLink]) -> f64 {
    links.iter().map(|l| l.distance).fold(0.0_f64, f64::max)
}

/// Render the heatmap region using half-block characters.
fn render_heatmap(
    buf: &mut Buffer,
    area: Rect,
    data: &GridData,
    norm: &dyn Normalize,
    colormap: &dyn Colormap,
    theme: &Theme,
    show_values: bool,
) {
    let nrows = data.nrows();
    let ncols = data.ncols();
    if nrows == 0 || ncols == 0 || area.width == 0 || area.height == 0 {
        return;
    }

    let aw = area.width;
    let ah = area.height;
    let effective_height = ah as usize * 2;

    for cy in 0..ah {
        for cx in 0..aw {
            let sx = area.x + cx;
            let sy = area.y + cy;
            if sx >= area.x + area.width || sy >= area.y + area.height {
                continue;
            }

            // Top half-pixel
            let top_row_f = (cy as usize * 2) as f64 / effective_height as f64;
            let top_data_row = ((1.0 - top_row_f) * nrows as f64).min((nrows - 1) as f64) as usize;
            let top_data_col =
                (cx as f64 / aw as f64 * ncols as f64).min((ncols - 1) as f64) as usize;
            let top_val = data
                .values
                .get(top_data_row)
                .and_then(|row| row.get(top_data_col).copied())
                .unwrap_or(0.0);
            let top_color = if top_val.is_finite() {
                colormap.color_at(norm.normalize(top_val))
            } else {
                theme.bad_data_color
            };

            // Bottom half-pixel
            let bot_row_f = (cy as usize * 2 + 1) as f64 / effective_height as f64;
            let bot_data_row = ((1.0 - bot_row_f) * nrows as f64).min((nrows - 1) as f64) as usize;
            let bot_val = data
                .values
                .get(bot_data_row)
                .and_then(|row| row.get(top_data_col).copied())
                .unwrap_or(0.0);
            let bot_color = if bot_val.is_finite() {
                colormap.color_at(norm.normalize(bot_val))
            } else {
                theme.bad_data_color
            };

            buf[(sx, sy)]
                .set_char(theme.chars.fill.half_upper)
                .set_fg(top_color)
                .set_bg(bot_color);
        }
    }

    // Render cell values if enabled and cells are wide enough
    if show_values {
        let cell_width = aw as f64 / ncols as f64;
        let cell_height = ah as f64 / nrows as f64;

        if cell_width >= 4.0 && cell_height >= 1.0 {
            for row in 0..nrows {
                for col in 0..ncols {
                    let val = data
                        .values
                        .get(row)
                        .and_then(|r| r.get(col).copied())
                        .unwrap_or(0.0);
                    if !val.is_finite() {
                        continue;
                    }

                    let label = format!("{val:.1}");
                    let center_x = area.x as f64 + (col as f64 + 0.5) * cell_width;
                    let center_y = area.y as f64 + ((nrows - 1 - row) as f64 + 0.5) * cell_height;

                    let xi = center_x.round() as u16;
                    let yi = center_y.round() as u16;

                    let fg_color = match colormap.color_at(norm.normalize(val)) {
                        Color::Rgb(r, g, b) => {
                            let luminance =
                                (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000;
                            if luminance > 128 {
                                Color::Black
                            } else {
                                Color::White
                            }
                        }
                        _ => Color::White,
                    };

                    let label_start = xi.saturating_sub(label.len() as u16 / 2);
                    if yi >= area.y && yi < area.y + ah {
                        for (j, ch) in label.chars().enumerate() {
                            let lx = label_start + j as u16;
                            if lx >= area.x && lx < area.x + aw {
                                buf[(lx, yi)].set_char(ch).set_fg(fg_color);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Render a row dendrogram (tree grows rightward, leaves on the right edge).
fn render_row_dendrogram(
    buf: &mut Buffer,
    area: Rect,
    links: &[DendroLink],
    n_leaves: usize,
    theme: &Theme,
) {
    if area.width < 2 || area.height < 2 || links.is_empty() {
        return;
    }

    let positions = compute_positions(links, n_leaves);
    let heights = compute_heights(links, n_leaves);
    let max_h = max_distance(links);
    if max_h <= 0.0 {
        return;
    }

    let border = &theme.chars.border;
    let color = theme.foreground;

    for link in links {
        let left_pos = positions.get(link.left).copied().unwrap_or(0.0);
        let right_pos = positions.get(link.right).copied().unwrap_or(0.0);
        let left_h = heights.get(link.left).copied().unwrap_or(0.0);
        let right_h = heights.get(link.right).copied().unwrap_or(0.0);
        let merge_h = link.distance;

        // Map category position to screen Y (leaves spread across area height)
        let map_y = |pos: f64| -> u16 {
            let t = pos / n_leaves as f64;
            let sy = area.y as f64 + t * (area.height as f64 - 1.0);
            (sy.round() as u16).clamp(area.y, area.y + area.height - 1)
        };

        // Map distance to screen X (0 = right edge, max = left edge)
        let map_x = |h: f64| -> u16 {
            let t = h / max_h;
            let sx = area.x as f64 + (area.width as f64 - 1.0) * (1.0 - t);
            (sx.round() as u16).clamp(area.x, area.x + area.width - 1)
        };

        let merge_x = map_x(merge_h);
        let left_x = map_x(left_h);
        let right_x = map_x(right_h);
        let left_y = map_y(left_pos);
        let right_y = map_y(right_pos);

        // Horizontal line from left child to merge point
        let x_start = merge_x.min(left_x);
        let x_end = merge_x.max(left_x);
        for x in x_start..=x_end {
            if x >= area.x
                && x < area.x + area.width
                && left_y >= area.y
                && left_y < area.y + area.height
            {
                buf[(x, left_y)].set_char(border.horizontal).set_fg(color);
            }
        }

        // Horizontal line from right child to merge point
        let x_start = merge_x.min(right_x);
        let x_end = merge_x.max(right_x);
        for x in x_start..=x_end {
            if x >= area.x
                && x < area.x + area.width
                && right_y >= area.y
                && right_y < area.y + area.height
            {
                buf[(x, right_y)].set_char(border.horizontal).set_fg(color);
            }
        }

        // Vertical line connecting the two children at the merge x
        let y_top = left_y.min(right_y);
        let y_bot = left_y.max(right_y);
        for y in y_top..=y_bot {
            if merge_x >= area.x
                && merge_x < area.x + area.width
                && y >= area.y
                && y < area.y + area.height
            {
                buf[(merge_x, y)].set_char(border.vertical).set_fg(color);
            }
        }

        // Corner connectors
        if merge_x >= area.x && merge_x < area.x + area.width {
            if y_top >= area.y && y_top < area.y + area.height {
                buf[(merge_x, y_top)]
                    .set_char(border.top_left)
                    .set_fg(color);
            }
            if y_bot >= area.y && y_bot < area.y + area.height {
                buf[(merge_x, y_bot)]
                    .set_char(border.bottom_left)
                    .set_fg(color);
            }
        }
    }
}

/// Render a column dendrogram (tree grows downward, leaves at the bottom).
fn render_col_dendrogram(
    buf: &mut Buffer,
    area: Rect,
    links: &[DendroLink],
    n_leaves: usize,
    theme: &Theme,
) {
    if area.width < 2 || area.height < 2 || links.is_empty() {
        return;
    }

    let positions = compute_positions(links, n_leaves);
    let heights = compute_heights(links, n_leaves);
    let max_h = max_distance(links);
    if max_h <= 0.0 {
        return;
    }

    let border = &theme.chars.border;
    let color = theme.foreground;

    for link in links {
        let left_pos = positions.get(link.left).copied().unwrap_or(0.0);
        let right_pos = positions.get(link.right).copied().unwrap_or(0.0);
        let left_h = heights.get(link.left).copied().unwrap_or(0.0);
        let right_h = heights.get(link.right).copied().unwrap_or(0.0);
        let merge_h = link.distance;

        // Map category position to screen X (leaves spread across area width)
        let map_x = |pos: f64| -> u16 {
            let t = pos / n_leaves as f64;
            let sx = area.x as f64 + t * (area.width as f64 - 1.0);
            (sx.round() as u16).clamp(area.x, area.x + area.width - 1)
        };

        // Map distance to screen Y (0 = bottom edge, max = top edge)
        let map_y = |h: f64| -> u16 {
            let t = h / max_h;
            let sy = area.y as f64 + (area.height as f64 - 1.0) * (1.0 - t);
            (sy.round() as u16).clamp(area.y, area.y + area.height - 1)
        };

        let merge_y = map_y(merge_h);
        let left_y = map_y(left_h);
        let right_y = map_y(right_h);
        let left_x = map_x(left_pos);
        let right_x = map_x(right_pos);

        // Vertical line from left child to merge point
        let y_start = merge_y.min(left_y);
        let y_end = merge_y.max(left_y);
        for y in y_start..=y_end {
            if left_x >= area.x
                && left_x < area.x + area.width
                && y >= area.y
                && y < area.y + area.height
            {
                buf[(left_x, y)].set_char(border.vertical).set_fg(color);
            }
        }

        // Vertical line from right child to merge point
        let y_start = merge_y.min(right_y);
        let y_end = merge_y.max(right_y);
        for y in y_start..=y_end {
            if right_x >= area.x
                && right_x < area.x + area.width
                && y >= area.y
                && y < area.y + area.height
            {
                buf[(right_x, y)].set_char(border.vertical).set_fg(color);
            }
        }

        // Horizontal line connecting the two children at the merge y
        let x_left = left_x.min(right_x);
        let x_right = left_x.max(right_x);
        for x in x_left..=x_right {
            if x >= area.x
                && x < area.x + area.width
                && merge_y >= area.y
                && merge_y < area.y + area.height
            {
                buf[(x, merge_y)].set_char(border.horizontal).set_fg(color);
            }
        }

        // Corner connectors
        if merge_y >= area.y && merge_y < area.y + area.height {
            if x_left >= area.x && x_left < area.x + area.width {
                buf[(x_left, merge_y)]
                    .set_char(border.top_left)
                    .set_fg(color);
            }
            if x_right >= area.x && x_right < area.x + area.width {
                buf[(x_right, merge_y)]
                    .set_char(border.top_right)
                    .set_fg(color);
            }
        }
    }
}

/// Render spines (borders) around the heatmap region.
fn render_spines(buf: &mut Buffer, area: Rect, spines: &Spines, theme: &Theme) {
    let border = &theme.chars.border;
    let color = theme.axis_color;

    if spines.top {
        for x in area.x..area.x + area.width {
            if area.y > 0 {
                let y = area.y.saturating_sub(1);
                if y < buf.area.y + buf.area.height {
                    buf[(x, y)].set_char(border.horizontal).set_fg(color);
                }
            }
        }
    }
    if spines.bottom {
        let y = area.y + area.height;
        if y < buf.area.y + buf.area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_char(border.horizontal).set_fg(color);
            }
        }
    }
    if spines.left && area.x > 0 {
        let x = area.x.saturating_sub(1);
        for y in area.y..area.y + area.height {
            buf[(x, y)].set_char(border.vertical).set_fg(color);
        }
    }
    if spines.right {
        let x = area.x + area.width;
        if x < buf.area.x + buf.area.width {
            for y in area.y..area.y + area.height {
                buf[(x, y)].set_char(border.vertical).set_fg(color);
            }
        }
    }
}

#[cfg(feature = "plotters-render")]
impl crate::plotters_render::PlottersRenderable for ClusterMap {
    fn render_plotters(
        &self,
        area: ratatui::layout::Rect,
        buf: &mut ratatui::buffer::Buffer,
        theme: &crate::theme::Theme,
    ) {
        use crate::plotters_render::{bridge, theme_bridge};
        bridge::render_plotters_to_buf(
            area, buf, theme_bridge::theme_bg_rgb(theme),
            |_root| { /* Minimal stub - full plotters rendering TBD */ },
        );
    }
}

impl Widget for &ClusterMap {
    fn render(self, area: Rect, buf: &mut Buffer) {
        #[cfg(feature = "plotters-render")]
        {
            if crate::plotters_render::should_use_plotters() {
                use crate::plotters_render::PlottersRenderable;
                self.render_plotters(area, buf, &self.theme);
                return;
            }
        }
        if area.width < 8 || area.height < 4 {
            return;
        }

        let nrows = self.data.nrows();
        let ncols = self.data.ncols();
        if nrows == 0 || ncols == 0 {
            return;
        }

        // Reorder data if permutations are provided
        let ordered_data = reorder_grid(
            &self.data,
            self.row_order.as_deref(),
            self.col_order.as_deref(),
        );

        let has_row_dendro = self.row_links.is_some();
        let has_col_dendro = self.col_links.is_some();

        // Reserve space for title
        let title_height: u16 = if self.title.is_some() { 1 } else { 0 };
        let remaining_y = area.y + title_height;
        let remaining_height = area.height.saturating_sub(title_height);
        if remaining_height < 4 {
            return;
        }

        // Compute dendrogram sizes
        let col_dendro_height = if has_col_dendro {
            ((remaining_height as f64 * self.dendrogram_ratio).round() as u16).max(2)
        } else {
            0
        };
        let row_dendro_width = if has_row_dendro {
            ((area.width as f64 * self.dendrogram_ratio).round() as u16).max(3)
        } else {
            0
        };
        let colorbar_width: u16 = if self.show_colorbar { 10 } else { 0 };

        // Compute heatmap area
        let heatmap_x = area.x + row_dendro_width;
        let heatmap_y = remaining_y + col_dendro_height;
        let heatmap_width = area.width.saturating_sub(row_dendro_width + colorbar_width);
        let heatmap_height = remaining_height.saturating_sub(col_dendro_height);

        if heatmap_width < 3 || heatmap_height < 2 {
            return;
        }

        let heatmap_area = Rect::new(heatmap_x, heatmap_y, heatmap_width, heatmap_height);

        // Render title
        if let Some(ref title) = self.title {
            let title_x = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            for (i, ch) in title.chars().enumerate() {
                let tx = title_x + i as u16;
                if tx < area.x + area.width {
                    buf[(tx, area.y)].set_char(ch).set_fg(self.theme.foreground);
                }
            }
        }

        // Render column dendrogram (top region)
        if let Some(ref col_links) = self.col_links
            && !col_links.is_empty()
        {
            let col_dendro_area =
                Rect::new(heatmap_x, remaining_y, heatmap_width, col_dendro_height);
            let effective_ncols = self
                .col_order
                .as_ref()
                .map_or(ncols, |o| o.iter().filter(|&&i| i < ncols).count());
            render_col_dendrogram(
                buf,
                col_dendro_area,
                col_links,
                effective_ncols,
                &self.theme,
            );
        }

        // Render row dendrogram (left region)
        if let Some(ref row_links) = self.row_links
            && !row_links.is_empty()
        {
            let row_dendro_area = Rect::new(area.x, heatmap_y, row_dendro_width, heatmap_height);
            let effective_nrows = self
                .row_order
                .as_ref()
                .map_or(nrows, |o| o.iter().filter(|&&i| i < nrows).count());
            render_row_dendrogram(
                buf,
                row_dendro_area,
                row_links,
                effective_nrows,
                &self.theme,
            );
        }

        // Render the heatmap
        render_heatmap(
            buf,
            heatmap_area,
            &ordered_data,
            self.norm.as_ref(),
            self.colormap.as_ref(),
            &self.theme,
            self.show_values,
        );

        // Render spines around the heatmap
        render_spines(buf, heatmap_area, &self.spines, &self.theme);

        // Render colorbar
        if self.show_colorbar {
            let (vmin, vmax) = self.data.value_bounds();
            let cb = Colorbar::new(self.colormap.as_ref(), vmin, vmax)
                .label_color(self.theme.foreground);
            let cb_x = heatmap_x + heatmap_width + 2;
            if cb_x + colorbar_width <= area.x + area.width {
                let cb_area = Rect::new(
                    cb_x,
                    heatmap_y,
                    colorbar_width.min(area.x + area.width - cb_x),
                    heatmap_height,
                );
                (&cb).render(cb_area, buf);
            }
        }
    }
}
