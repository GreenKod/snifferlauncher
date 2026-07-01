#![no_std]
#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
#[allow(clippy::missing_const_for_fn)]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[link(wasm_import_module = "env")]
extern "C" {
    fn host_update_text(widget_id: i64, text_ptr: *const u8, text_len: i32);
    fn host_update_bg_color(widget_id: i64, color: u32);
    fn host_draw_circle(widget_id: i64, cx: f32, cy: f32, r: f32, color: u32);
    fn host_clear_draw_list(widget_id: i64);
    fn host_clear_label(widget_id: i64);
    fn host_clear_style(widget_id: i64);
    fn host_set_visible(widget_id: i64, visible: i32);
    fn host_execute_action(action_id: i32);
}

#[no_mangle]
pub extern "C" fn on_event(
    widget_id: i64,
    event_type: i32,
    _width: f32,
    height: f32,
    mouse_x: f32,
    mouse_y: f32,
) {
    // We only react to events.
    // Event types: Click(1), Hover(2), HoverEnd(3)
    match event_type {
        1 => {
            // Click -> Update label AND execute platform action
            let text = "Wasm Clicked!";
            let Ok(text_len) = i32::try_from(text.len()) else {
                return;
            };
            unsafe {
                host_update_text(widget_id, text.as_ptr(), text_len);
                host_set_visible(widget_id, 0); // Hide briefly on click

                // Map widget_id to an action (Settings=1001->1, Contacts=1002->2, Camera=1003->3)
                let action_id = match widget_id {
                    1001 => 1,
                    1002 => 2,
                    1003 => 3,
                    _ => 0,
                };
                if action_id != 0 {
                    host_execute_action(action_id);
                }
            }
        }
        2 => {
            // Hover -> Complete control: Change label, change BG color, draw spotlight!
            let text = "Opening...";
            let Ok(text_len) = i32::try_from(text.len()) else {
                return;
            };
            unsafe {
                host_update_text(widget_id, text.as_ptr(), text_len);
                host_update_bg_color(widget_id, 0x1A_6B_6B_FF); // Dark teal accent from native plugin

                host_clear_draw_list(widget_id);
                host_draw_circle(
                    widget_id,
                    mouse_x,
                    mouse_y,
                    height * 0.5,
                    0x66FF_FFFF, // Spotlight ripple
                );
            }
        }
        3 => {
            // HoverEnd -> Restore everything back to core defaults
            unsafe {
                host_clear_draw_list(widget_id);
                host_clear_label(widget_id);
                host_clear_style(widget_id);
            }
        }
        _ => {}
    }
}
