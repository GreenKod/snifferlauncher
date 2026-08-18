pub mod app;
pub mod input;
pub mod window;

use sniffer_render::GlowRenderer;

struct DesktopHostBridge;

impl sniffer_plugin::js::host_bridge::HostPlatformBridge for DesktopHostBridge {
    fn get_application_list(&self) -> Result<Vec<sniffer_core::types::AppInfo>, String> {
        get_application_list()
    }
}

/// Run the desktop preview.
#[allow(clippy::missing_panics_doc)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    sniffer_plugin::js::host_bridge::set_host_bridge(Box::new(DesktopHostBridge));

    let (event_loop, window, gl) = window::DesktopWindow::new()?;
    let font_bytes: &[u8] = include_bytes!("../fonts/audiowide.ttf");
    let renderer = unsafe { GlowRenderer::with_font(gl, Some(font_bytes))? };

    // Initialize screen dimensions BEFORE AppState evaluates JS plugins
    // so that the initial UI layout has the correct vw/vh values.
    let (phys_w, phys_h) = window.drawable_size();
    sniffer_core::types::SCREEN_WIDTH.store(
        f32::from(u16::try_from(phys_w).unwrap_or(0)).to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
    sniffer_core::types::SCREEN_HEIGHT.store(
        f32::from(u16::try_from(phys_h).unwrap_or(0)).to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );

    let app_state = app::AppState::new();

    app::run_loop(app_state, event_loop, window, renderer).map_err(Into::into)
}

/// Returns a mocked list of applications for the desktop environment.
pub fn get_application_list() -> Result<Vec<sniffer_core::types::AppInfo>, String> {
    use sniffer_core::types::AppInfo;

    let mock_apps = vec![
        ("Browser", "com.desktop.browser"),
        ("Calculator", "com.desktop.calculator"),
        ("Camera", "com.desktop.camera"),
        ("Contacts", "com.desktop.contacts"),
        ("Files", "com.desktop.files"),
        ("Settings", "com.desktop.settings"),
        ("Phone", "com.desktop.phone"),
        ("Messages", "com.desktop.messages"),
        ("Calendar", "com.desktop.calendar"),
        ("Music", "com.desktop.music"),
    ];

    let mut apps = Vec::new();
    for (name, pkg) in mock_apps {
        apps.push(AppInfo::from_package_info(
            name.to_string(),
            pkg.to_string(),
        ));
    }

    Ok(apps)
}
