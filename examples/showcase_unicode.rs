//! Showcase: Extended Unicode rendering improvements in ratatui-plt.
//!
//! Demonstrates side-by-side comparisons of extended Unicode rendering features
//! using a 2x3 MultiPanel mosaic (ABC / DEF):
//!   A: Marker Gallery — all 27 MarkerShape variants
//!   B: Arrow Styles — four ArrowCharSet variants
//!   C: Border Styles — four BorderStyle variants
//!   D: Histogram with Eighth Blocks — sub-cell vertical precision
//!   E: Enclosed Numbers — circled number annotations
//!   F: Fill Level Demo — 9-level vertical and horizontal fill gradients
//!
//! Requires the `unicode-extended` feature:
//!   cargo run --features unicode-extended --example showcase_unicode

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui_plt::drawing::{HORIZONTAL_FILL_LEVELS, VERTICAL_FILL_LEVELS};
use ratatui_plt::prelude::*;
use ratatui_plt::widgets::histogram::HistDataset;

/// Simple deterministic LCG pseudo-random number generator.
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    (*seed >> 33) as f64 / (1u64 << 31) as f64
}

/// Box-Muller approximation for pseudo-normal values.
fn box_muller(seed: &mut u64, mean: f64, std: f64) -> f64 {
    let u1 = lcg(seed).max(1e-10);
    let u2 = lcg(seed);
    let val = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    val * std + mean
}

fn parse_theme() -> Theme {
    match std::env::args().nth(1).as_deref() {
        Some("dark") => Theme::dark(),
        Some("minimal") => Theme::minimal(),
        Some("publication") => Theme::publication(),
        Some("solarized") => Theme::solarized(),
        Some("light") | None => Theme::light(),
        Some(other) => {
            eprintln!(
                "Unknown theme '{other}'. Available: dark, light, minimal, publication, solarized"
            );
            std::process::exit(1);
        }
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    Theme::set_default(parse_theme());
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let blue_dark = Color::Rgb(31, 119, 180);
    let blue_mid = Color::Rgb(70, 130, 180);
    let blue_light = Color::Rgb(100, 149, 237);

    // ---- Panel A: Marker Gallery — all 27 MarkerShape variants ----
    let markers: Vec<(MarkerShape, &str)> = vec![
        (MarkerShape::Dot, "Dot"),
        (MarkerShape::Cross, "Cross"),
        (MarkerShape::Plus, "Plus"),
        (MarkerShape::Circle, "Circle"),
        (MarkerShape::FilledCircle, "FilledCircle"),
        (MarkerShape::Triangle, "Triangle"),
        (MarkerShape::Square, "Square"),
        (MarkerShape::FilledSquare, "FilledSquare"),
        (MarkerShape::Diamond, "Diamond"),
        (MarkerShape::Star, "Star"),
        (MarkerShape::Braille, "Braille"),
        (MarkerShape::FourPointedStar, "4Star"),
        (MarkerShape::SixPointedStar, "6Star"),
        (MarkerShape::EightPointedStar, "8Star"),
        (MarkerShape::Sparkle, "Sparkle"),
        (MarkerShape::SmallCircle, "SmallCircle"),
        (MarkerShape::Ring, "Ring"),
        (MarkerShape::TriangleDown, "TriDown"),
        (MarkerShape::TriangleRight, "TriRight"),
        (MarkerShape::TriangleLeft, "TriLeft"),
        (MarkerShape::FilledDiamond, "FilledDia"),
        (MarkerShape::CircleHalfLeft, "HalfL"),
        (MarkerShape::CircleHalfRight, "HalfR"),
        (MarkerShape::CircleHalfTop, "HalfT"),
        (MarkerShape::CircleHalfBottom, "HalfB"),
        (MarkerShape::Pentagon, "Pentagon"),
        (MarkerShape::Hexagon, "Hexagon"),
    ];

    // Place markers in a grid: 9 columns x 3 rows
    let cols = 9;
    let mut marker_plot = ScatterPlot::new()
        .title("A: Marker Gallery (27 shapes)")
        .x_axis(Axis::new().bounds(Bounds::Manual(-0.5, cols as f64 - 0.5)))
        .y_axis(Axis::new().bounds(Bounds::Manual(-0.5, 2.5)))
        .show_legend(false);

    for (i, (shape, name)) in markers.iter().enumerate() {
        let col = (i % cols) as f64;
        let row = (i / cols) as f64;
        // Original markers (first 11) in dark blue, new ones in lighter blue
        let color = if i < 11 { blue_dark } else { blue_mid };
        let series = Series::new(*name)
            .data(vec![(col, 2.0 - row)])
            .color(color)
            .marker(*shape);
        marker_plot = marker_plot.series(series);
        // Add label annotation below each marker
        marker_plot = marker_plot.annotation(
            Annotation::new(*name, col, 2.0 - row - 0.3).color(Color::DarkGray),
        );
    }

    // ---- Panel B: Arrow Styles — four ArrowCharSet variants ----
    let arrow_styles = [
        (ArrowCharSet::Standard, "Standard"),
        (ArrowCharSet::Heavy, "Heavy"),
        (ArrowCharSet::Harpoon, "Harpoon"),
        (ArrowCharSet::Double, "Double"),
    ];

    // Build 4 separate VectorField plots; we'll composite them in the mosaic panel
    let mut arrow_fields = Vec::new();
    for (char_set, label) in &arrow_styles {
        let field =
            VectorFieldData::from_fn((-2.0, 2.0), (-2.0, 2.0), 6, 6, |x, y| (-y, x));
        let plot = VectorField::new(field)
            .title(format!("B: Arrows ({label})"))
            .color_by_magnitude(true)
            .colormap(Blues)
            .arrow_scale(2.5)
            .arrow_char_set(char_set.clone())
            .x_axis(Axis::new().grid(true))
            .y_axis(Axis::new().grid(true))
            .aspect_ratio(AspectRatio::Equal);
        arrow_fields.push(plot);
    }

    // ---- Panel C: Border Styles — four BorderStyle variants ----
    // Since LinePlot doesn't expose border_style, we'll render 4 mini
    // line plots using Layout splits inside the mosaic panel callback.
    // Each mini-plot will have a title showing the border style name.
    let sine_data: Vec<(f64, f64)> = (0..100)
        .map(|i| {
            let x = i as f64 * 0.063;
            (x, x.sin())
        })
        .collect();

    // ---- Panel D: Histogram with Eighth Blocks ----
    let mut seed_d = 777u64;
    let hist_data: Vec<f64> = (0..500)
        .map(|_| box_muller(&mut seed_d, 0.0, 1.0))
        .collect();

    let histogram = Histogram::new(vec![])
        .dataset(HistDataset::new("Normal(0,1)", hist_data, blue_dark))
        .bins(30)
        .title("D: Histogram (eighth-block precision)")
        .x_axis(Axis::new().label("Value").grid(true))
        .y_axis(Axis::new().label("Count").grid(true))
        .show_legend(false);

    // ---- Panel E: Enclosed Numbers — annotations with circled digits ----
    let mut seed_e = 2024u64;
    let n_points = 10;
    let scatter_pts: Vec<(f64, f64)> = (0..n_points)
        .map(|_| (lcg(&mut seed_e) * 8.0 + 1.0, lcg(&mut seed_e) * 8.0 + 1.0))
        .collect();
    let scatter_s = Series::new("points")
        .data(scatter_pts.clone())
        .color(blue_dark)
        .marker(MarkerShape::FilledCircle);

    let mut enc_plot = ScatterPlot::new()
        .series(scatter_s)
        .title("E: Enclosed Numbers")
        .x_axis(Axis::new().label("x").grid(true))
        .y_axis(Axis::new().label("y").grid(true))
        .show_legend(false);

    for (i, &(x, y)) in scatter_pts.iter().enumerate() {
        enc_plot = enc_plot.annotation(
            Annotation::new(enclosed_number(i + 1), x + 0.3, y + 0.3).color(blue_mid),
        );
    }

    // ---- Assemble 2x3 mosaic: "ABC\nDEF" ----
    let panel = MultiPanel::from_mosaic("ABC\nDEF")
        .gap(1)
        .suptitle("Unicode Extended Showcase  (q to quit)")
        .mosaic_panel('A', move |area: Rect, buf: &mut Buffer| {
            (&marker_plot).render(area, buf);
        })
        .mosaic_panel('B', move |area: Rect, buf: &mut Buffer| {
            // Show 4 arrow styles in a 2x2 sub-grid
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            let top_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[0]);
            let bot_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[1]);
            let sub_areas = [top_cols[0], top_cols[1], bot_cols[0], bot_cols[1]];
            for (i, sub_area) in sub_areas.iter().enumerate() {
                if i < arrow_fields.len() {
                    (&arrow_fields[i]).render(*sub_area, buf);
                }
            }
        })
        .mosaic_panel('C', {
            let sine = sine_data.clone();
            move |area: Rect, buf: &mut Buffer| {
                // Show 4 border styles in a 2x2 sub-grid
                let rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(area);
                let top_cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(rows[0]);
                let bot_cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(rows[1]);

                let styles: [(BorderStyle, &str); 4] = [
                    (BorderStyle::Single, "C: Single"),
                    (BorderStyle::Rounded, "Rounded"),
                    (BorderStyle::Double, "Double"),
                    (BorderStyle::None, "None"),
                ];
                let sub_areas = [top_cols[0], top_cols[1], bot_cols[0], bot_cols[1]];
                for (i, sub_area) in sub_areas.iter().enumerate() {
                    if i < styles.len() {
                        let (ref style, label) = styles[i];
                        // Manually render a PlotFrame with border_style + a sine wave
                        let theme = Theme::get_default();
                        let x_axis = Axis::new().grid(true);
                        let y_axis = Axis::new().grid(true);
                        let bounds = DataBounds {
                            x_lo: 0.0,
                            x_hi: 6.3,
                            y_lo: -1.2,
                            y_hi: 1.2,
                        };
                        let frame = PlotFrame::new(&x_axis, &y_axis, &theme)
                            .title(Some(label))
                            .border_style(style.clone());
                        if let Some(pa) = frame.render(*sub_area, buf, bounds) {
                            // Draw the sine wave using braille line segments
                            for w in sine.windows(2) {
                                let (x0, y0) = w[0];
                                let (x1, y1) = w[1];
                                let sx0 = pa.screen_x(x0);
                                let sy0 = pa.screen_y(y0);
                                let sx1 = pa.screen_x(x1);
                                let sy1 = pa.screen_y(y1);
                                ratatui_plt::drawing::draw_braille_line(
                                    buf, sx0, sy0, sx1, sy1, blue_dark, &pa,
                                );
                            }
                        }
                    }
                }
            }
        })
        .mosaic_panel('D', move |area: Rect, buf: &mut Buffer| {
            (&histogram).render(area, buf);
        })
        .mosaic_panel('E', move |area: Rect, buf: &mut Buffer| {
            (&enc_plot).render(area, buf);
        })
        .mosaic_panel('F', move |area: Rect, buf: &mut Buffer| {
            // Panel F: Fill Level Demo — 9-level vertical and horizontal gradients
            // We render directly into the buffer since this is a custom demo.
            let theme = Theme::get_default();
            let title = "F: Fill Levels (V + H)";
            let title_start =
                area.x + area.width.saturating_sub(title.len() as u16) / 2;
            for (i, ch) in title.chars().enumerate() {
                let x = title_start + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_fg(theme.foreground);
                }
            }

            let content_y = area.y + 2;
            let content_h = area.height.saturating_sub(4);

            // Vertical fill levels (left half)
            let v_label = "Vertical:";
            for (i, ch) in v_label.chars().enumerate() {
                let x = area.x + 1 + i as u16;
                if x < area.x + area.width && content_y < area.y + area.height {
                    buf[(x, content_y)].set_char(ch).set_fg(theme.foreground);
                }
            }

            let half_w = area.width / 2;
            for (level, &fill_ch) in VERTICAL_FILL_LEVELS.iter().enumerate() {
                let x = area.x + 2 + level as u16 * 2;
                if x >= area.x + half_w {
                    break;
                }
                // Draw bars at varying heights
                for row in 0..content_h.saturating_sub(2) {
                    let y = content_y + 1 + row;
                    if y < area.y + area.height && x < area.x + area.width {
                        if fill_ch == ' ' {
                            buf[(x, y)].set_char('.').set_fg(Color::DarkGray);
                        } else {
                            buf[(x, y)].set_char(fill_ch).set_fg(blue_dark);
                        }
                    }
                }
                // Label below
                let label = format!("{level}");
                let ly = content_y + 1 + content_h.saturating_sub(2);
                if ly < area.y + area.height && x < area.x + area.width {
                    for (ci, ch) in label.chars().enumerate() {
                        let lx = x + ci as u16;
                        if lx < area.x + area.width {
                            buf[(lx, ly)].set_char(ch).set_fg(Color::DarkGray);
                        }
                    }
                }
            }

            // Horizontal fill levels (right half)
            let h_label = "Horizontal:";
            let h_start_x = area.x + half_w + 1;
            for (i, ch) in h_label.chars().enumerate() {
                let x = h_start_x + i as u16;
                if x < area.x + area.width && content_y < area.y + area.height {
                    buf[(x, content_y)].set_char(ch).set_fg(theme.foreground);
                }
            }

            for (level, &fill_ch) in HORIZONTAL_FILL_LEVELS.iter().enumerate() {
                let y = content_y + 1 + level as u16;
                if y >= area.y + area.height {
                    break;
                }
                // Draw a row of the fill character
                let bar_len = (half_w.saturating_sub(6)).min(20);
                for col in 0..bar_len {
                    let x = h_start_x + 1 + col;
                    if x < area.x + area.width {
                        if fill_ch == ' ' {
                            buf[(x, y)].set_char('.').set_fg(Color::DarkGray);
                        } else {
                            buf[(x, y)].set_char(fill_ch).set_fg(blue_light);
                        }
                    }
                }
                // Label after the bar
                let label = format!("{level}/8");
                let lx = h_start_x + 2 + bar_len;
                for (ci, ch) in label.chars().enumerate() {
                    let x = lx + ci as u16;
                    if x < area.x + area.width {
                        buf[(x, y)].set_char(ch).set_fg(Color::DarkGray);
                    }
                }
            }
        });

    loop {
        terminal.draw(|frame| {
            frame.render_widget(&panel, square_area(frame.area()));
        })?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && (key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
        {
            break;
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
