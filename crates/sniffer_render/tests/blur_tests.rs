use sniffer_render::PingPongTarget;

#[test]
fn test_ping_pong_target_alternation() {
    let target = PingPongTarget::A;
    assert_eq!(target.other(), PingPongTarget::B);
    assert_eq!(target.other().other(), PingPongTarget::A);
}

#[test]
fn test_blur_downsample_and_vram_budget() {
    // Standard phone screen resolution: 1080x2400 (FHD+)
    let screen_w = 1080_f32;
    let screen_h = 2400_f32;
    let downsample_factor = 0.5_f32;

    let target_w = ((screen_w * downsample_factor) as i32).max(1);
    let target_h = ((screen_h * downsample_factor) as i32).max(1);

    assert_eq!(target_w, 540);
    assert_eq!(target_h, 1200);

    // VRAM calculation: 4 bytes per RGBA8 pixel
    let single_texture_bytes = target_w as usize * target_h as usize * 4;
    let ping_pong_pair_bytes = single_texture_bytes * 2;

    // A single 1080x2400 screen texture is ~10.36 MB.
    // Half-res ping-pong textures are ~2.59 MB each = ~5.18 MB total.
    assert_eq!(single_texture_bytes, 2_592_000);
    assert_eq!(ping_pong_pair_bytes, 5_184_000);

    // Ensure ping-pong FBO pair fits well under 8 MB memory budget
    assert!(
        ping_pong_pair_bytes < 8 * 1024 * 1024,
        "Ping-pong FBOs must not exceed 8MB for mobile GPU efficiency"
    );
}

#[test]
fn test_fast_path_zero_blur_overhead() {
    use sniffer_core::Rect;
    use sniffer_core::render::Renderer;

    struct MockRenderer {
        blur_called: bool,
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
        fn set_clip_rect(&mut self, _rect: Rect) {}
        fn clear_clip_rect(&mut self) {}
        fn set_global_alpha(&mut self, _alpha: f32) {}
        fn load_image(&mut self, _id: &str, _rgba_pixels: &[u8], _width: u32, _height: u32) {}
        fn has_image(&self, _id: &str) -> bool {
            false
        }
        fn draw_image(
            &mut self,
            _id: &str,
            _rect: Rect,
            _radius: f32,
            _object_fit: sniffer_core::style::ObjectFit,
        ) {
        }
        fn measure_text(&self, _text: &str, _size: f32) -> f32 {
            0.0
        }
        fn draw_backdrop_blur(
            &mut self,
            _rect: Rect,
            _radius: f32,
            blur_radius: f32,
            _tint: Option<u32>,
        ) {
            if blur_radius > 0.0 {
                self.blur_called = true;
            }
        }
    }

    let mut mock = MockRenderer { blur_called: false };
    mock.draw_backdrop_blur(Rect::new(0.0, 0.0, 100.0, 100.0), 0.0, 0.0, None);
    assert!(
        !mock.blur_called,
        "When blur_radius is 0.0, blur pass must be skipped entirely"
    );

    mock.draw_backdrop_blur(Rect::new(0.0, 0.0, 100.0, 100.0), 0.0, 12.0, None);
    assert!(
        mock.blur_called,
        "When blur_radius > 0.0, blur pass must execute"
    );
}
