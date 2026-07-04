pub mod app;
pub mod input;
pub mod window;

use crate::core::GlowRenderer;

/// Run the desktop preview.
///
/// # Errors
///
/// Returns an error if SDL2 or GL initialization fails.
///
/// # Panics
///
/// Panics if mouse or drawable dimensions do not fit the narrow integer
/// conversions used for clippy-clean float handling.
#[allow(clippy::missing_panics_doc)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let (window, gl) = window::DesktopWindow::new()?;
    let font_bytes: &[u8] = include_bytes!("../../fonts/audiowide.ttf");
    let renderer = unsafe { GlowRenderer::with_font(gl, Some(font_bytes))? };

    // Initialize screen dimensions BEFORE AppState evaluates JS plugins
    // so that the initial UI layout has the correct vw/vh values.
    let (phys_w, phys_h) = window.window.drawable_size();
    crate::core::types::SCREEN_WIDTH.store(
        f32::from(u16::try_from(phys_w).unwrap_or(0)).to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );
    crate::core::types::SCREEN_HEIGHT.store(
        f32::from(u16::try_from(phys_h).unwrap_or(0)).to_bits(),
        std::sync::atomic::Ordering::Relaxed,
    );

    let app_state = app::AppState::new();

    app::run_loop(app_state, window, renderer).map_err(Into::into)
}
