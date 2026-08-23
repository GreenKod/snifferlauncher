/// Discovers available system fonts and color emoji fonts on the host platform.
#[must_use]
pub fn discover_system_fonts() -> Vec<Vec<u8>> {
    let mut font_data_list = Vec::new();

    #[cfg(target_os = "android")]
    let candidate_paths = [
        "/system/fonts/Roboto-Regular.ttf",
        "/system/fonts/NotoSans-Regular.ttf",
        "/system/fonts/NotoSansCJK-Regular.ttc",
        "/system/fonts/NotoColorEmoji.ttf",
        "/system/fonts/GoogleColorEmoji.ttf",
    ];

    #[cfg(target_os = "windows")]
    let candidate_paths = [
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\seguiemj.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];

    #[cfg(not(any(target_os = "android", target_os = "windows")))]
    let candidate_paths = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/noto/NotoColorEmoji.ttf",
        "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
    ];

    for path in candidate_paths {
        if let Ok(bytes) = std::fs::read(path) {
            if !bytes.is_empty() {
                font_data_list.push(bytes);
            }
        }
    }

    font_data_list
}
