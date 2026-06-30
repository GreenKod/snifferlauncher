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
}

#[no_mangle]
pub extern "C" fn on_event(widget_id: i64, event_type: i32) {
    // We only react to events.
    // Event types: Click(1), Hover(2), HoverEnd(3)
    match event_type {
        1 => {
            // Click -> Update label
            let text = "Wasm Clicked!";
            let Ok(text_len) = i32::try_from(text.len()) else {
                return;
            };
            unsafe {
                host_update_text(widget_id, text.as_ptr(), text_len);
            }
        }
        2 => {
            // Hover -> Background color Yellow (0x00RRGGBB)
            unsafe {
                host_update_bg_color(widget_id, 0x00FF_FF00); // Yellow
            }
        }
        3 => {
            // HoverEnd -> Background color Blue (0x00RRGGBB)
            unsafe {
                host_update_bg_color(widget_id, 0x0000_00FF); // Blue
            }
        }
        _ => {}
    }
}
