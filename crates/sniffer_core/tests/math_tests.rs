use sniffer_core::math::{Point, Rect, ScreenMetrics, Size};

#[test]
fn test_rect_geometry_and_contains() {
    let r = Rect::new(10.0, 20.0, 100.0, 50.0);
    assert_eq!(r.x, 10.0);
    assert_eq!(r.y, 20.0);
    assert_eq!(r.width, 100.0);
    assert_eq!(r.height, 50.0);

    // Inside points
    assert!(r.contains(Point::new(10.0, 20.0)));
    assert!(r.contains(Point::new(50.0, 45.0)));
    assert!(r.contains(Point::new(110.0, 70.0)));

    // Outside points
    assert!(!r.contains(Point::new(9.9, 20.0)));
    assert!(!r.contains(Point::new(10.0, 19.9)));
    assert!(!r.contains(Point::new(110.1, 70.0)));
    assert!(!r.contains(Point::new(50.0, 70.1)));
}

#[test]
fn test_rect_default() {
    let r = Rect::default();
    assert_eq!(r.x, 0.0);
    assert_eq!(r.y, 0.0);
    assert_eq!(r.width, 0.0);
    assert_eq!(r.height, 0.0);
}

#[test]
fn test_point_and_size() {
    let p = Point::new(15.5, 30.25);
    assert_eq!(p.x, 15.5);
    assert_eq!(p.y, 30.25);

    let s = Size::new(800.0, 600.0);
    assert_eq!(s.width, 800.0);
    assert_eq!(s.height, 600.0);
}

#[test]
fn test_screen_metrics_from_dpi() {
    // 160 dpi -> 1.0 scale
    let m = ScreenMetrics::from_dpi(1080.0, 1920.0, 160.0);
    assert!((m.scale_factor - 1.0).abs() < f32::EPSILON);
    assert_eq!(m.physical_width, 1080.0);
    assert_eq!(m.physical_height, 1920.0);

    // 320 dpi -> 2.0 scale
    let m2 = ScreenMetrics::from_dpi(1080.0, 2400.0, 320.0);
    assert!((m2.scale_factor - 2.0).abs() < f32::EPSILON);

    // Clamped scale factors
    let m_min = ScreenMetrics::from_dpi(720.0, 1280.0, 50.0);
    assert!((m_min.scale_factor - 0.75).abs() < f32::EPSILON);

    let m_max = ScreenMetrics::from_dpi(1440.0, 3200.0, 800.0);
    assert!((m_max.scale_factor - 4.0).abs() < f32::EPSILON);
}

#[test]
fn test_screen_metrics_from_scale() {
    let m = ScreenMetrics::from_scale(720.0, 1612.0, 2.25, 2.25);
    assert_eq!(m.physical_width, 720.0);
    assert_eq!(m.physical_height, 1612.0);
    assert!((m.scale_factor - 2.25).abs() < f32::EPSILON);
    assert!((m.font_scale_factor - 2.25).abs() < f32::EPSILON);
}
