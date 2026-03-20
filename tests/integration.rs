use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{StatefulWidget, Widget};

use ratatui_plt::prelude::*;
use ratatui_plt::widgets::bar_chart::{BarDataset, BarMode};
use ratatui_plt::widgets::box_plot::BoxData;
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
    let plot = Hist2D::new(data)
        .bins_x(10)
        .bins_y(10)
        .title("Hist2D");

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
        .data((0..10).map(|i| (i as f64, (i as f64 * 0.3).sin().abs())).collect())
        .color(Color::Cyan);
    let s2 = Series::new("B")
        .data((0..10).map(|i| (i as f64, (i as f64 * 0.2).cos().abs())).collect())
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
    let data: Vec<(f64, f64)> = (0..10).map(|i| (i as f64, (i as f64 * 0.5).sin())).collect();
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
    let vals1: Vec<f64> = (0..20).map(|i| 5.0 + (i as f64 * 0.3).sin() * 2.0).collect();
    let vals2: Vec<f64> = (0..20).map(|i| 7.0 + (i as f64 * 0.2).cos() * 3.0).collect();
    let d1 = ViolinData::new("A", vals1, Color::Cyan);
    let d2 = ViolinData::new("B", vals2, Color::Yellow);
    let plot = ViolinPlot::new()
        .dataset(d1)
        .dataset(d2)
        .title("Violin");

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
    assert_eq!(chars.len(), unique.len(), "Marker shapes must have distinct chars");
}

#[test]
fn test_line_style_constructors() {
    let solid = LineStyle::solid();
    let dashed = LineStyle::dashed();
    let dotted = LineStyle::dotted();
    // They should produce different patterns
    assert_ne!(format!("{:?}", solid.pattern), format!("{:?}", dashed.pattern));
    assert_ne!(format!("{:?}", dashed.pattern), format!("{:?}", dotted.pattern));
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
