use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{StatefulWidget, Widget};

use ratatui_plt::prelude::*;
use ratatui_plt::widgets::bar_chart::{BarDataset, BarMode};
use ratatui_plt::widgets::box_plot::BoxData;
use ratatui_plt::widgets::histogram::{BinMethod, HistDataset, HistMode, HistType};
use ratatui_plt::widgets::violin_plot::ViolinData;
use ratatui_plt::{plot, series};

// Helper removed - render inline in each test

#[test]
fn test_series_creation() {
    let s = Series::new("test")
        .data(vec![(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)])
        .color(Color::Red);

    assert_eq!(s.name, "test");
    assert_eq!(s.data.len(), 3);
    assert_eq!(s.x_bounds(), Some((0.0, 2.0)));
    assert_eq!(s.y_bounds(), Some((1.0, 3.0)));
}

#[test]
fn test_series_with_errors() {
    let s = Series::new("err")
        .data(vec![(0.0, 5.0), (1.0, 10.0)])
        .y_err(vec![1.0, 2.0]);

    let (lo, hi) = s.y_bounds().unwrap();
    assert!((lo - 4.0).abs() < 1e-10);
    assert!((hi - 12.0).abs() < 1e-10);
}

#[test]
fn test_grid_data() {
    let g = GridData::from_fn((-1.0, 1.0), (-1.0, 1.0), 10, 10, |x, y| x * y);
    assert_eq!(g.nrows(), 10);
    assert_eq!(g.ncols(), 10);
    let (vmin, vmax) = g.value_bounds();
    assert!(vmin < 0.0);
    assert!(vmax > 0.0);
}

#[test]
fn test_vector_field_data() {
    let v = VectorFieldData::from_fn((-1.0, 1.0), (-1.0, 1.0), 5, 5, |x, y| (-y, x));
    assert_eq!(v.vectors.len(), 25);
    assert!(v.max_magnitude() > 0.0);
}

#[test]
fn test_linear_norm() {
    let n = LinearNorm::new(0.0, 100.0);
    assert!((n.normalize(0.0) - 0.0).abs() < 1e-10);
    assert!((n.normalize(50.0) - 0.5).abs() < 1e-10);
    assert!((n.normalize(100.0) - 1.0).abs() < 1e-10);
    assert!((n.normalize(-10.0) - 0.0).abs() < 1e-10); // clamped
    assert!((n.normalize(110.0) - 1.0).abs() < 1e-10); // clamped
}

#[test]
fn test_log_norm() {
    let n = LogNorm::new(1.0, 1000.0);
    assert!((n.normalize(1.0) - 0.0).abs() < 1e-10);
    assert!((n.normalize(1000.0) - 1.0).abs() < 1e-10);
    let mid = n.normalize(31.623);
    assert!((mid - 0.5).abs() < 0.01);
}

#[test]
fn test_two_slope_norm() {
    let n = TwoSlopeNorm::new(0.0, -10.0, 100.0);
    assert!((n.normalize(-10.0) - 0.0).abs() < 1e-10);
    assert!((n.normalize(0.0) - 0.5).abs() < 1e-10);
    assert!((n.normalize(100.0) - 1.0).abs() < 1e-10);
}

#[test]
fn test_max_n_locator() {
    let loc = MaxNLocator::new(5);
    let ticks = loc.tick_values(0.0, 100.0);
    assert!(!ticks.is_empty());
    assert!(ticks.len() <= 11);
    for &t in &ticks {
        assert!((0.0..=100.0).contains(&t));
    }
}

#[test]
fn test_log_locator() {
    let loc = LogLocator::new(10.0);
    let ticks = loc.tick_values(1.0, 10000.0);
    assert!(ticks.contains(&1.0));
    assert!(ticks.contains(&10.0));
    assert!(ticks.contains(&100.0));
    assert!(ticks.contains(&1000.0));
    assert!(ticks.contains(&10000.0));
}

#[test]
fn test_scalar_formatter() {
    let f = ScalarFormatter;
    assert_eq!(f.format(0.0), "0");
    assert!(f.format(1e7).contains('e'));
}

#[test]
fn test_si_formatter() {
    let f = SiFormatter;
    assert_eq!(f.format(1000.0), "1k");
    assert_eq!(f.format(1000000.0), "1M");
    assert_eq!(f.format(0.001), "1m");
}

#[test]
fn test_colormap_viridis() {
    let cmap = Viridis;
    let c0 = cmap.color_at(0.0);
    let c1 = cmap.color_at(1.0);
    assert!(matches!(c0, Color::Rgb(_, _, _)));
    assert!(matches!(c1, Color::Rgb(_, _, _)));
    // Viridis starts dark, ends bright
    if let (Color::Rgb(r0, g0, b0), Color::Rgb(r1, g1, b1)) = (c0, c1) {
        assert!(r1 as u16 + g1 as u16 + b1 as u16 > r0 as u16 + g0 as u16 + b0 as u16);
    }
}

#[test]
fn test_listed_colormap() {
    let cmap = ListedColormap::new("test", vec![(0.0, Color::Red), (1.0, Color::Blue)]);
    assert_eq!(cmap.name(), "test");
}

#[test]
fn test_mathtext() {
    use ratatui_plt::mathtext::render_mathtext;

    assert_eq!(render_mathtext(r"\alpha"), "\u{03b1}");
    assert_eq!(render_mathtext("x^2"), "x\u{00b2}");
    assert_eq!(render_mathtext("x_0"), "x\u{2080}");
}

#[test]
fn test_scientific_notation() {
    use ratatui_plt::mathtext::scientific_notation;

    let s = scientific_notation(1.5e6);
    assert!(s.contains("10"));
    assert!(s.contains("1.5"));
}

#[test]
fn test_axis_scale_transform() {
    let log = Scale::Log(10.0);
    assert!((log.transform(100.0) - 2.0).abs() < 1e-10);
    assert!((log.inverse(2.0) - 100.0).abs() < 1e-10);

    let linear = Scale::Linear;
    assert!((linear.transform(42.0) - 42.0).abs() < 1e-10);
}

#[test]
fn test_camera3d_projection() {
    let cam = Camera3D::new().azimuth(0.0).elevation(0.0);
    let (sx, sy, depth) = cam.project(1.0, 0.0, 0.0);
    // With azimuth=0, elevation=0: x maps to screen x, y maps to depth
    assert!(sx.is_finite());
    assert!(sy.is_finite());
    assert!(depth.is_finite());
}

#[test]
fn test_camera3d_state() {
    let mut state = Camera3DState::default();
    let initial_az = state.azimuth;
    state.rotate(10.0, 5.0);
    assert!((state.azimuth - initial_az - 10.0).abs() < 1e-10);
    state.zoom(0.5);
    assert!((state.zoom - 0.5).abs() < 1e-10);
}

#[test]
fn test_line_plot_renders() {
    let s = Series::new("test")
        .data(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5)])
        .color(Color::Cyan);
    let plot = LinePlot::new()
        .series(s)
        .title("Test Plot")
        .show_legend(false);

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
    // Just verify it doesn't panic
}

#[test]
fn test_heatmap_renders() {
    let data = GridData::from_fn((-1.0, 1.0), (-1.0, 1.0), 10, 10, |x, y| x + y);
    let hm = Heatmap::new(data)
        .title("Test Heatmap")
        .aspect_ratio(AspectRatio::Equal);

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&hm).render(area, &mut buf);
}

#[test]
fn test_histogram_renders() {
    let data: Vec<f64> = (0..100).map(|i| i as f64 * 0.01).collect();
    let hist = Histogram::new(data).bins(10).title("Test");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&hist).render(area, &mut buf);
}

#[test]
fn test_contour_renders() {
    let data = GridData::from_fn((-1.0, 1.0), (-1.0, 1.0), 10, 10, |x, y| x * x + y * y);
    let contour = ContourPlot::new(data).levels(5).filled(true);

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&contour).render(area, &mut buf);
}

#[test]
fn test_surface3d_renders() {
    let data = GridData::from_fn((-1.0, 1.0), (-1.0, 1.0), 8, 8, |x, y| x + y);
    let surface = Surface3D::new(data).title("Test 3D");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    Widget::render(&surface, area, &mut buf);
}

#[test]
fn test_box_data_quartiles() {
    use ratatui_plt::widgets::box_plot::BoxData;

    let d = BoxData::new("test", vec![1.0, 2.0, 3.0, 4.0, 5.0], Color::White);
    let (q1, median, q3) = d.quartiles();
    assert!((median - 3.0).abs() < 1e-10);
    assert!(q1 < median);
    assert!(q3 > median);
}

#[test]
fn test_aspect_ratio_equal() {
    use ratatui_plt::transform::apply_aspect_ratio;

    let (_x_off, _y_off, w, _h) = apply_aspect_ratio(
        &AspectRatio::Equal,
        10.0,
        10.0, // square data
        80,
        40, // wide terminal area
    );
    // With Equal aspect + 2:1 cell ratio, the effective result should differ from input
    // The function constrains one dimension to maintain aspect ratio
    assert!(w <= 80);
}

#[test]
fn test_series_macro() {
    let s = series!("test", [(0.0, 1.0), (1.0, 2.0)]);
    assert_eq!(s.name, "test");
    assert_eq!(s.data.len(), 2);
}

#[test]
fn test_plot_macro() {
    let s = series!("a", [(0.0, 0.0), (1.0, 1.0)]);
    let p = plot!(s);
    // Just verify it compiles and creates a LinePlot
    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&p).render(area, &mut buf);
}

#[test]
fn test_depth_sort() {
    use ratatui_plt::transform::depth_sort;

    let depths = vec![3.0, 1.0, 2.0];
    let sorted = depth_sort(&depths);
    assert_eq!(sorted[0], 0); // 3.0 is farthest, drawn first
    assert_eq!(sorted[1], 2); // 2.0
    assert_eq!(sorted[2], 1); // 1.0 is closest, drawn last
}

// ===== Part 1A: Missing Render Tests =====

#[test]
fn test_scatter_plot_renders() {
    let s = Series::new("pts")
        .data(vec![
            (0.0, 0.0),
            (1.0, 2.0),
            (2.0, 1.0),
            (3.0, 3.0),
            (4.0, 2.5),
        ])
        .color(Color::Cyan)
        .marker(MarkerShape::FilledCircle);
    let plot = ScatterPlot::new().series(s).title("Scatter");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_bar_chart_grouped_renders() {
    let cats = vec!["A", "B", "C"];
    let ds1 = BarDataset::new("G1", vec![10.0, 20.0, 15.0], Color::Cyan);
    let ds2 = BarDataset::new("G2", vec![12.0, 18.0, 22.0], Color::Yellow);
    let chart = BarChart::new()
        .categories(cats)
        .dataset(ds1)
        .dataset(ds2)
        .mode(BarMode::Grouped)
        .title("Grouped");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&chart).render(area, &mut buf);
}

#[test]
fn test_bar_chart_stacked_renders() {
    let cats = vec!["A", "B", "C"];
    let ds1 = BarDataset::new("G1", vec![10.0, 20.0, 15.0], Color::Cyan);
    let ds2 = BarDataset::new("G2", vec![12.0, 18.0, 22.0], Color::Yellow);
    let chart = BarChart::new()
        .categories(cats)
        .dataset(ds1)
        .dataset(ds2)
        .mode(BarMode::Stacked)
        .title("Stacked");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&chart).render(area, &mut buf);
}

#[test]
fn test_box_plot_renders() {
    let g1 = BoxData::new(
        "A",
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
        Color::Cyan,
    );
    let g2 = BoxData::new(
        "B",
        vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0],
        Color::Yellow,
    );
    let plot = BoxPlot::new().box_data(g1).box_data(g2).title("Box");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_error_bar_plot_renders() {
    let points: Vec<(f64, f64)> = (0..5).map(|i| (i as f64, (i as f64).powi(2))).collect();
    let err_low = vec![0.5, 1.0, 1.5, 2.0, 2.5];
    let err_high = vec![0.5, 1.0, 1.5, 2.0, 2.5];
    let plot = ErrorBarPlot::new()
        .data(points, err_low, err_high)
        .color(Color::Red)
        .title("Error Bars");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_event_plot_renders() {
    let g1 = EventGroup::new("Neuron 1", vec![1.0, 3.0, 5.0, 7.0, 9.0]).color(Color::Cyan);
    let g2 = EventGroup::new("Neuron 2", vec![2.0, 4.0, 6.0, 8.0, 10.0]).color(Color::Yellow);
    let g3 = EventGroup::new("Neuron 3", vec![1.5, 3.5, 5.5, 7.5, 9.5]).color(Color::Magenta);
    let plot = EventPlot::new()
        .group(g1)
        .group(g2)
        .group(g3)
        .title("Events");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_hexbin_plot_renders() {
    let data: Vec<(f64, f64)> = (0..100)
        .map(|i| {
            let t = i as f64 * 0.1;
            (t.sin(), t.cos())
        })
        .collect();
    let plot = HexbinPlot::new(data).gridsize(10).title("Hexbin");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_hist2d_renders() {
    let data: Vec<(f64, f64)> = (0..200)
        .map(|i| {
            let t = i as f64 * 0.05;
            (t.sin(), t.cos())
        })
        .collect();
    let plot = Hist2D::new(data).bins_x(10).bins_y(10).title("Hist2D");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_pie_chart_renders() {
    let plot = PieChart::new()
        .slice(PieSlice::new("A", 30.0).color(Color::Red))
        .slice(PieSlice::new("B", 25.0).color(Color::Blue))
        .slice(PieSlice::new("C", 25.0).color(Color::Green))
        .slice(PieSlice::new("D", 20.0).color(Color::Yellow))
        .donut_ratio(0.4)
        .show_labels(true)
        .title("Pie");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_radial_plot_renders() {
    let data: Vec<(f64, f64)> = (0..=36)
        .map(|i| {
            let theta = i as f64 * std::f64::consts::TAU / 36.0;
            (theta, 1.0)
        })
        .collect();
    let s = Series::new("circle").data(data).color(Color::Cyan);
    let plot = RadialPlot::new().series(s).title("Radial");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_stacked_area_renders() {
    let s1 = Series::new("A")
        .data(
            (0..10)
                .map(|i| (i as f64, (i as f64 * 0.3).sin().abs()))
                .collect(),
        )
        .color(Color::Cyan);
    let s2 = Series::new("B")
        .data(
            (0..10)
                .map(|i| (i as f64, (i as f64 * 0.2).cos().abs()))
                .collect(),
        )
        .color(Color::Yellow);
    let s3 = Series::new("C")
        .data((0..10).map(|i| (i as f64, 0.5)).collect())
        .color(Color::Magenta);
    let plot = StackedArea::new()
        .series(s1)
        .series(s2)
        .series(s3)
        .title("Stacked");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_stem_plot_renders() {
    let data: Vec<(f64, f64)> = (0..10)
        .map(|i| (i as f64, (i as f64 * 0.5).sin()))
        .collect();
    let plot = StemPlot::new(data).color(Color::Green).title("Stem");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_vector_field_renders() {
    let field = VectorFieldData::from_fn((-1.0, 1.0), (-1.0, 1.0), 5, 5, |x, y| (-y, x));
    let plot = VectorField::new(field).title("Vectors");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_scatter3d_renders() {
    let data: Vec<(f64, f64, f64)> = (0..10)
        .map(|i| {
            let t = i as f64 * 0.5;
            (t.cos(), t.sin(), t * 0.1)
        })
        .collect();
    let s = Series3D::new("helix").data(data).color(Color::Cyan);
    let plot = Scatter3D::new().series(s).title("3D Scatter");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    Widget::render(&plot, area, &mut buf);
}

#[test]
fn test_wireframe3d_renders() {
    let data = GridData::from_fn((-1.0, 1.0), (-1.0, 1.0), 5, 5, |x, y| x * x + y * y);
    let plot = Wireframe3D::new(data).title("Wireframe");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    Widget::render(&plot, area, &mut buf);
}

#[test]
fn test_multi_panel_renders() {
    let panel = MultiPanel::new(2, 1)
        .panel(0, 0, |area: Rect, buf: &mut Buffer| {
            let s = Series::new("a")
                .data(vec![(0.0, 0.0), (1.0, 1.0)])
                .color(Color::Cyan);
            let p = LinePlot::new().series(s);
            (&p).render(area, buf);
        })
        .panel(1, 0, |area: Rect, buf: &mut Buffer| {
            let s = Series::new("b")
                .data(vec![(0.0, 1.0), (1.0, 0.0)])
                .color(Color::Yellow);
            let p = LinePlot::new().series(s);
            (&p).render(area, buf);
        });

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&panel).render(area, &mut buf);
}

#[test]
fn test_twin_axes_renders() {
    let primary = Series::new("temp")
        .data(vec![(0.0, 20.0), (6.0, 25.0), (12.0, 30.0), (18.0, 22.0)])
        .color(Color::Red);
    let secondary = Series::new("humidity")
        .data(vec![(0.0, 80.0), (6.0, 60.0), (12.0, 40.0), (18.0, 70.0)])
        .color(Color::Blue);
    let plot = TwinAxes::new()
        .primary(primary)
        .secondary(secondary)
        .title("Twin Axes");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_streamplot_renders() {
    let field = VectorFieldData::from_fn((-1.0, 1.0), (-1.0, 1.0), 5, 5, |x, y| (x, -y));
    let plot = StreamPlot::new(field).density(1).title("Stream");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_violin_plot_renders() {
    let vals1: Vec<f64> = (0..20)
        .map(|i| 5.0 + (i as f64 * 0.3).sin() * 2.0)
        .collect();
    let vals2: Vec<f64> = (0..20)
        .map(|i| 7.0 + (i as f64 * 0.2).cos() * 3.0)
        .collect();
    let d1 = ViolinData::new("A", vals1, Color::Cyan);
    let d2 = ViolinData::new("B", vals2, Color::Yellow);
    let plot = ViolinPlot::new().dataset(d1).dataset(d2).title("Violin");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

// ===== Part 1B: Feature-Focused Tests =====

// --- Normalization ---

#[test]
fn test_symlog_norm() {
    let n = SymLogNorm::new(1.0, -100.0, 100.0);
    let center = n.normalize(0.0);
    assert!((center - 0.5).abs() < 0.01, "center={center}");
    // Near-zero values map close to 0.5
    assert!((n.normalize(0.1) - 0.5).abs() < 0.1);
    // Large values map near extremes
    assert!(n.normalize(100.0) > 0.9);
    assert!(n.normalize(-100.0) < 0.1);
}

#[test]
fn test_power_norm() {
    let n = PowerNorm::new(0.5, 0.0, 1.0);
    // gamma=0.5: normalize(x) = x^0.5, so 0.25^0.5 = 0.5
    let val = n.normalize(0.25);
    assert!((val - 0.5).abs() < 0.01, "val={val}");
    assert!((n.normalize(0.0) - 0.0).abs() < 1e-10);
    assert!((n.normalize(1.0) - 1.0).abs() < 1e-10);
}

#[test]
fn test_boundary_norm() {
    let n = BoundaryNorm::new(vec![0.0, 10.0, 20.0, 50.0, 100.0]);
    // Values within the first bin map to the low end
    let v5 = n.normalize(5.0);
    let v15 = n.normalize(15.0);
    let v75 = n.normalize(75.0);
    // Each boundary region should produce increasing normalized values
    assert!(v5 < v15);
    assert!(v15 < v75);
    assert!(v75 <= 1.0);
}

// --- Tick Locators ---

#[test]
fn test_multiple_locator() {
    let loc = MultipleLocator::new(5.0);
    let ticks = loc.tick_values(0.0, 22.0);
    assert!(ticks.contains(&0.0));
    assert!(ticks.contains(&5.0));
    assert!(ticks.contains(&10.0));
    assert!(ticks.contains(&15.0));
    assert!(ticks.contains(&20.0));
    // 25 is outside range
    assert!(!ticks.contains(&25.0));
}

#[test]
fn test_fixed_locator() {
    let loc = FixedLocator::new(vec![1.0, 3.0, 7.0, 15.0]);
    let ticks = loc.tick_values(0.0, 10.0);
    assert!(ticks.contains(&1.0));
    assert!(ticks.contains(&3.0));
    assert!(ticks.contains(&7.0));
    // 15 is outside [0, 10]
    assert!(!ticks.contains(&15.0));
}

#[test]
fn test_categorical_locator_formatter() {
    let cats = vec!["Mon".to_string(), "Tue".to_string(), "Wed".to_string()];
    let loc = CategoricalLocator::new(cats.clone());
    let ticks = loc.tick_values(0.0, 2.0);
    assert!(ticks.contains(&0.0));
    assert!(ticks.contains(&1.0));
    assert!(ticks.contains(&2.0));

    let fmt = CategoricalFormatter::new(cats);
    assert_eq!(fmt.format(0.0), "Mon");
    assert_eq!(fmt.format(1.0), "Tue");
    assert_eq!(fmt.format(2.0), "Wed");
}

// --- Tick Formatters ---

#[test]
fn test_log_formatter() {
    let f = LogFormatter::new(10.0);
    assert_eq!(f.format(100.0), "10^2");
    assert_eq!(f.format(1000.0), "10^3");
    assert_eq!(f.format(1.0), "10^0");
}

#[test]
fn test_func_formatter() {
    let f = FuncFormatter::new(|v| format!("{:.1}%", v * 100.0));
    assert_eq!(f.format(0.5), "50.0%");
    assert_eq!(f.format(1.0), "100.0%");
}

// --- Scales ---

#[test]
fn test_scale_symlog_transform() {
    let scale = Scale::SymLog {
        lin_thresh: 1.0,
        lin_scale: 1.0,
        base: 10.0,
    };
    // Near zero: linear
    let t_small = scale.transform(0.5);
    assert!(t_small.is_finite());
    // Large: logarithmic
    let t_large = scale.transform(1000.0);
    assert!(t_large > t_small);
    // Symmetric: transform(-x) == -transform(x)
    let t_neg = scale.transform(-1000.0);
    assert!((t_neg + t_large).abs() < 1e-10);
}

#[test]
fn test_scale_power_transform() {
    let scale = Scale::Power(2.0);
    assert!((scale.transform(3.0) - 9.0).abs() < 1e-10);
    assert!((scale.inverse(9.0) - 3.0).abs() < 1e-10);
    assert!((scale.transform(0.0) - 0.0).abs() < 1e-10);
}

// --- Colormaps ---

#[test]
fn test_listed_colormap_interpolation() {
    let cmap = ListedColormap::new(
        "rg",
        vec![
            (0.0, Color::Rgb(255, 0, 0)),
            (0.5, Color::Rgb(0, 255, 0)),
            (1.0, Color::Rgb(0, 0, 255)),
        ],
    );
    let c0 = cmap.color_at(0.0);
    let c1 = cmap.color_at(1.0);
    assert!(matches!(c0, Color::Rgb(255, 0, 0)));
    assert!(matches!(c1, Color::Rgb(0, 0, 255)));
}

#[test]
fn test_colormap_reversed() {
    let fwd = Viridis.color_at(0.0);
    let rev = Reversed::new(Viridis).color_at(1.0);
    // Reversed at 1.0 should equal forward at 0.0
    assert_eq!(fwd, rev);
}

// --- Styles ---

#[test]
fn test_marker_shapes() {
    let shapes = [
        MarkerShape::Dot,
        MarkerShape::Cross,
        MarkerShape::Plus,
        MarkerShape::Circle,
        MarkerShape::FilledCircle,
        MarkerShape::Triangle,
        MarkerShape::Square,
        MarkerShape::FilledSquare,
        MarkerShape::Diamond,
        MarkerShape::Star,
        MarkerShape::Braille,
    ];
    let chars: Vec<char> = shapes.iter().map(|s| s.char()).collect();
    // All chars should be distinct
    let mut unique = chars.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        chars.len(),
        unique.len(),
        "Marker shapes must have distinct chars"
    );
}

#[test]
fn test_line_style_constructors() {
    let solid = LineStyle::solid();
    let dashed = LineStyle::dashed();
    let dotted = LineStyle::dotted();
    // They should produce different patterns
    assert_ne!(
        format!("{:?}", solid.pattern),
        format!("{:?}", dashed.pattern)
    );
    assert_ne!(
        format!("{:?}", dashed.pattern),
        format!("{:?}", dotted.pattern)
    );
}

// --- Data Types ---

#[test]
fn test_series3d_creation() {
    let data = vec![
        (0.0, 0.0, 0.0),
        (1.0, 0.0, 1.0),
        (0.0, 1.0, 1.0),
        (1.0, 1.0, 2.0),
        (0.5, 0.5, 0.5),
    ];
    let s = Series3D::new("test3d")
        .data(data)
        .color(Color::Cyan)
        .values(vec![0.0, 1.0, 1.0, 2.0, 0.5]);
    assert_eq!(s.name, "test3d");
    assert_eq!(s.data.len(), 5);
    assert!(s.values.is_some());
    assert_eq!(s.values.as_ref().unwrap().len(), 5);
}

#[test]
fn test_series_filter_nan() {
    let s = Series::new("nan_test")
        .data(vec![
            (0.0, 1.0),
            (f64::NAN, 2.0),
            (2.0, f64::NAN),
            (3.0, 3.0),
        ])
        .filter_nan();
    assert_eq!(s.data.len(), 2);
    assert_eq!(s.data[0], (0.0, 1.0));
    assert_eq!(s.data[1], (3.0, 3.0));
}

#[test]
fn test_grid_data_from_fn_corners() {
    let g = GridData::from_fn((0.0, 10.0), (0.0, 10.0), 11, 11, |x, y| x + y);
    // Corner (0,0) -> 0+0 = 0
    assert!((g.values[0][0] - 0.0).abs() < 0.1);
    // Corner (10,10) -> 10+10 = 20
    assert!((g.values[10][10] - 20.0).abs() < 0.1);
}

// --- Themes ---

#[test]
fn test_theme_presets() {
    let _dark = Theme::dark();
    let _light = Theme::light();
    let _minimal = Theme::minimal();
    let _publication = Theme::publication();
    let _solarized = Theme::solarized();
    // All construct without panic
}

// --- Edge Cases ---

#[test]
fn test_render_empty_data_graceful() {
    let area = Rect::new(0, 0, 40, 20);

    // Empty LinePlot
    let p = LinePlot::new().title("Empty");
    let mut buf = Buffer::empty(area);
    (&p).render(area, &mut buf);

    // Empty BarChart
    let b = BarChart::new().title("Empty");
    let mut buf = Buffer::empty(area);
    (&b).render(area, &mut buf);

    // Empty PieChart
    let pie = PieChart::new().title("Empty");
    let mut buf = Buffer::empty(area);
    (&pie).render(area, &mut buf);
}

#[test]
fn test_render_small_area_graceful() {
    let area = Rect::new(0, 0, 3, 3);

    let s = Series::new("t")
        .data(vec![(0.0, 0.0), (1.0, 1.0)])
        .color(Color::White);
    let p = LinePlot::new().series(s);
    let mut buf = Buffer::empty(area);
    (&p).render(area, &mut buf);

    let data = GridData::from_fn((-1.0, 1.0), (-1.0, 1.0), 3, 3, |x, y| x + y);
    let hm = Heatmap::new(data);
    let mut buf = Buffer::empty(area);
    (&hm).render(area, &mut buf);
}

// ===== 3D StatefulWidget render tests =====

#[test]
fn test_scatter3d_stateful_renders() {
    let data: Vec<(f64, f64, f64)> = (0..10)
        .map(|i| {
            let t = i as f64 * 0.5;
            (t.cos(), t.sin(), t * 0.1)
        })
        .collect();
    let s = Series3D::new("helix").data(data).color(Color::Cyan);
    let plot = Scatter3D::new().series(s).title("3D Scatter Stateful");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    let mut state = Camera3DState::default();
    StatefulWidget::render(&plot, area, &mut buf, &mut state);
}

#[test]
fn test_wireframe3d_stateful_renders() {
    let data = GridData::from_fn((-1.0, 1.0), (-1.0, 1.0), 5, 5, |x, y| x * x + y * y);
    let plot = Wireframe3D::new(data).title("Wireframe Stateful");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    let mut state = Camera3DState::default();
    StatefulWidget::render(&plot, area, &mut buf, &mut state);
}

// ========================================================================
// Phase 1-8 comprehensive tests
// ========================================================================

// ===== 1. New scales (axis.rs) =====

#[test]
fn test_logit_scale_transform_and_inverse() {
    let logit = Scale::Logit;
    // Logit(0.5) = log(0.5/0.5) = 0
    assert!((logit.transform(0.5) - 0.0).abs() < 1e-10);
    // Logit(0.1) < 0
    assert!(logit.transform(0.1) < 0.0);
    // Logit(0.9) > 0
    assert!(logit.transform(0.9) > 0.0);
    // Roundtrip
    for &p in &[0.1, 0.25, 0.5, 0.75, 0.9] {
        let t = logit.transform(p);
        let back = logit.inverse(t);
        assert!(
            (back - p).abs() < 1e-6,
            "Logit roundtrip failed for p={p}: got {back}"
        );
    }
}

#[test]
fn test_asinh_scale_transform_and_inverse() {
    let asinh = Scale::Asinh { linear_width: 1.0 };
    // asinh(0) = 0
    assert!((asinh.transform(0.0) - 0.0).abs() < 1e-10);
    // Positive values
    let t1 = asinh.transform(10.0);
    assert!(t1 > 0.0);
    // Negative values (symmetric)
    let t_neg = asinh.transform(-10.0);
    assert!((t_neg + t1).abs() < 1e-10);
    // Roundtrip
    for &v in &[-100.0, -1.0, 0.0, 1.0, 100.0] {
        let t = asinh.transform(v);
        let back = asinh.inverse(t);
        assert!(
            (back - v).abs() < 1e-6,
            "Asinh roundtrip failed for v={v}: got {back}"
        );
    }
}

#[test]
fn test_func_scale_with_custom_forward_inverse() {
    fn square(x: f64) -> f64 {
        x * x
    }
    fn sqrt(x: f64) -> f64 {
        x.sqrt()
    }

    let func = Scale::Func {
        forward: square,
        inverse: sqrt,
    };
    assert!((func.transform(3.0) - 9.0).abs() < 1e-10);
    assert!((func.inverse(9.0) - 3.0).abs() < 1e-10);
    // Roundtrip for positive values
    for &v in &[0.5, 1.0, 2.0, 4.0] {
        let t = func.transform(v);
        let back = func.inverse(t);
        assert!(
            (back - v).abs() < 1e-6,
            "Func roundtrip failed for v={v}: got {back}"
        );
    }
}

// ===== 2. New locators and formatters (ticker.rs) =====

#[test]
fn test_null_locator_returns_empty_ticks() {
    let loc = NullLocator;
    let ticks = loc.tick_values(0.0, 100.0);
    assert!(ticks.is_empty());
    let ticks2 = loc.tick_values(-50.0, 50.0);
    assert!(ticks2.is_empty());
}

#[test]
fn test_null_formatter_returns_empty_strings() {
    let fmt = NullFormatter;
    assert_eq!(fmt.format(0.0), "");
    assert_eq!(fmt.format(42.0), "");
    assert_eq!(fmt.format(-100.0), "");
    assert_eq!(fmt.format(f64::MAX), "");
}

#[test]
fn test_percent_formatter_formats_correctly() {
    // scale=100 means input values are in [0, 1]
    let fmt = PercentFormatter::new(100.0);
    assert_eq!(fmt.format(0.0), "0%");
    assert_eq!(fmt.format(0.5), "50%");
    assert_eq!(fmt.format(1.0), "100%");

    // With decimals
    let fmt_dec = PercentFormatter::new(100.0).decimals(1);
    assert_eq!(fmt_dec.format(0.5), "50.0%");
    assert_eq!(fmt_dec.format(0.123), "12.3%");

    // scale=1 means input values are already percentages
    let fmt_raw = PercentFormatter::new(1.0);
    assert_eq!(fmt_raw.format(50.0), "50%");
}

#[test]
fn test_auto_minor_locator_produces_correct_subdivisions() {
    let loc = AutoMinorLocator::new(5);
    let ticks = loc.tick_values(0.0, 100.0);
    // Should produce minor ticks between major ticks
    assert!(!ticks.is_empty());
    // All minor ticks should be within range
    for &t in &ticks {
        assert!(t > 0.0 && t < 100.0, "minor tick {t} outside (0, 100)");
    }

    // With 2 subdivisions
    let loc2 = AutoMinorLocator::new(2);
    let ticks2 = loc2.tick_values(0.0, 10.0);
    assert!(!ticks2.is_empty());
    // Each minor tick should be between consecutive major ticks
    for &t in &ticks2 {
        assert!(t > 0.0 && t < 10.0);
    }
}

// ===== 3. New normalizations (norm.rs) =====

#[test]
fn test_centered_norm_maps_center_to_half() {
    let n = CenteredNorm::new(0.0, 10.0);
    // Center maps to 0.5
    assert!((n.normalize(0.0) - 0.5).abs() < 1e-10);
    // Extremes map to 0 and 1
    assert!((n.normalize(-10.0) - 0.0).abs() < 1e-10);
    assert!((n.normalize(10.0) - 1.0).abs() < 1e-10);
    // Symmetric
    let lo = n.normalize(-5.0);
    let hi = n.normalize(5.0);
    assert!((lo + hi - 1.0).abs() < 1e-10);
}

#[test]
fn test_centered_norm_from_bounds() {
    let n = CenteredNorm::from_bounds(0.0, -20.0, 10.0);
    // halfrange = max(20, 10) = 20
    assert!((n.normalize(0.0) - 0.5).abs() < 1e-10);
    assert!((n.normalize(-20.0) - 0.0).abs() < 1e-10);
    assert!((n.normalize(20.0) - 1.0).abs() < 1e-10);
}

#[test]
fn test_asinh_norm_handles_positive_negative_zero() {
    let n = AsinhNorm::new(1.0, -100.0, 100.0);
    // Zero should map near 0.5 (symmetric range)
    let center = n.normalize(0.0);
    assert!((center - 0.5).abs() < 0.01, "center={center}");
    // Positive values > 0.5
    assert!(n.normalize(50.0) > 0.5);
    // Negative values < 0.5
    assert!(n.normalize(-50.0) < 0.5);
    // Extremes
    assert!(n.normalize(100.0) > 0.9);
    assert!(n.normalize(-100.0) < 0.1);
    // Monotonic
    assert!(n.normalize(-10.0) < n.normalize(0.0));
    assert!(n.normalize(0.0) < n.normalize(10.0));
}

#[test]
fn test_func_norm_with_custom_function() {
    // Square root normalization: map [0, 100] -> [0, 1] via sqrt(v/100)
    let n = FuncNorm::new(|v| (v.max(0.0) / 100.0).sqrt());
    assert!((n.normalize(0.0) - 0.0).abs() < 1e-10);
    assert!((n.normalize(100.0) - 1.0).abs() < 1e-10);
    assert!((n.normalize(25.0) - 0.5).abs() < 1e-10);
    // Clamping: negative input clamped to 0 by our function, then normalize clamps to [0,1]
    assert!((n.normalize(-10.0) - 0.0).abs() < 1e-10);
}

// ===== 4. Colormap registry (colormap.rs) =====

#[test]
fn test_get_colormap_viridis_returns_some() {
    let cmap = get_colormap("viridis");
    assert!(cmap.is_some());
    assert_eq!(cmap.unwrap().name(), "viridis");
}

#[test]
fn test_get_colormap_blues_returns_some() {
    let cmap = get_colormap("blues");
    assert!(cmap.is_some());
    assert_eq!(cmap.unwrap().name(), "Blues");
}

#[test]
fn test_get_colormap_nonexistent_returns_none() {
    assert!(get_colormap("nonexistent").is_none());
    assert!(get_colormap("foobar").is_none());
    assert!(get_colormap("").is_none());
}

#[test]
fn test_all_registered_colormaps_produce_valid_colors() {
    let names = [
        "viridis",
        "plasma",
        "inferno",
        "magma",
        "cividis",
        "hot",
        "spring",
        "summer",
        "autumn",
        "winter",
        "grayscale",
        "blues",
        "greens",
        "reds",
        "oranges",
        "purples",
        "greys",
        "coolwarm",
        "rdbu",
        "seismic",
        "twilight",
        "hsv",
        "jet",
        "turbo",
        "tab20",
        "set1",
        "paired",
        "dark2",
        "accent",
        "piyg",
        "prgn",
        "brbg",
        "puor",
        "rdgy",
        "rdylbu",
        "rdylgn",
        "spectral",
    ];
    for name in &names {
        let cmap = get_colormap(name).unwrap_or_else(|| panic!("colormap '{name}' not found"));
        for &t in &[0.0, 0.5, 1.0] {
            let c = cmap.color_at(t);
            assert!(
                matches!(c, Color::Rgb(_, _, _)),
                "colormap '{name}' at t={t} did not produce Rgb: {:?}",
                c
            );
        }
    }
}

// ===== 5. PlotFrame (frame.rs) =====

#[test]
fn test_plot_area_screen_x_roundtrip() {
    let pa = PlotArea {
        x: 10,
        y: 5,
        width: 60,
        height: 30,
        x_lo: 0.0,
        x_hi: 100.0,
        y_lo: 0.0,
        y_hi: 50.0,
        area: Rect::new(0, 0, 80, 40),
    };

    // screen_x -> data_x_from_screen roundtrip
    for &data_x in &[0.0, 25.0, 50.0, 75.0, 100.0] {
        let sx = pa.screen_x(data_x).round() as u16;
        let recovered = pa.data_x_from_screen(sx);
        assert!(
            (recovered - data_x).abs() < 2.0,
            "x roundtrip failed for data_x={data_x}: got {recovered}"
        );
    }
}

#[test]
fn test_plot_area_screen_y_roundtrip() {
    let pa = PlotArea {
        x: 10,
        y: 5,
        width: 60,
        height: 30,
        x_lo: 0.0,
        x_hi: 100.0,
        y_lo: 0.0,
        y_hi: 50.0,
        area: Rect::new(0, 0, 80, 40),
    };

    for &data_y in &[0.0, 12.5, 25.0, 37.5, 50.0] {
        let sy = pa.screen_y(data_y).round() as u16;
        let recovered = pa.data_y_from_screen(sy);
        assert!(
            (recovered - data_y).abs() < 2.5,
            "y roundtrip failed for data_y={data_y}: got {recovered}"
        );
    }
}

#[test]
fn test_plot_area_contains() {
    let pa = PlotArea {
        x: 10,
        y: 5,
        width: 20,
        height: 10,
        x_lo: 0.0,
        x_hi: 1.0,
        y_lo: 0.0,
        y_hi: 1.0,
        area: Rect::new(0, 0, 40, 20),
    };

    // Inside
    assert!(pa.contains(10, 5));
    assert!(pa.contains(15, 10));
    assert!(pa.contains(29, 14));
    // On boundary (right edge exclusive)
    assert!(!pa.contains(30, 5));
    // On boundary (bottom edge exclusive)
    assert!(!pa.contains(10, 15));
    // Outside
    assert!(!pa.contains(9, 5));
    assert!(!pa.contains(10, 4));
    assert!(!pa.contains(31, 16));
}

#[test]
fn test_plot_area_nearest_point() {
    let pa = PlotArea {
        x: 0,
        y: 0,
        width: 100,
        height: 50,
        x_lo: 0.0,
        x_hi: 10.0,
        y_lo: 0.0,
        y_hi: 10.0,
        area: Rect::new(0, 0, 100, 50),
    };

    let points = vec![(0.0, 0.0), (5.0, 5.0), (10.0, 10.0)];

    // Click near point (5, 5)
    let sx = pa.screen_x(5.0).round() as u16;
    let sy = pa.screen_y(5.0).round() as u16;
    let result = pa.nearest_point(sx, sy, &points);
    assert!(result.is_some());
    let (idx, dx, dy) = result.unwrap();
    assert_eq!(idx, 1);
    assert!((dx - 5.0).abs() < 1e-10);
    assert!((dy - 5.0).abs() < 1e-10);

    // Empty points returns None
    assert!(pa.nearest_point(50, 25, &[]).is_none());
}

#[test]
fn test_reference_line_constructors() {
    let hl = ReferenceLine::hline(0.0, Color::Gray);
    assert!(matches!(hl, ReferenceLine::Horizontal { y, .. } if (y - 0.0).abs() < 1e-10));

    let vl = ReferenceLine::vline(5.0, Color::Red);
    assert!(matches!(vl, ReferenceLine::Vertical { x, .. } if (x - 5.0).abs() < 1e-10));

    let hld = ReferenceLine::hline_dashed(1.0, Color::Blue);
    assert!(
        matches!(hld, ReferenceLine::Horizontal { y, dash: RefLineDash::Dashed, .. } if (y - 1.0).abs() < 1e-10)
    );

    let vld = ReferenceLine::vline_dashed(2.0, Color::Green);
    assert!(
        matches!(vld, ReferenceLine::Vertical { x, dash: RefLineDash::Dashed, .. } if (x - 2.0).abs() < 1e-10)
    );

    let hs = ReferenceLine::hspan(0.5, 1.5, Color::Cyan);
    assert!(
        matches!(hs, ReferenceLine::HorizontalSpan { y1, y2, .. } if (y1 - 0.5).abs() < 1e-10 && (y2 - 1.5).abs() < 1e-10)
    );

    let vs = ReferenceLine::vspan(2.0, 4.0, Color::Magenta);
    assert!(
        matches!(vs, ReferenceLine::VerticalSpan { x1, x2, .. } if (x1 - 2.0).abs() < 1e-10 && (x2 - 4.0).abs() < 1e-10)
    );
}

// ===== 6. Export (export.rs) integration-level tests =====

#[test]
fn test_export_render_to_buffer_correct_size() {
    use ratatui_plt::export::render_to_buffer;

    let s = Series::new("s")
        .data(vec![(0.0, 0.0), (1.0, 1.0)])
        .color(Color::Cyan);
    let plot = LinePlot::new().series(s).title("Export Test");
    let buf = render_to_buffer(&plot, 80, 24);

    assert_eq!(buf.area.width, 80);
    assert_eq!(buf.area.height, 24);
    assert_eq!(buf.content.len(), 80 * 24);
}

#[test]
fn test_export_buffer_to_text_non_empty() {
    use ratatui_plt::export::{buffer_to_text, render_to_buffer};

    let s = Series::new("s")
        .data(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5)])
        .color(Color::Red);
    let plot = LinePlot::new().series(s).title("Text Export");
    let buf = render_to_buffer(&plot, 60, 20);
    let text = buffer_to_text(&buf);

    assert!(!text.is_empty());
    // Should contain the title
    assert!(text.contains("Text Export"));
}

#[test]
fn test_export_buffer_to_svg_valid_start_tag() {
    use ratatui_plt::export::{buffer_to_svg, render_to_buffer};

    let s = Series::new("s")
        .data(vec![(0.0, 0.0), (1.0, 1.0)])
        .color(Color::Green);
    let plot = LinePlot::new().series(s).title("SVG Export");
    let buf = render_to_buffer(&plot, 40, 12);
    let svg = buffer_to_svg(&buf, 14.0);

    assert!(svg.starts_with("<svg"), "SVG should start with <svg tag");
    assert!(svg.contains("</svg>"), "SVG should have closing </svg> tag");
    assert!(svg.contains("xmlns"), "SVG should contain xmlns attribute");
}

// ===== 7. New widgets (render-without-panic tests) =====

#[test]
fn test_ecdf_plot_renders() {
    let plot = EcdfPlot::new()
        .dataset(EcdfDataset::new(
            "Sample A",
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
            Color::Cyan,
        ))
        .dataset(EcdfDataset::new(
            "Sample B",
            vec![2.0, 3.0, 4.0, 5.0, 8.0],
            Color::Yellow,
        ))
        .title("ECDF");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_stairs_plot_renders() {
    use ratatui_plt::widgets::stairs::StairsDataset;

    let plot = StairsPlot::new()
        .dataset(StairsDataset::new(
            "Hist",
            vec![0.0, 1.0, 2.0, 3.0, 4.0],
            vec![5.0, 12.0, 8.0, 3.0],
            Color::Cyan,
        ))
        .title("Stairs");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_rug_plot_renders() {
    let plot = RugPlot::new()
        .dataset(RugDataset::new(
            "Obs",
            vec![1.0, 2.5, 3.0, 4.2, 5.5],
            Color::Cyan,
        ))
        .title("Rug");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_strip_plot_renders() {
    let plot = StripPlot::new()
        .group(StripGroup::new("A", vec![1.0, 2.0, 3.0, 4.0], Color::Cyan))
        .group(StripGroup::new(
            "B",
            vec![2.0, 3.0, 5.0, 7.0],
            Color::Yellow,
        ))
        .jitter(0.0)
        .title("Strip");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_band_plot_renders() {
    let x: Vec<f64> = (0..20).map(|i| i as f64 * 0.25).collect();
    let y_lower: Vec<f64> = x.iter().map(|&v| v.sin() - 0.3).collect();
    let y_upper: Vec<f64> = x.iter().map(|&v| v.sin() + 0.3).collect();

    let plot = BandPlot::new()
        .band(Band::new("confidence", x, y_lower, y_upper).color(Color::Cyan))
        .title("Band");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_candlestick_chart_renders() {
    let plot = CandlestickChart::new()
        .candles(vec![
            Candle::new(1.0, 100.0, 110.0, 95.0, 108.0),
            Candle::new(2.0, 108.0, 115.0, 105.0, 103.0),
            Candle::new(3.0, 103.0, 112.0, 100.0, 110.0),
            Candle::new(4.0, 110.0, 118.0, 107.0, 115.0),
            Candle::new(5.0, 115.0, 120.0, 110.0, 108.0),
        ])
        .title("OHLC");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_parallel_coords_renders() {
    use ratatui_plt::widgets::parallel_coords::{ParallelAxis, ParallelRecord};

    let plot = ParallelCoords::new()
        .axes(vec![
            ParallelAxis::new("Speed", 0.0, 100.0),
            ParallelAxis::new("Power", 0.0, 500.0),
            ParallelAxis::new("Weight", 1000.0, 3000.0),
        ])
        .record(
            ParallelRecord::new(vec![60.0, 300.0, 1500.0])
                .color(Color::Cyan)
                .name("Car A"),
        )
        .record(
            ParallelRecord::new(vec![80.0, 450.0, 2000.0])
                .color(Color::Red)
                .name("Car B"),
        )
        .title("Parallel Coords");

    let area = Rect::new(0, 0, 60, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_bar3d_renders_with_camera_state() {
    use ratatui_plt::widgets::bar3d::{Bar3D, Bar3DData};

    let bars = vec![
        Bar3DData::new(0.0, 0.0, 3.0).color(Color::Cyan),
        Bar3DData::new(1.0, 0.0, 5.0).color(Color::Yellow),
        Bar3DData::new(2.0, 0.0, 2.0).color(Color::Green),
        Bar3DData::new(0.0, 1.0, 4.0).color(Color::Red),
    ];
    let plot = Bar3D::new(bars).title("3D Bars");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    let mut state = Camera3DState::default();
    StatefulWidget::render(&plot, area, &mut buf, &mut state);
}

#[test]
fn test_swarm_plot_renders() {
    use ratatui_plt::widgets::swarm::SwarmGroup;

    let plot = SwarmPlot::new()
        .group(SwarmGroup::new(
            "A",
            vec![1.0, 1.1, 1.2, 2.0, 3.0, 3.1, 4.0],
            Color::Cyan,
        ))
        .group(SwarmGroup::new(
            "B",
            vec![2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0],
            Color::Yellow,
        ))
        .title("Swarm");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_boxen_plot_renders() {
    use ratatui_plt::widgets::boxen::BoxenGroup;

    let data_a: Vec<f64> = (0..50)
        .map(|i| 5.0 + (i as f64 * 0.1).sin() * 3.0)
        .collect();
    let data_b: Vec<f64> = (0..50)
        .map(|i| 8.0 + (i as f64 * 0.15).cos() * 2.0)
        .collect();

    let plot = BoxenPlot::new()
        .group(BoxenGroup::new("A", data_a, Color::Cyan))
        .group(BoxenGroup::new("B", data_b, Color::Yellow))
        .title("Boxen");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_quiver3d_renders_with_camera_state() {
    use ratatui_plt::widgets::quiver3d::Arrow3D;

    let arrows: Vec<Arrow3D> = (0..3)
        .flat_map(|i| {
            (0..3).map(move |j| {
                let x = i as f64 - 1.0;
                let y = j as f64 - 1.0;
                Arrow3D::new(x, y, 0.0, -y * 0.3, x * 0.3, 0.1)
            })
        })
        .collect();

    let plot = Quiver3D::new(arrows).title("3D Quiver");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    let mut state = Camera3DState::default();
    StatefulWidget::render(&plot, area, &mut buf, &mut state);
}

#[test]
fn test_contour3d_renders_with_camera_state() {
    let data = GridData::from_fn((-2.0, 2.0), (-2.0, 2.0), 10, 10, |x, y| {
        (-(x * x + y * y) / 2.0).exp()
    });
    let plot = Contour3D::new(data).levels(5).title("3D Contour");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    let mut state = Camera3DState::default();
    StatefulWidget::render(&plot, area, &mut buf, &mut state);
}

#[test]
fn test_dendrogram_renders() {
    let links = vec![
        DendroLink::new(0, 1, 1.0),
        DendroLink::new(2, 3, 1.5),
        DendroLink::new(4, 5, 3.0),
    ];
    let labels = vec!["A".into(), "B".into(), "C".into(), "D".into()];
    let plot = Dendrogram::new(links, labels).title("Clustering");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_pcolormesh_renders() {
    let nr = 5;
    let nt = 8;
    let mut x = vec![vec![0.0; nt + 1]; nr + 1];
    let mut y = vec![vec![0.0; nt + 1]; nr + 1];
    let mut values = vec![vec![0.0; nt]; nr];

    for i in 0..=nr {
        let r = i as f64 / nr as f64;
        for j in 0..=nt {
            let theta = 2.0 * std::f64::consts::PI * j as f64 / nt as f64;
            x[i][j] = r * theta.cos();
            y[i][j] = r * theta.sin();
        }
    }
    for i in 0..nr {
        for j in 0..nt {
            values[i][j] = (i as f64 + j as f64) / (nr + nt) as f64;
        }
    }

    let plot = Pcolormesh::new(x, y, values).title("Polar Mesh");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_crosshair_overlay_renders() {
    // First render a plot to get a PlotArea
    let s = Series::new("data")
        .data(vec![(0.0, 0.0), (5.0, 5.0), (10.0, 2.0)])
        .color(Color::Cyan);
    let plot = LinePlot::new().series(s).title("Crosshair Test");

    let area = Rect::new(0, 0, 60, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);

    // Create a PlotArea for the crosshair to render on
    let pa = PlotArea {
        x: 8,
        y: 1,
        width: 50,
        height: 16,
        x_lo: 0.0,
        x_hi: 10.0,
        y_lo: 0.0,
        y_hi: 5.0,
        area,
    };

    let cursor = Crosshair::new(5.0, 2.5)
        .color(Color::Yellow)
        .show_labels(true);
    cursor.render_on(&pa, &mut buf);
    // No panic is the test
}

// ===== 8. Enhanced existing widgets =====

#[test]
fn test_line_plot_step_mode_pre_renders() {
    use ratatui_plt::widgets::line_plot::StepMode;

    let s = Series::new("step")
        .data(vec![(0.0, 1.0), (1.0, 3.0), (2.0, 2.0), (3.0, 4.0)])
        .color(Color::Cyan);
    let plot = LinePlot::new()
        .series(s)
        .step_mode(StepMode::Pre)
        .title("Step Pre");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_line_plot_step_mode_mid_renders() {
    use ratatui_plt::widgets::line_plot::StepMode;

    let s = Series::new("step")
        .data(vec![(0.0, 1.0), (1.0, 3.0), (2.0, 2.0), (3.0, 4.0)])
        .color(Color::Yellow);
    let plot = LinePlot::new()
        .series(s)
        .step_mode(StepMode::Mid)
        .title("Step Mid");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_line_plot_step_mode_post_renders() {
    use ratatui_plt::widgets::line_plot::StepMode;

    let s = Series::new("step")
        .data(vec![(0.0, 1.0), (1.0, 3.0), (2.0, 2.0), (3.0, 4.0)])
        .color(Color::Green);
    let plot = LinePlot::new()
        .series(s)
        .step_mode(StepMode::Post)
        .title("Step Post");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_heatmap_show_values_renders() {
    let data = GridData::from_fn((-1.0, 1.0), (-1.0, 1.0), 5, 5, |x, y| x + y);
    let hm = Heatmap::new(data).show_values(true).title("Heatmap Values");

    let area = Rect::new(0, 0, 60, 20);
    let mut buf = Buffer::empty(area);
    (&hm).render(area, &mut buf);
}

#[test]
fn test_heatmap_mask_renders() {
    let data = GridData::from_fn((-1.0, 1.0), (-1.0, 1.0), 5, 5, |x, y| x * y);
    // Mask the diagonal cells
    let mask = (0..5).map(|i| (0..5).map(|j| i == j).collect()).collect();
    let hm = Heatmap::new(data).mask(mask).title("Masked Heatmap");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&hm).render(area, &mut buf);
}

#[test]
fn test_contour_show_labels_renders() {
    let data = GridData::from_fn((-2.0, 2.0), (-2.0, 2.0), 15, 15, |x, y| x * x + y * y);
    let contour = ContourPlot::new(data)
        .levels(5)
        .show_labels(true)
        .title("Contour Labels");

    let area = Rect::new(0, 0, 60, 30);
    let mut buf = Buffer::empty(area);
    (&contour).render(area, &mut buf);
}

#[test]
fn test_box_plot_show_means_renders() {
    let g1 = BoxData::new(
        "A",
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
        Color::Cyan,
    );
    let plot = BoxPlot::new()
        .box_data(g1)
        .show_means(true)
        .title("Box Means");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_box_plot_notch_renders() {
    let g1 = BoxData::new(
        "A",
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
        Color::Cyan,
    );
    let g2 = BoxData::new(
        "B",
        vec![3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
        Color::Yellow,
    );
    let plot = BoxPlot::new()
        .box_data(g1)
        .box_data(g2)
        .notch(true)
        .title("Notched Box");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_box_plot_show_means_and_notch_renders() {
    let g = BoxData::new(
        "Combined",
        (1..=20).map(|i| i as f64).collect(),
        Color::Green,
    );
    let plot = BoxPlot::new()
        .box_data(g)
        .show_means(true)
        .notch(true)
        .title("Means + Notch");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_histogram_multiple_datasets_renders() {
    let hist = Histogram::new(vec![1.0, 2.0, 3.0])
        .dataset(HistDataset::new(
            "Set A",
            vec![1.0, 1.5, 2.0, 2.5, 3.0],
            Color::Cyan,
        ))
        .dataset(HistDataset::new(
            "Set B",
            vec![2.0, 2.5, 3.0, 3.5, 4.0],
            Color::Yellow,
        ))
        .hist_mode(HistMode::Stacked)
        .title("Multi-Histogram");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&hist).render(area, &mut buf);
}

#[test]
fn test_histogram_bin_method_fd_renders() {
    let data: Vec<f64> = (0..100).map(|i| (i as f64 * 0.05).sin()).collect();
    let hist = Histogram::new(data)
        .bin_method(BinMethod::Fd)
        .title("FD Bins");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&hist).render(area, &mut buf);
}

#[test]
fn test_histogram_bin_method_scott_renders() {
    let data: Vec<f64> = (0..100).map(|i| (i as f64 * 0.05).cos()).collect();
    let hist = Histogram::new(data)
        .bin_method(BinMethod::Scott)
        .title("Scott Bins");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&hist).render(area, &mut buf);
}

#[test]
fn test_histogram_bin_method_sturges_renders() {
    let data: Vec<f64> = (0..80).map(|i| i as f64 * 0.1).collect();
    let hist = Histogram::new(data)
        .bin_method(BinMethod::Sturges)
        .title("Sturges Bins");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&hist).render(area, &mut buf);
}

#[test]
fn test_histogram_bin_method_sqrt_renders() {
    let data: Vec<f64> = (0..64).map(|i| i as f64 * 0.1).collect();
    let hist = Histogram::new(data)
        .bin_method(BinMethod::Sqrt)
        .title("Sqrt Bins");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&hist).render(area, &mut buf);
}

#[test]
fn test_histogram_bin_method_auto_renders() {
    let data: Vec<f64> = (0..200).map(|i| (i as f64 * 0.03).sin() * 5.0).collect();
    let hist = Histogram::new(data)
        .bin_method(BinMethod::Auto)
        .title("Auto Bins");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&hist).render(area, &mut buf);
}

#[test]
fn test_histogram_step_type_renders() {
    let data: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
    let hist = Histogram::new(data)
        .histtype(HistType::Step)
        .bins(10)
        .title("Step Histogram");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&hist).render(area, &mut buf);
}

#[test]
fn test_histogram_side_by_side_mode_renders() {
    let hist = Histogram::new(vec![1.0])
        .dataset(HistDataset::new(
            "X",
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
            Color::Cyan,
        ))
        .dataset(HistDataset::new(
            "Y",
            vec![2.0, 3.0, 4.0, 5.0, 6.0],
            Color::Red,
        ))
        .hist_mode(HistMode::SideBySide)
        .bins(5)
        .title("Side-by-side");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&hist).render(area, &mut buf);
}

#[test]
fn test_violin_plot_split_mode_renders() {
    let vals1: Vec<f64> = (0..30)
        .map(|i| 5.0 + (i as f64 * 0.3).sin() * 2.0)
        .collect();
    let vals2: Vec<f64> = (0..30)
        .map(|i| 5.0 + (i as f64 * 0.2).cos() * 3.0)
        .collect();
    let d1 = ViolinData::new("Male", vals1, Color::Cyan);
    let d2 = ViolinData::new("Female", vals2, Color::Magenta);
    let plot = ViolinPlot::new()
        .dataset(d1)
        .dataset(d2)
        .split(true)
        .title("Split Violin");

    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

// ========================================================================
// Gap Analysis Implementation Tests
// ========================================================================

#[test]
fn test_minor_grid_color_theme() {
    let theme = Theme::dark();
    assert!(matches!(theme.minor_grid_color, Color::Rgb(40, 40, 40)));
}

#[test]
fn test_tick_direction_and_params() {
    use ratatui_plt::axis::TickDirection;
    let axis = Axis::new()
        .tick_direction(TickDirection::InOut)
        .tick_size(2)
        .tick_padding(2)
        .label_rotation(LabelRotation::Vertical);
    assert_eq!(axis.tick_direction, TickDirection::InOut);
    assert_eq!(axis.tick_size, 2);
    assert_eq!(axis.tick_padding, 2);
    assert_eq!(axis.label_rotation, LabelRotation::Vertical);
}

#[test]
fn test_custom_dash_pattern_renders() {
    let s = Series::new("custom_dash")
        .data(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.5)])
        .color(Color::Cyan)
        .line_style(LineStyle {
            pattern: DashPattern::Custom(vec![6, 3]),
            thickness: ratatui_plt::style::Thickness::Normal,
        });
    let plot = LinePlot::new().series(s);
    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_hatch_pattern_char_at() {
    let fwd = HatchPattern::Forward;
    let has_some = (0..10u16)
        .flat_map(|r| (0..10u16).map(move |c| (r, c)))
        .any(|(r, c)| fwd.char_at(r, c).is_some());
    assert!(has_some);
    assert!(HatchPattern::None.char_at(0, 0).is_none());
}

#[test]
fn test_multi_column_legend_size() {
    use ratatui_plt::legend::LegendEntry;
    let entries = vec![
        LegendEntry {
            name: "AAAA".into(),
            color: Color::Red,
            marker: None,
        },
        LegendEntry {
            name: "BBBB".into(),
            color: Color::Blue,
            marker: None,
        },
        LegendEntry {
            name: "CCCC".into(),
            color: Color::Green,
            marker: None,
        },
        LegendEntry {
            name: "DDDD".into(),
            color: Color::Yellow,
            marker: None,
        },
    ];
    let l1 = ratatui_plt::legend::Legend::new(entries.clone()).columns(1);
    let l2 = ratatui_plt::legend::Legend::new(entries).columns(2);
    let (w1, h1) = l1.size();
    let (w2, h2) = l2.size();
    assert!(w2 > w1);
    assert!(h2 < h1);
}

#[test]
fn test_colorbar_extend_renders() {
    use ratatui_plt::colormap::{Colorbar, ColorbarExtend, Viridis};
    let cb = Colorbar::new(&Viridis, 0.0, 10.0).extend(ColorbarExtend::Both);
    let area = Rect::new(0, 0, 12, 20);
    let mut buf = Buffer::empty(area);
    (&cb).render(area, &mut buf);
}

#[test]
fn test_colormap_resample() {
    use ratatui_plt::colormap::{Colormap, Viridis, resample};
    let r = resample(&Viridis, 10);
    assert_eq!(r.name(), "viridis_resampled");
    assert_eq!(Viridis.color_at(0.0), r.color_at(0.0));
}

#[test]
fn test_split_at_nan() {
    let data = vec![
        (0.0, 0.0),
        (1.0, 1.0),
        (f64::NAN, 0.0),
        (3.0, 3.0),
        (4.0, 4.0),
    ];
    let segs = split_at_nan(&data);
    assert_eq!(segs.len(), 2);
    assert_eq!(segs[0].len(), 2);
    assert_eq!(segs[1].len(), 2);
}

#[test]
fn test_collections_api() {
    let lc = LineCollection::new()
        .segment((0.0, 0.0), (1.0, 1.0), Color::Red)
        .segment((1.0, 0.0), (0.0, 1.0), Color::Blue);
    assert_eq!(lc.segments.len(), 2);
    let pc =
        PathCollection::new().path(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)], Color::Green, true);
    assert_eq!(pc.paths.len(), 1);
}

#[test]
fn test_sankey_diagram_renders() {
    let d = SankeyDiagram::new()
        .node(SankeyNode {
            label: "A".into(),
            color: Color::Red,
        })
        .node(SankeyNode {
            label: "B".into(),
            color: Color::Blue,
        })
        .node(SankeyNode {
            label: "C".into(),
            color: Color::Green,
        })
        .flow(SankeyFlow {
            source: 0,
            target: 2,
            value: 5.0,
            color: None,
        })
        .flow(SankeyFlow {
            source: 1,
            target: 2,
            value: 3.0,
            color: None,
        })
        .title("Sankey");
    let area = Rect::new(0, 0, 60, 20);
    let mut buf = Buffer::empty(area);
    (&d).render(area, &mut buf);
}

#[test]
fn test_treemap_renders() {
    let root = TreemapNode {
        label: "R".into(),
        value: 100.0,
        color: None,
        children: vec![
            TreemapNode {
                label: "A".into(),
                value: 60.0,
                color: Some(Color::Red),
                children: vec![],
            },
            TreemapNode {
                label: "B".into(),
                value: 40.0,
                color: Some(Color::Blue),
                children: vec![],
            },
        ],
    };
    let tm = Treemap::new(root).title("TM");
    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&tm).render(area, &mut buf);
}

#[test]
fn test_sunburst_renders() {
    let root = SunburstNode {
        label: "R".into(),
        value: 100.0,
        color: None,
        children: vec![
            SunburstNode {
                label: "A".into(),
                value: 60.0,
                color: Some(Color::Cyan),
                children: vec![],
            },
            SunburstNode {
                label: "B".into(),
                value: 40.0,
                color: Some(Color::Yellow),
                children: vec![],
            },
        ],
    };
    let sb = Sunburst::new(root).title("SB");
    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&sb).render(area, &mut buf);
}

#[test]
fn test_ternary_plot_renders() {
    let data = TernaryData {
        label: "Mix".into(),
        points: vec![(0.5, 0.3, 0.2), (0.1, 0.8, 0.1)],
        color: Color::Cyan,
        marker: MarkerShape::FilledCircle,
    };
    let plot = TernaryPlot::new().dataset(data).title("Tern");
    let area = Rect::new(0, 0, 50, 25);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_network_plot_renders() {
    let plot = NetworkPlot::new()
        .node(GraphNode {
            label: "A".into(),
            color: Color::Cyan,
            position: None,
            marker: MarkerShape::FilledCircle,
        })
        .node(GraphNode {
            label: "B".into(),
            color: Color::Yellow,
            position: None,
            marker: MarkerShape::FilledCircle,
        })
        .edge(GraphEdge {
            source: 0,
            target: 1,
            weight: 1.0,
            color: None,
        })
        .layout(GraphLayout::Circular)
        .title("Net");
    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_triangulation_and_triplot() {
    use ratatui_plt::triangulation::Triangulation;
    let tri =
        Triangulation::from_explicit(vec![(0.0, 0.0), (1.0, 0.0), (0.5, 0.87)], vec![(0, 1, 2)]);
    assert_eq!(tri.edges().len(), 3);
    let plot = TriPlot::new(tri).title("Tri");
    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_inset_axes_rect() {
    let inset = InsetAxes::new(0.5, 0.1, 0.4, 0.3);
    let parent = Rect::new(0, 0, 100, 50);
    let r = inset.rect(parent);
    assert_eq!(r.x, 50);
    assert_eq!(r.y, 5);
}

#[test]
fn test_pick_nearest() {
    let pa = PlotArea {
        x: 0,
        y: 0,
        width: 100,
        height: 50,
        x_lo: 0.0,
        x_hi: 10.0,
        y_lo: 0.0,
        y_hi: 10.0,
        area: Rect::new(0, 0, 100, 50),
    };
    let s1 = Series::new("a")
        .data(vec![(2.0, 3.0), (5.0, 5.0)])
        .color(Color::Red);
    let result = pick_nearest(
        pa.screen_x(5.0).round() as u16,
        pa.screen_y(5.0).round() as u16,
        &[s1],
        &pa,
    );
    assert!(result.is_some());
    assert_eq!(result.unwrap().point_index, 1);
}

#[test]
fn test_brush_state() {
    let mut brush = BrushState::new();
    brush.set_selection(1.0, 1.0, 5.0, 5.0);
    assert!(brush.contains(3.0, 3.0));
    assert!(!brush.contains(0.0, 0.0));
    let data = vec![(0.0, 0.0), (3.0, 3.0), (6.0, 6.0)];
    brush.update_indices(&[&data]);
    assert_eq!(brush.selected_indices[0], vec![1]);
}

#[test]
fn test_theme_guard_restores() {
    let orig = Theme::get_default();
    {
        let _g = Theme::solarized().activate();
        assert!(matches!(
            Theme::get_default().background,
            Color::Rgb(0, 43, 54)
        ));
    }
    assert_eq!(
        format!("{:?}", orig.background),
        format!("{:?}", Theme::get_default().background)
    );
}

#[test]
fn test_plot_config_activate() {
    let mut cfg = PlotConfig::default();
    cfg.grid_visible = true;
    let _g = cfg.activate();
    assert!(PlotConfig::get_default().grid_visible);
}

#[test]
fn test_radial_plot_enhancements() {
    let data: Vec<(f64, f64)> = (0..36)
        .map(|i| (i as f64 * std::f64::consts::TAU / 36.0, 1.0))
        .collect();
    let s = Series::new("c").data(data).color(Color::Magenta);
    let plot = RadialPlot::new()
        .series(s)
        .theta_direction(ThetaDirection::Clockwise)
        .plot_type(PolarPlotType::Scatter)
        .title("Enhanced Polar");
    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}

#[test]
fn test_box_plot_bootstrap_ci() {
    let data: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
    let g = BoxData::new("Bootstrap", data, Color::Cyan);
    let plot = BoxPlot::new()
        .box_data(g)
        .bootstrap_ci(true)
        .bootstrap_n(100)
        .title("CI");
    let area = Rect::new(0, 0, 40, 20);
    let mut buf = Buffer::empty(area);
    (&plot).render(area, &mut buf);
}
