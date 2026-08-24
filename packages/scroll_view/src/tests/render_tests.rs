use super::*;
use sniffer_core::math::Rect;
use sniffer_core::render::Renderer;
use sniffer_core::style::ObjectFit;

#[derive(Default)]
struct MockRenderer {
    pub rects_drawn: u32,
    pub circles_drawn: u32,
    pub clip_depth: i32,
    pub transform_depth: i32,
}

impl Renderer for MockRenderer {
    fn clear(&mut self, _color: u32) {}

    fn draw_rect(
        &mut self,
        _rect: Rect,
        _color: u32,
        _radius: f32,
        _border_width: f32,
        _border_color: Option<u32>,
    ) {
        self.rects_drawn += 1;
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

    fn draw_circle(&mut self, _cx: f32, _cy: f32, _radius: f32, _color: u32) {
        self.circles_drawn += 1;
    }

    fn draw_text(&mut self, _text: &str, _x: f32, _y: f32, _size: f32, _color: u32) {}

    fn begin_frame(&mut self, _width: f32, _height: f32) {}
    fn end_frame(&mut self) {}

    fn set_clip_rect(&mut self, _rect: Rect) {
        self.clip_depth += 1;
    }

    fn clear_clip_rect(&mut self) {
        self.clip_depth -= 1;
    }

    fn push_clip_rect(&mut self, _rect: Rect, _radius: f32) {
        self.clip_depth += 1;
    }

    fn pop_clip_rect(&mut self) {
        self.clip_depth -= 1;
    }

    fn push_transform(
        &mut self,
        _cx: f32,
        _cy: f32,
        _scale: f32,
        _rotate: f32,
        _tx: f32,
        _ty: f32,
    ) {
        self.transform_depth += 1;
    }

    fn pop_transform(&mut self) {
        self.transform_depth -= 1;
    }

    fn set_global_alpha(&mut self, _alpha: f32) {}

    fn load_image(&mut self, _id: &str, _pixels: &[u8], _w: u32, _h: u32) {}

    fn has_image(&self, _id: &str) -> bool {
        false
    }

    fn draw_image(&mut self, _id: &str, _rect: Rect, _radius: f32, _object_fit: ObjectFit) {}

    fn measure_text(&self, _text: &str, _size: f32) -> f32 {
        0.0
    }
}

fn layout_rect() -> Rect {
    Rect::new(0.0, 0.0, 400.0, 800.0)
}

#[test]
fn render_draws_vertical_scrollbar_when_max_scroll_y_set() {
    let pkg = ScrollViewPackage::new();
    {
        let mut st = pkg.state.write().unwrap();
        st.max_scroll_y = 800.0;
        st.scroll_y = 200.0;
    }

    let mut renderer = MockRenderer::default();
    pkg.on_render(&mut renderer, layout_rect(), None);

    assert!(
        renderer.rects_drawn >= 1,
        "must draw at least one scrollbar rect"
    );
}

#[test]
fn render_draws_page_dots_in_pager_mode() {
    let pkg = ScrollViewPackage::new();
    {
        let mut st = pkg.state.write().unwrap();
        st.snap_x = Some(400.0);
        st.page_count = 4;
    }

    let mut renderer = MockRenderer::default();
    pkg.on_render(&mut renderer, layout_rect(), None);

    assert_eq!(
        renderer.circles_drawn, 4,
        "must draw one dot per page (4 dots)"
    );
}

#[test]
fn render_clip_stack_is_balanced() {
    let pkg = ScrollViewPackage::new();
    let mut renderer = MockRenderer::default();
    pkg.on_render(&mut renderer, layout_rect(), None);

    assert_eq!(
        renderer.clip_depth, 0,
        "push/pop clip rect calls must be balanced, net depth = {}",
        renderer.clip_depth
    );
}

#[test]
fn render_transform_stack_is_balanced() {
    let pkg = ScrollViewPackage::new();
    let mut renderer = MockRenderer::default();
    pkg.on_render(&mut renderer, layout_rect(), None);

    assert_eq!(
        renderer.transform_depth, 0,
        "push/pop transform calls must be balanced"
    );
}
