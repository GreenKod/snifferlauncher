#[cfg(target_os = "android")]
pub mod intent;

#[cfg(target_os = "android")]
pub use intent::launch_action;

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    use crate::core::{App, Point};
    use android_activity::{
        input::InputEvent, input::MotionAction, InputStatus, MainEvent, PollEvent,
    };
    use std::time::Duration;

    let mut width = 420;
    let mut height = 760;

    if let Some(window) = app.native_window() {
        width = window.width() as usize; // Screen width in pixels
        height = window.height() as usize; // Screen height in pixels
    }

    let mut launcher = App::new(width, height);
    let mut running = true;
    let mut needs_redraw = true;

    // Android app in action
    while running {
        app.poll_events(Some(Duration::from_millis(16)), |event| match event {
            PollEvent::Wake | PollEvent::Timeout => {
                if needs_redraw {
                    render(&app, &launcher);
                    needs_redraw = false;
                }
            }
            PollEvent::Main(main_event) => match main_event {
                MainEvent::InitWindow { .. }
                | MainEvent::WindowResized { .. }
                | MainEvent::ContentRectChanged { .. }
                | MainEvent::RedrawNeeded { .. } => {
                    sync_layout_to_window(&app, &mut launcher);
                    render(&app, &launcher);
                    needs_redraw = false;
                }
                MainEvent::InputAvailable => {
                    if let Ok(mut iter) = app.input_events_iter() {
                        loop {
                            let had_event = iter.next(|input_event| match input_event {
                                InputEvent::MotionEvent(motion_event) => {
                                    let pointer =
                                        motion_event.pointer_at_index(motion_event.pointer_index());
                                    let point = Point {
                                        x: pointer.raw_x(),
                                        y: pointer.raw_y(),
                                    };

                                    match motion_event.action() {
                                        MotionAction::Down
                                        | MotionAction::Move
                                        | MotionAction::PointerDown => {
                                            launcher.pointer_moved(point);
                                            needs_redraw = true;
                                            InputStatus::Handled
                                        }
                                        MotionAction::Up | MotionAction::PointerUp => {
                                            launcher.pointer_moved(point);
                                            if let Some(action) = launcher.click(point) {
                                                let _ = launch_action(action);
                                            }
                                            needs_redraw = true;
                                            InputStatus::Handled
                                        }
                                        MotionAction::Cancel => {
                                            needs_redraw = true;
                                            InputStatus::Handled
                                        }
                                        _ => InputStatus::Unhandled,
                                    }
                                }
                                _ => InputStatus::Unhandled,
                            });

                            if !had_event {
                                break;
                            }
                        }
                    }
                }
                MainEvent::TerminateWindow { .. } => {
                    needs_redraw = false;
                }
                MainEvent::Destroy => {
                    running = false;
                }
                _ => {}
            },
            _ => {}
        });
    }
}

#[cfg(not(target_os = "android"))]
pub fn launch_action(_action: crate::core::app::Action) -> Result<(), String> {
    Err("android launcher is only available on Android".to_string())
}

#[cfg(target_os = "android")]
fn sync_layout_to_window(app: &android_activity::AndroidApp, launcher: &mut crate::core::App) {
    use ndk::hardware_buffer_format::HardwareBufferFormat;

    if let Some(window) = app.native_window() {
        let width = window.width().max(1);
        let height = window.height().max(1);

        let (safe_area_top, safe_area_bottom) = crate::android::intent::get_safe_area(app)
            .map(|(top, bottom)| (top as f32, bottom as f32))
            .unwrap_or_else(|| {
                let content_rect = app.content_rect();
                (
                    content_rect.top.max(0) as f32,
                    (height - content_rect.bottom).max(0) as f32,
                )
            });

        launcher.set_safe_area(safe_area_top, safe_area_bottom);

        let _ = window.set_buffers_geometry(width, height, Some(HardwareBufferFormat::R8G8B8A8_UNORM));
        launcher.relayout(width as usize, height as usize);
    }
}

#[cfg(target_os = "android")]
fn render(app: &android_activity::AndroidApp, launcher: &crate::core::App) {
    use crate::core::style::{BACKGROUND, CARD_RADIUS, SHADOW_OFFSET_Y, SHADOW_SPREAD};

    let Some(window) = app.native_window() else {
        return;
    };

    let Ok(mut buffer) = window.lock(None) else {
        return;
    };

    let width = buffer.width();
    let height = buffer.height();
    let stride = buffer.stride();
    let Some(bytes_per_pixel) = buffer.format().bytes_per_pixel() else {
        return;
    };
    if bytes_per_pixel != 4 {
        return;
    }

    let Some(bytes) = buffer.bytes() else {
        return;
    };

    clear(bytes, stride, height, BACKGROUND);
    draw_header(bytes, stride, width, height, launcher.safe_area_top());
    for button in launcher.buttons() {
        dbg!("button: ");
        dbg!(&button);
        let (fill, shadow) = button_colors(button.id, launcher.hovered() == Some(button.id));
        let shadow_rect = crate::core::Rect {
            x: button.rect.x,
            y: button.rect.y + SHADOW_OFFSET_Y,
            width: button.rect.width,
            height: button.rect.height + SHADOW_SPREAD,
        };

        fill_rounded_rect(
            bytes,
            stride,
            width,
            height,
            shadow_rect,
            CARD_RADIUS,
            shadow,
        );
        fill_rounded_rect(bytes, stride, width, height, button.rect, CARD_RADIUS, fill);
        draw_button_contents(bytes, stride, width, height, button.id, button.rect);
    }
}

#[cfg(target_os = "android")]
fn draw_header(
    bytes: &mut [std::mem::MaybeUninit<u8>],
    stride: usize,
    width: usize,
    height: usize,
    safe_area_top: f32,
) {
    use crate::core::style::{BUTTON_MUTED, BUTTON_TEXT, HEADER_SURFACE, PANEL_PADDING};

    let hero = crate::core::Rect {
        x: PANEL_PADDING,
        y: safe_area_top + PANEL_PADDING,
        width: (width as f32 - PANEL_PADDING * 2.0).max(0.0),
        height: 68.0,
    };
    fill_rounded_rect(bytes, stride, width, height, hero, 24.0, HEADER_SURFACE);

    let accent = crate::core::Rect {
        x: hero.x + 18.0,
        y: hero.y + 18.0,
        width: 84.0,
        height: 8.0,
    };
    fill_rounded_rect(bytes, stride, width, height, accent, 4.0, BUTTON_TEXT);

    let subline = crate::core::Rect {
        x: hero.x + 18.0,
        y: hero.y + 34.0,
        width: 140.0,
        height: 6.0,
    };
    fill_rounded_rect(bytes, stride, width, height, subline, 3.0, BUTTON_MUTED);
}

#[cfg(target_os = "android")]
fn clear(bytes: &mut [std::mem::MaybeUninit<u8>], stride: usize, height: usize, color: u32) {
    for y in 0..height {
        for x in 0..stride {
            write_pixel(bytes, stride, x, y, color);
        }
    }
}

#[cfg(target_os = "android")]
fn draw_button_contents(
    bytes: &mut [std::mem::MaybeUninit<u8>],
    stride: usize,
    width: usize,
    height: usize,
    button_id: crate::core::app::ButtonId,
    rect: crate::core::Rect,
) {
    use crate::core::style::{BUTTON_MUTED, BUTTON_TEXT, ICON_BOX_SIZE, ICON_SURFACE};

    let icon_rect = crate::core::Rect {
        x: rect.x + 14.0,
        y: rect.y + ((rect.height - ICON_BOX_SIZE) * 0.5),
        width: ICON_BOX_SIZE,
        height: ICON_BOX_SIZE,
    };
    fill_rounded_rect(bytes, stride, width, height, icon_rect, 16.0, ICON_SURFACE);
    draw_icon(bytes, stride, width, height, button_id, icon_rect);

    let text_left = icon_rect.x + icon_rect.width + 18.0;
    let title = crate::core::Rect {
        x: text_left,
        y: rect.y + 21.0,
        width: rect.width * 0.34,
        height: 8.0,
    };
    let detail = crate::core::Rect {
        x: text_left,
        y: title.y + 18.0,
        width: rect.width * 0.52,
        height: 6.0,
    };
    let detail_two = crate::core::Rect {
        x: text_left,
        y: detail.y + 14.0,
        width: rect.width * 0.44,
        height: 6.0,
    };
    fill_rounded_rect(bytes, stride, width, height, title, 4.0, BUTTON_TEXT);
    fill_rounded_rect(bytes, stride, width, height, detail, 3.0, BUTTON_MUTED);
    fill_rounded_rect(bytes, stride, width, height, detail_two, 3.0, BUTTON_MUTED);

    let pill = crate::core::Rect {
        x: rect.x + rect.width - 64.0,
        y: rect.y + (rect.height - 24.0) * 0.5,
        width: 36.0,
        height: 24.0,
    };
    fill_rounded_rect(bytes, stride, width, height, pill, 12.0, BUTTON_TEXT);
}

#[cfg(target_os = "android")]
fn button_colors(button_id: crate::core::app::ButtonId, hovered: bool) -> (u32, u32) {
    use crate::core::app::ButtonId;
    use crate::core::style::{
        CARD_CAMERA, CARD_CAMERA_HOVER, CARD_CONTACTS, CARD_CONTACTS_HOVER, CARD_SETTINGS,
        CARD_SETTINGS_HOVER, SHADOW,
    };

    match (button_id, hovered) {
        (ButtonId::Settings, false) => (CARD_SETTINGS, SHADOW),
        (ButtonId::Settings, true) => (CARD_SETTINGS_HOVER, SHADOW),
        (ButtonId::Contacts, false) => (CARD_CONTACTS, SHADOW),
        (ButtonId::Contacts, true) => (CARD_CONTACTS_HOVER, SHADOW),
        (ButtonId::Camera, false) => (CARD_CAMERA, SHADOW),
        (ButtonId::Camera, true) => (CARD_CAMERA_HOVER, SHADOW),
    }
}

#[cfg(target_os = "android")]
fn draw_icon(
    bytes: &mut [std::mem::MaybeUninit<u8>],
    stride: usize,
    width: usize,
    height: usize,
    button_id: crate::core::app::ButtonId,
    rect: crate::core::Rect,
) {
    use crate::core::app::ButtonId;
    use crate::core::style::{BUTTON_MUTED, BUTTON_TEXT, ICON_SURFACE};

    match button_id {
        ButtonId::Settings => {
            for (index, y) in [14.0_f32, 28.0, 42.0].into_iter().enumerate() {
                let track = crate::core::Rect {
                    x: rect.x + 10.0,
                    y: rect.y + y,
                    width: rect.width - 20.0,
                    height: 4.0,
                };
                fill_rounded_rect(bytes, stride, width, height, track, 2.0, BUTTON_MUTED);

                let knob = crate::core::Rect {
                    x: rect.x + if index % 2 == 0 { 16.0 } else { 30.0 },
                    y: track.y - 4.0,
                    width: 12.0,
                    height: 12.0,
                };
                fill_rounded_rect(bytes, stride, width, height, knob, 6.0, BUTTON_TEXT);
            }
        }
        ButtonId::Contacts => {
            fill_circle(
                bytes,
                stride,
                width,
                height,
                rect.x + 28.0,
                rect.y + 22.0,
                10.0,
                BUTTON_TEXT,
            );
            let body = crate::core::Rect {
                x: rect.x + 14.0,
                y: rect.y + 36.0,
                width: 28.0,
                height: 12.0,
            };
            fill_rounded_rect(bytes, stride, width, height, body, 6.0, BUTTON_TEXT);
        }
        ButtonId::Camera => {
            let body = crate::core::Rect {
                x: rect.x + 11.0,
                y: rect.y + 16.0,
                width: 34.0,
                height: 24.0,
            };
            fill_rounded_rect(bytes, stride, width, height, body, 8.0, BUTTON_TEXT);
            fill_circle(
                bytes,
                stride,
                width,
                height,
                rect.x + 28.0,
                rect.y + 28.0,
                7.0,
                ICON_SURFACE,
            );
            let flash = crate::core::Rect {
                x: rect.x + 18.0,
                y: rect.y + 12.0,
                width: 10.0,
                height: 6.0,
            };
            fill_rounded_rect(bytes, stride, width, height, flash, 3.0, BUTTON_TEXT);
        }
    }
}

#[cfg(target_os = "android")]
fn fill_rounded_rect(
    bytes: &mut [std::mem::MaybeUninit<u8>],
    stride: usize,
    width: usize,
    height: usize,
    rect: crate::core::Rect,
    radius: f32,
    color: u32,
) {
    let start_x = rect.x.max(0.0) as usize;
    let end_x = (rect.x + rect.width).min(width as f32) as usize;
    let start_y = rect.y.max(0.0) as usize;
    let end_y = (rect.y + rect.height).min(height as f32) as usize;

    for y in start_y..end_y {
        for x in start_x..end_x {
            if inside_rounded_rect(x as f32 + 0.5, y as f32 + 0.5, rect, radius) {
                write_pixel(bytes, stride, x, y, color);
            }
        }
    }
}

#[cfg(target_os = "android")]
fn fill_circle(
    bytes: &mut [std::mem::MaybeUninit<u8>],
    stride: usize,
    width: usize,
    height: usize,
    cx: f32,
    cy: f32,
    radius: f32,
    color: u32,
) {
    let rect = crate::core::Rect {
        x: cx - radius,
        y: cy - radius,
        width: radius * 2.0,
        height: radius * 2.0,
    };
    let start_x = rect.x.max(0.0) as usize;
    let end_x = (rect.x + rect.width).min(width as f32) as usize;
    let start_y = rect.y.max(0.0) as usize;
    let end_y = (rect.y + rect.height).min(height as f32) as usize;
    let radius_sq = radius * radius;

    for y in start_y..end_y {
        for x in start_x..end_x {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= radius_sq {
                write_pixel(bytes, stride, x, y, color);
            }
        }
    }
}

#[cfg(target_os = "android")]
fn inside_rounded_rect(px: f32, py: f32, rect: crate::core::Rect, radius: f32) -> bool {
    let left = rect.x;
    let right = rect.x + rect.width;
    let top = rect.y;
    let bottom = rect.y + rect.height;

    if px < left || px > right || py < top || py > bottom {
        return false;
    }

    let inner_left = left + radius;
    let inner_right = right - radius;
    let inner_top = top + radius;
    let inner_bottom = bottom - radius;

    if (px >= inner_left && px <= inner_right) || (py >= inner_top && py <= inner_bottom) {
        return true;
    }

    let cx = if px < inner_left {
        inner_left
    } else {
        inner_right
    };
    let cy = if py < inner_top {
        inner_top
    } else {
        inner_bottom
    };
    let dx = px - cx;
    let dy = py - cy;

    dx * dx + dy * dy <= radius * radius
}

#[cfg(target_os = "android")]
fn write_pixel(
    bytes: &mut [std::mem::MaybeUninit<u8>],
    stride: usize,
    x: usize,
    y: usize,
    color: u32,
) {
    let offset = ((y * stride) + x) * 4;
    if offset + 3 >= bytes.len() {
        return;
    }

    let red = ((color >> 16) & 0xff) as u8;
    let green = ((color >> 8) & 0xff) as u8;
    let blue = (color & 0xff) as u8;

    bytes[offset] = std::mem::MaybeUninit::new(red);
    bytes[offset + 1] = std::mem::MaybeUninit::new(green);
    bytes[offset + 2] = std::mem::MaybeUninit::new(blue);
    bytes[offset + 3] = std::mem::MaybeUninit::new(0xff);
}
