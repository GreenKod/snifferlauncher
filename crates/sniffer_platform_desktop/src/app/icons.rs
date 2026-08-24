use sniffer_core::render::Renderer;
use sniffer_render::GlowRenderer;

fn generate_icon_pixels(base_r: u8, base_g: u8, base_b: u8) -> Vec<u8> {
    const SIZE: usize = 64;
    let mut pixels = vec![0u8; SIZE * SIZE * 4];

    for y in 0..SIZE {
        for x in 0..SIZE {
            let px = x as f32;
            let py = y as f32;
            let idx = (y * SIZE + x) * 4;

            // Rounded box SDF
            let cx = (px - 31.5).abs();
            let cy = (py - 31.5).abs();
            let qx = (cx - 20.0).max(0.0);
            let qy = (cy - 20.0).max(0.0);
            let dist = qx.hypot(qy) - 10.0;

            if dist > 0.0 {
                // Outside rounded corner: transparent
                pixels[idx] = 0;
                pixels[idx + 1] = 0;
                pixels[idx + 2] = 0;
                pixels[idx + 3] = 0;
            } else {
                // Inside icon background with vertical gradient
                let t = py / 64.0;
                let r = (f32::from(base_r) * (1.0 - t * 0.25)) as u8;
                let g = (f32::from(base_g) * (1.0 - t * 0.25)) as u8;
                let b = (f32::from(base_b) * (1.0 - t * 0.25)) as u8;

                // Center emblem (inner circle + inner dot)
                let center_dist = (px - 31.5).hypot(py - 31.5);
                if (10.0..14.0).contains(&center_dist) || center_dist < 5.0 {
                    pixels[idx] = 255;
                    pixels[idx + 1] = 255;
                    pixels[idx + 2] = 255;
                    pixels[idx + 3] = 240;
                } else {
                    pixels[idx] = r;
                    pixels[idx + 1] = g;
                    pixels[idx + 2] = b;
                    pixels[idx + 3] = 255;
                }
            }
        }
    }
    pixels
}

pub fn preload_desktop_icons(renderer: &mut GlowRenderer) {
    let mock_apps = [
        ("com.desktop.browser", (56, 189, 248)),
        ("com.android.chrome", (56, 189, 248)),
        ("com.desktop.calculator", (249, 115, 22)),
        ("com.android.calculator2", (249, 115, 22)),
        ("com.desktop.camera", (236, 72, 153)),
        ("com.android.camera", (236, 72, 153)),
        ("com.desktop.contacts", (14, 165, 233)),
        ("com.android.contacts", (14, 165, 233)),
        ("com.desktop.files", (234, 179, 8)),
        ("com.android.documentsui", (234, 179, 8)),
        ("com.desktop.settings", (100, 116, 139)),
        ("com.android.settings", (100, 116, 139)),
        ("com.desktop.phone", (34, 197, 94)),
        ("com.android.dialer", (34, 197, 94)),
        ("com.desktop.messages", (59, 130, 246)),
        ("com.android.mms", (59, 130, 246)),
        ("com.desktop.calendar", (239, 68, 68)),
        ("com.desktop.music", (168, 85, 247)),
    ];

    for (pkg, (r, g, b)) in mock_apps {
        let pixels = generate_icon_pixels(r, g, b);
        let uri = format!("app-icon://{pkg}");
        renderer.load_image(&uri, &pixels, 64, 64);
        renderer.load_image(pkg, &pixels, 64, 64);
    }
}
