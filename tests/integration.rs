use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use ratatui_plt::prelude::*;
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
    (&surface).render(area, &mut buf);
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
