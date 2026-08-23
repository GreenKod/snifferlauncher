use super::constants::{
    PAGE_DOT_MARGIN_BOTTOM, PAGE_DOT_SIZE, PAGE_DOT_SPACING, SCROLLBAR_MIN_THUMB_RATIO,
    SCROLLBAR_WIDTH,
};
use super::state::ScrollState;
use sniffer_core::{math::Rect, render_api::Renderer};

pub fn render_scroll_view(
    st: &ScrollState,
    renderer: &mut dyn Renderer,
    layout_rect: Rect,
    clip_rect: Option<Rect>,
) {
    let effective_clip = clip_rect.unwrap_or(layout_rect);
    renderer.push_clip_rect(effective_clip, 0.0);

    renderer.push_transform(
        0.0,
        0.0,
        1.0, // scale
        0.0, // rotation
        -st.scroll_x,
        -st.scroll_y,
    );

    renderer.pop_transform();

    // Scrollbar indicator (vertical)
    if st.max_scroll_y > f32::EPSILON {
        let track_h = layout_rect.height;
        let content_h = track_h + st.max_scroll_y;
        let thumb_h = (track_h * (track_h / content_h)).max(track_h * SCROLLBAR_MIN_THUMB_RATIO);
        let scroll_fraction = (st.scroll_y / st.max_scroll_y).clamp(0.0, 1.0);
        let thumb_y = layout_rect.y + scroll_fraction * (track_h - thumb_h);
        let thumb_x = layout_rect.x + layout_rect.width - SCROLLBAR_WIDTH - 2.0;

        renderer.draw_rect(
            Rect {
                x: thumb_x,
                y: thumb_y,
                width: SCROLLBAR_WIDTH,
                height: thumb_h,
            },
            0x66_FF_FF_FF, // semi-transparent white thumb
            SCROLLBAR_WIDTH * 0.5,
            0.0,
            None,
        );
    }

    // Page dots (horizontal pager indicator)
    if let Some(snap_x) = st.snap_x {
        if snap_x > f32::EPSILON && st.page_count > 1 {
            let count = st.page_count as usize;
            let total_w =
                count as f32 * PAGE_DOT_SIZE + (count.saturating_sub(1)) as f32 * PAGE_DOT_SPACING;
            let start_x = layout_rect.x + (layout_rect.width - total_w) * 0.5;
            let dot_y = layout_rect.y + layout_rect.height - PAGE_DOT_MARGIN_BOTTOM;

            for i in 0..count {
                let dot_x = start_x + i as f32 * (PAGE_DOT_SIZE + PAGE_DOT_SPACING);
                let color = if i32::try_from(i) == Ok(st.current_page) {
                    0xFF_FF_FF_FF // solid white for active page
                } else {
                    0x44_FF_FF_FF // dim white for inactive pages
                };
                renderer.draw_circle(
                    dot_x + PAGE_DOT_SIZE * 0.5,
                    dot_y + PAGE_DOT_SIZE * 0.5,
                    PAGE_DOT_SIZE * 0.5,
                    color,
                );
            }
        }
    }

    renderer.pop_clip_rect();
}
