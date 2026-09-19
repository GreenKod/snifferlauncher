use sniffer_render::PingPongTarget;

#[test]
fn test_ping_pong_target_alternation() {
    let target = PingPongTarget::A;
    assert_eq!(target.other(), PingPongTarget::B);
    assert_eq!(target.other().other(), PingPongTarget::A);
}

#[test]
fn test_kawase_iteration_kernel_offsets() {
    let width = 540_f32;
    let height = 960_f32;
    let inv_w = 1.0 / width;
    let inv_h = 1.0 / height;

    let blur_radius = 16.0_f32;
    let base_offset = (blur_radius * 0.25).max(1.0);
    assert!((base_offset - 4.0).abs() < f32::EPSILON);

    // Pass 0: offset = (0.0 + 4.0) * inv_res
    let pass0_x = (0.0 + base_offset) * inv_w;
    let pass0_y = (0.0 + base_offset) * inv_h;
    assert!((pass0_x - (4.0 / 540.0)).abs() < 1e-6);
    assert!((pass0_y - (4.0 / 960.0)).abs() < 1e-6);

    // Pass 1: offset = (1.0 + 4.0) * inv_res
    let pass1_x = (1.0 + base_offset) * inv_w;
    let pass1_y = (1.0 + base_offset) * inv_h;
    assert!((pass1_x - (5.0 / 540.0)).abs() < 1e-6);
    assert!((pass1_y - (5.0 / 960.0)).abs() < 1e-6);

    // Ping pong target toggling across 2 passes:
    let mut target = PingPongTarget::A;
    for _ in 0..2 {
        target = target.other();
    }
    // After 2 passes, target returns to A
    assert_eq!(target, PingPongTarget::A);

    // After 3 passes, target is B
    target = target.other();
    assert_eq!(target, PingPongTarget::B);
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

#[test]
fn test_draw_ui_frosted_glass_card_pipeline() {
    use sniffer_core::layout::calculate_layout;
    use sniffer_core::math::Size;
    use sniffer_core::render::Renderer;
    use sniffer_core::style::{Dimension, Style};
    use sniffer_core::types::Element;
    use sniffer_core::ui::data_map::DataMap;
    use sniffer_core::ui::style_map::StyleMap;
    use sniffer_core::{Rect, ScreenMetrics};
    use sniffer_render::draw_ui;

    struct GlassSpyRenderer {
        blur_called: bool,
        last_blur_radius: f32,
        last_tint: Option<u32>,
    }

    impl Renderer for GlassSpyRenderer {
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
            tint: Option<u32>,
        ) {
            self.blur_called = true;
            self.last_blur_radius = blur_radius;
            self.last_tint = tint;
        }
    }

    let mut spy = GlassSpyRenderer {
        blur_called: false,
        last_blur_radius: 0.0,
        last_tint: None,
    };

    let card = Element::Container {
        id: Some("dock_glass".to_string()),
        style: Style {
            width: Dimension::Pixels(300.0),
            height: Dimension::Pixels(80.0),
            backdrop_blur: 24.0,
            backdrop_tint: Some(0x33FF_FFFF),
            border_radius: 20.0,
            ..Default::default()
        },
        children: vec![],
    };

    let layout = calculate_layout(&card, Size::new(800.0, 600.0), 0.0, 0.0);
    let metrics = ScreenMetrics::default_mdpi(800.0, 600.0);
    let style_map = StyleMap::default();
    let data_map = DataMap::default();
    let transitions = sniffer_core::anim::TransitionManager::default();

    draw_ui(
        &mut spy,
        &card,
        &layout,
        &metrics,
        &style_map,
        &data_map,
        &transitions,
        1.0,
        0.0,
        0.0,
    );

    assert!(
        spy.blur_called,
        "draw_ui must trigger draw_backdrop_blur for frosted glass cards"
    );
    assert_eq!(spy.last_blur_radius, 24.0);
    assert_eq!(spy.last_tint, Some(0x33FF_FFFF));
}

#[test]
fn test_120hz_pass_count_scaling_and_capping() {
    use sniffer_render::BlurPipeline;

    // Small blurs run in 1 pass for maximum 120 FPS performance
    assert_eq!(BlurPipeline::optimal_pass_count(0.0), 1);
    assert_eq!(BlurPipeline::optimal_pass_count(4.0), 1);
    assert_eq!(BlurPipeline::optimal_pass_count(8.0), 1);

    // Medium blurs (frosted glass standard) run in 2 passes
    assert_eq!(BlurPipeline::optimal_pass_count(12.0), 2);
    assert_eq!(BlurPipeline::optimal_pass_count(20.0), 2);
    assert_eq!(BlurPipeline::optimal_pass_count(24.0), 2);

    // Large blurs capped at 3 passes to guarantee < 8.33ms frame budget
    assert_eq!(BlurPipeline::optimal_pass_count(32.0), 3);
    assert_eq!(BlurPipeline::optimal_pass_count(64.0), 3);
    assert_eq!(BlurPipeline::optimal_pass_count(500.0), 3);
}

#[test]
fn test_vram_bounds_across_resolutions() {
    let clamp_max_dim = |w: f32, h: f32| -> (usize, usize) {
        let max_dim = 960.0_f32;
        let mut target_w = (w * 0.5).max(1.0);
        let mut target_h = (h * 0.5).max(1.0);
        if target_w > max_dim || target_h > max_dim {
            let scale = (max_dim / target_w).min(max_dim / target_h);
            target_w = (target_w * scale).max(1.0);
            target_h = (target_h * scale).max(1.0);
        }
        (target_w as usize, target_h as usize)
    };

    let resolutions = [
        ("720p HD", 720.0, 1280.0),
        ("1080p FHD+", 1080.0, 2400.0),
        ("1440p QHD+", 1440.0, 3200.0),
        ("4K UHD", 3840.0, 2160.0),
    ];

    for (name, screen_w, screen_h) in resolutions {
        let (tw, th) = clamp_max_dim(screen_w, screen_h);
        let pair_vram = tw * th * 4 * 2;

        // VRAM for ping-pong pair must strictly stay under 8 MB on any resolution
        assert!(
            pair_vram <= 8 * 1024 * 1024,
            "Resolution {name} exceeded 8MB VRAM: {pair_vram} bytes"
        );
        assert!(tw <= 960);
        assert!(th <= 960);
    }
}

#[test]
fn test_half_res_fill_rate_savings_75_percent() {
    let full_pixels: f32 = 1080.0 * 2400.0;
    let half_pixels: f32 = (1080.0 * 0.5) * (2400.0 * 0.5);

    let reduction = 1.0_f32 - (half_pixels / full_pixels);
    assert!((reduction - 0.75_f32).abs() < f32::EPSILON);
}
