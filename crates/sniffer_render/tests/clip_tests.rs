use sniffer_core::layout::LayoutNode;
use sniffer_core::math::Rect;
use sniffer_core::render::{ClipRegion, Renderer};
use sniffer_core::style::{ObjectFit, Style};
use sniffer_core::types::Element;
use sniffer_core::ui::{DataMap, StyleMap};
use sniffer_render::draw::draw_ui;

/// A recording renderer that records all clip and transform operations to verify
/// that ScrollView and overflow_hidden correctly establish and balance rounded clip regions.
#[derive(Default)]
struct RecordingClipRenderer {
    pub pushed_regions: Vec<ClipRegion>,
    pub popped_count: usize,
    pub current_depth: usize,
    pub max_depth: usize,
    pub recorded_transforms: Vec<[f32; 9]>,
    pub current_clip_stack: Vec<ClipRegion>,
}

impl Renderer for RecordingClipRenderer {
    fn clear(&mut self, _color: u32) {}
    fn draw_rect(
        &mut self,
        _rect: Rect,
        _color: u32,
        _radius: f32,
        _border_width: f32,
        _border_color: Option<u32>,
    ) {
    }
    fn draw_rect_gradient(
        &mut self,
        _rect: Rect,
        _color_top: u32,
        _color_bottom: u32,
        _radius: f32,
        _border_width: f32,
        _border_color: Option<u32>,
    ) {
    }
    fn draw_shadow(
        &mut self,
        _rect: Rect,
        _radius: f32,
        _offset_y: f32,
        _spread: f32,
        _color: u32,
    ) {
    }
    fn draw_circle(&mut self, _cx: f32, _cy: f32, _radius: f32, _color: u32) {}
    fn draw_text(&mut self, _text: &str, _x: f32, _y: f32, _size: f32, _color: u32) {}
    fn begin_frame(&mut self, _width: f32, _height: f32) {}
    fn end_frame(&mut self) {}

    fn set_clip_rect(&mut self, rect: Rect) {
        self.push_clip_region(ClipRegion::from_rect(rect));
    }
    fn clear_clip_rect(&mut self) {
        self.current_clip_stack.clear();
        self.current_depth = 0;
    }

    fn push_clip_region(&mut self, region: ClipRegion) {
        self.pushed_regions.push(region);
        self.current_clip_stack.push(region);
        self.current_depth += 1;
        self.max_depth = self.max_depth.max(self.current_depth);
    }

    fn pop_clip_rect(&mut self) {
        self.popped_count += 1;
        if self.current_depth > 0 {
            self.current_depth -= 1;
            self.current_clip_stack.pop();
        }
    }

    fn current_clip(&self) -> Option<&ClipRegion> {
        self.current_clip_stack.last()
    }

    fn push_transform(&mut self, _cx: f32, _cy: f32, _scale: f32, _rotate: f32, tx: f32, ty: f32) {
        self.recorded_transforms
            .push([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, tx, ty, 1.0]);
    }
    fn pop_transform(&mut self) {}

    fn set_global_alpha(&mut self, _alpha: f32) {}
    fn load_image(&mut self, _id: &str, _rgba_pixels: &[u8], _width: u32, _height: u32) {}
    fn has_image(&self, _id: &str) -> bool {
        false
    }
    fn draw_image(&mut self, _id: &str, _rect: Rect, _radius: f32, _object_fit: ObjectFit) {}
    fn measure_text(&self, text: &str, size: f32) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        (text.len() as f32 * size * 0.5)
    }
}

/// Signed distance field for rounded box matching GLSL sdRoundedBox
fn sd_rounded_box(p: (f32, f32), b: (f32, f32), r: f32) -> f32 {
    let q = (p.0.abs() - b.0 + r, p.1.abs() - b.1 + r);
    let max_q = (q.0.max(0.0), q.1.max(0.0));
    let length = max_q.0.hypot(max_q.1);
    let min_q = q.0.max(q.1).min(0.0);
    length + min_q - r
}

/// Fragment shader smoothstep antialiasing calculation matching GLSL
fn sdf_clip_alpha(p: (f32, f32), center: (f32, f32), half: (f32, f32), radius: f32) -> f32 {
    let local = (p.0 - center.0, p.1 - center.1);
    let dist = sd_rounded_box(local, half, radius);
    let edge0 = -0.5;
    let edge1 = 0.5;
    let t = ((dist - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    1.0 - t * t * 3.0f32.mul_add(1.0, -(2.0 * t))
}

#[test]
fn test_scrollview_clip_stack_lifecycle_and_radius() {
    let child = Element::Container {
        id: Some("card_1".to_string()),
        style: Style::default(),
        children: vec![],
    };
    let child_layout = LayoutNode {
        element: child.clone(),
        rect: Rect::new(0.0, 0.0, 200.0, 80.0),
        children: vec![],
    };

    let scroll_style = Style {
        border_radius: 24.0,
        ..Default::default()
    };

    let scroll_view = Element::ScrollView {
        id: Some("app_scroll".to_string()),
        style: scroll_style,
        children: vec![child],
        scroll_x: 0.0,
        scroll_y: 50.0,
        scroll_sensitivity: None,
        dynamic_sensitivity: None,
        momentum_scrolling: None,
        capture_drag: None,
        snap_x: None,
        snap_y: None,
        rubber_band: None,
        page_count: None,
        on_snap: None,
    };

    let scroll_layout = LayoutNode {
        element: scroll_view.clone(),
        rect: Rect::new(20.0, 40.0, 360.0, 600.0),
        children: vec![child_layout],
    };

    let mut renderer = RecordingClipRenderer::default();
    let style_map = StyleMap::default();
    let data_map = DataMap::default();
    let metrics = sniffer_core::math::ScreenMetrics::default_mdpi(800.0, 600.0);
    let transitions = sniffer_core::anim::TransitionManager::default();

    draw_ui(
        &mut renderer,
        &scroll_view,
        &scroll_layout,
        &metrics,
        &style_map,
        &data_map,
        &transitions,
        1.0,
        0.0,
        0.0,
    );

    // Verify clip was pushed with radius 24.0 and matching bounds
    assert_eq!(renderer.pushed_regions.len(), 1);
    let clip = renderer.pushed_regions[0];
    assert_eq!(clip.radius, 24.0);
    assert_eq!(clip.rect, Rect::new(20.0, 40.0, 360.0, 600.0));
    assert!(clip.is_rounded());

    // Verify clip stack is completely balanced after drawing
    assert_eq!(renderer.popped_count, 1);
    assert_eq!(renderer.current_depth, 0);
    assert!(renderer.current_clip().is_none());
}

#[test]
fn test_overflow_hidden_rounded_container_clip() {
    let container_style = Style {
        overflow_hidden: true,
        border_radius: 16.0,
        ..Default::default()
    };

    let child = Element::Container {
        id: Some("inner".to_string()),
        style: Style::default(),
        children: vec![],
    };
    let child_layout = LayoutNode {
        element: child.clone(),
        rect: Rect::new(10.0, 10.0, 100.0, 50.0),
        children: vec![],
    };

    let container = Element::Container {
        id: Some("card".to_string()),
        style: container_style,
        children: vec![child],
    };
    let container_layout = LayoutNode {
        element: container.clone(),
        rect: Rect::new(50.0, 50.0, 200.0, 100.0),
        children: vec![child_layout],
    };

    let mut renderer = RecordingClipRenderer::default();
    let style_map = StyleMap::default();
    let data_map = DataMap::default();
    let metrics = sniffer_core::math::ScreenMetrics::default_mdpi(800.0, 600.0);
    let transitions = sniffer_core::anim::TransitionManager::default();

    draw_ui(
        &mut renderer,
        &container,
        &container_layout,
        &metrics,
        &style_map,
        &data_map,
        &transitions,
        1.0,
        0.0,
        0.0,
    );

    assert_eq!(renderer.pushed_regions.len(), 1);
    let clip = renderer.pushed_regions[0];
    assert_eq!(clip.radius, 16.0);
    assert_eq!(clip.rect, Rect::new(50.0, 50.0, 200.0, 100.0));
    assert!(clip.is_rounded());
    assert_eq!(renderer.current_depth, 0);
}

#[test]
fn test_analytical_sdf_rounded_box_antialiasing() {
    let center = (50.0, 50.0);
    let half_size = (50.0, 50.0); // 100x100 box centered at (50, 50)
    let radius = 20.0;

    // Point in center: deep interior -> alpha == 1.0
    let alpha_inside = sdf_clip_alpha((50.0, 50.0), center, half_size, radius);
    assert_eq!(alpha_inside, 1.0);

    // Point on straight edge: x = 0.0, y = 50.0 -> dist == 0.0 -> alpha == 0.5
    let alpha_edge = sdf_clip_alpha((0.0, 50.0), center, half_size, radius);
    assert!((alpha_edge - 0.5).abs() < 1e-5);

    // Point outside straight edge: x = -2.0, y = 50.0 -> dist == 2.0 > 0.5 -> alpha == 0.0
    let alpha_outside = sdf_clip_alpha((-2.0, 50.0), center, half_size, radius);
    assert_eq!(alpha_outside, 0.0);

    // Corner at (0, 0): outside the rounded arc (dist ~ 8.28 > 0.5) -> alpha == 0.0
    let alpha_corner_outside = sdf_clip_alpha((0.0, 0.0), center, half_size, radius);
    assert_eq!(alpha_corner_outside, 0.0);

    // Smooth transition across edge: dist = -0.25 -> alpha should be between 0.5 and 1.0
    let alpha_transition = sdf_clip_alpha((0.25, 50.0), center, half_size, radius);
    assert!(alpha_transition > 0.5 && alpha_transition < 1.0);
}
