use crate::core::app::{Action, App, ButtonId};
use crate::core::geometry::{Point, Rect};
use crate::core::style::{
    BACKGROUND, BUTTON_MUTED, BUTTON_TEXT, CARD_CAMERA, CARD_CAMERA_HOVER,
    CARD_CONTACTS, CARD_CONTACTS_HOVER, CARD_RADIUS, CARD_SETTINGS, CARD_SETTINGS_HOVER,
    HEADER_SURFACE, ICON_BOX_SIZE, ICON_SURFACE, PANEL_PADDING, SHADOW, SHADOW_OFFSET_Y,
    SHADOW_SPREAD, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use minifb::{Key, MouseButton, MouseMode, Scale, ScaleMode, Window, WindowOptions};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new(
        "Launcher Template",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions {
            resize: true,
            scale: Scale::X1,
            scale_mode: ScaleMode::Stretch,
            ..WindowOptions::default()
        },
    )?;

    let mut width = WINDOW_WIDTH;
    let mut height = WINDOW_HEIGHT;
    let mut app = App::new(width, height);
    let mut buffer = vec![BACKGROUND; width * height];
    let mut last_mouse_down = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (new_width, new_height) = window.get_size();
        if new_width != width || new_height != height {
            width = new_width.max(1);
            height = new_height.max(1);
            app.relayout(width, height);
            buffer.resize(width * height, BACKGROUND);
        }

        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Clamp) {
            let point = Point { x, y };
            app.pointer_moved(point);

            let mouse_down = window.get_mouse_down(MouseButton::Left);
            if mouse_down && !last_mouse_down {
                if let Some(action) = app.click(point) {
                    handle_action(action);
                }
            }
            last_mouse_down = mouse_down;
        }

        draw(&mut buffer, width, height, &app);
        window.update_with_buffer(&buffer, width, height)?;
    }

    Ok(())
}

fn handle_action(action: Action) {
    match action {
        Action::OpenSettings => println!("desktop preview: open settings"),
        Action::OpenContacts => println!("desktop preview: open contacts"),
        Action::OpenCamera => println!("desktop preview: open camera"),
    }
}

fn draw(buffer: &mut [u32], width: usize, height: usize, app: &App) {
    buffer.fill(BACKGROUND);
    draw_header(buffer, width, height);

    for button in app.buttons() {
        let (fill, shadow) = button_colors(button.id, app.hovered() == Some(button.id));
        let shadow_rect = Rect {
            x: button.rect.x,
            y: button.rect.y + SHADOW_OFFSET_Y,
            width: button.rect.width,
            height: button.rect.height + SHADOW_SPREAD,
        };

        fill_rounded_rect(buffer, width, height, shadow_rect, CARD_RADIUS, shadow);
        fill_rounded_rect(buffer, width, height, button.rect, CARD_RADIUS, fill);
        draw_button_contents(buffer, width, height, button.id, button.rect);
    }
}

fn draw_header(buffer: &mut [u32], width: usize, height: usize) {
    let hero = Rect {
        x: PANEL_PADDING,
        y: PANEL_PADDING,
        width: (width as f32 - PANEL_PADDING * 2.0).max(0.0),
        height: 68.0,
    };
    fill_rounded_rect(buffer, width, height, hero, 24.0, HEADER_SURFACE);

    let accent = Rect {
        x: hero.x + 18.0,
        y: hero.y + 18.0,
        width: 84.0,
        height: 8.0,
    };
    fill_rounded_rect(buffer, width, height, accent, 4.0, BUTTON_TEXT);

    let subline = Rect {
        x: hero.x + 18.0,
        y: hero.y + 34.0,
        width: 140.0,
        height: 6.0,
    };
    fill_rounded_rect(buffer, width, height, subline, 3.0, BUTTON_MUTED);
}

fn draw_button_contents(buffer: &mut [u32], width: usize, height: usize, button_id: ButtonId, rect: Rect) {
    let icon_rect = Rect {
        x: rect.x + 14.0,
        y: rect.y + ((rect.height - ICON_BOX_SIZE) * 0.5),
        width: ICON_BOX_SIZE,
        height: ICON_BOX_SIZE,
    };
    fill_rounded_rect(buffer, width, height, icon_rect, 16.0, ICON_SURFACE);
    draw_icon(buffer, width, height, button_id, icon_rect);

    let text_left = icon_rect.x + icon_rect.width + 18.0;
    let title = Rect {
        x: text_left,
        y: rect.y + 21.0,
        width: rect.width * 0.34,
        height: 8.0,
    };
    let detail = Rect {
        x: text_left,
        y: title.y + 18.0,
        width: rect.width * 0.52,
        height: 6.0,
    };
    let detail_two = Rect {
        x: text_left,
        y: detail.y + 14.0,
        width: rect.width * 0.44,
        height: 6.0,
    };
    fill_rounded_rect(buffer, width, height, title, 4.0, BUTTON_TEXT);
    fill_rounded_rect(buffer, width, height, detail, 3.0, BUTTON_MUTED);
    fill_rounded_rect(buffer, width, height, detail_two, 3.0, BUTTON_MUTED);

    let pill = Rect {
        x: rect.x + rect.width - 64.0,
        y: rect.y + (rect.height - 24.0) * 0.5,
        width: 36.0,
        height: 24.0,
    };
    fill_rounded_rect(buffer, width, height, pill, 12.0, BUTTON_TEXT);
}

fn button_colors(button_id: ButtonId, hovered: bool) -> (u32, u32) {
    match (button_id, hovered) {
        (ButtonId::Settings, false) => (CARD_SETTINGS, SHADOW),
        (ButtonId::Settings, true) => (CARD_SETTINGS_HOVER, SHADOW),
        (ButtonId::Contacts, false) => (CARD_CONTACTS, SHADOW),
        (ButtonId::Contacts, true) => (CARD_CONTACTS_HOVER, SHADOW),
        (ButtonId::Camera, false) => (CARD_CAMERA, SHADOW),
        (ButtonId::Camera, true) => (CARD_CAMERA_HOVER, SHADOW),
    }
}

fn draw_icon(buffer: &mut [u32], width: usize, height: usize, button_id: ButtonId, rect: Rect) {
    match button_id {
        ButtonId::Settings => {
            for (index, y) in [14.0_f32, 28.0, 42.0].into_iter().enumerate() {
                let track = Rect {
                    x: rect.x + 10.0,
                    y: rect.y + y,
                    width: rect.width - 20.0,
                    height: 4.0,
                };
                fill_rounded_rect(buffer, width, height, track, 2.0, BUTTON_MUTED);

                let knob = Rect {
                    x: rect.x + if index % 2 == 0 { 16.0 } else { 30.0 },
                    y: track.y - 4.0,
                    width: 12.0,
                    height: 12.0,
                };
                fill_rounded_rect(buffer, width, height, knob, 6.0, BUTTON_TEXT);
            }
        }
        ButtonId::Contacts => {
            fill_circle(buffer, width, height, rect.x + 28.0, rect.y + 22.0, 10.0, BUTTON_TEXT);
            let body = Rect {
                x: rect.x + 14.0,
                y: rect.y + 36.0,
                width: 28.0,
                height: 12.0,
            };
            fill_rounded_rect(buffer, width, height, body, 6.0, BUTTON_TEXT);
        }
        ButtonId::Camera => {
            let body = Rect {
                x: rect.x + 11.0,
                y: rect.y + 16.0,
                width: 34.0,
                height: 24.0,
            };
            fill_rounded_rect(buffer, width, height, body, 8.0, BUTTON_TEXT);
            fill_circle(buffer, width, height, rect.x + 28.0, rect.y + 28.0, 7.0, ICON_SURFACE);
            let flash = Rect {
                x: rect.x + 18.0,
                y: rect.y + 12.0,
                width: 10.0,
                height: 6.0,
            };
            fill_rounded_rect(buffer, width, height, flash, 3.0, BUTTON_TEXT);
        }
    }
}

fn fill_rounded_rect(buffer: &mut [u32], width: usize, height: usize, rect: Rect, radius: f32, color: u32) {
    let start_x = rect.x.max(0.0) as usize;
    let end_x = (rect.x + rect.width).min(width as f32) as usize;
    let start_y = rect.y.max(0.0) as usize;
    let end_y = (rect.y + rect.height).min(height as f32) as usize;

    for y in start_y..end_y {
        let row_start = y * width;
        for x in start_x..end_x {
            if inside_rounded_rect(x as f32 + 0.5, y as f32 + 0.5, rect, radius) {
                buffer[row_start + x] = color;
            }
        }
    }
}

fn fill_circle(buffer: &mut [u32], width: usize, height: usize, cx: f32, cy: f32, radius: f32, color: u32) {
    let rect = Rect {
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
        let row_start = y * width;
        for x in start_x..end_x {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= radius_sq {
                buffer[row_start + x] = color;
            }
        }
    }
}

fn inside_rounded_rect(px: f32, py: f32, rect: Rect, radius: f32) -> bool {
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

    let cx = if px < inner_left { inner_left } else { inner_right };
    let cy = if py < inner_top { inner_top } else { inner_bottom };
    let dx = px - cx;
    let dy = py - cy;

    dx * dx + dy * dy <= radius * radius
}
