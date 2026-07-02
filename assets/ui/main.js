// main.js - Declarative UI in Vanilla JS for SnifferLauncher"

// Helper to convert #RRGGBB to Rust's u32 color
function hexToColor(hex) {
    hex = hex.replace(/^#/, '');
    if (hex.length === 6) hex = "FF" + hex; // Add alpha if missing
    return parseInt(hex, 16);
}

function getScreenSize() {
    return {
        "width": host_screen_width(),
        "height": host_screen_height()
    };
}

const UI_TREE = {
    "Container": {
        "id": "root",
        "style": {
            "display": "Flex",
            "flex_direction": "Column",
            "justify_content": "Center",
            "align_items": "Center",
            "width": { "Percent": 100.0 },
            "height": { "Percent": 100.0 }, // Ekranın %100'ünü kaplar
            "background_color": hexToColor("#FFD700") // Yellow BG
        },
        "children": [
            {
                "Label": {
                    "id": "btn-merhaba",
                    "text": "merhaba",
                    "style": {
                        "text_color": hexToColor("#000000"), // Black Text
                        "text_size": 48.0,
                        "width": { "Pixels": 220.0 } // Fixed width so it centers correctly
                    }
                }
            },
            {
                "Label": {
                    "id": "btn-width",
                    "text": "Genişliği Göster",
                    "style": {
                        "text_color": hexToColor("#000000"), // Black Text
                        "text_size": 32.0,
                        "width": { "Pixels": 300.0 }
                    }
                }
            }
        ]
    }
};

// Log hashes to console so we can debug which ID is which
host_log("Hash of 'root': " + host_hash("root"));
host_log("Hash of 'btn-merhaba': " + host_hash("btn-merhaba"));

// Send UI to Rust Host
host_set_ui(JSON.stringify(UI_TREE));

// Phase 2: Binary ArrayBuffer Helpers
function parseAppState(buffer) {
    // AppState is 12 bytes (u32, f32, f32).
    // Bincode default is Little-Endian.
    const view = new DataView(buffer);
    const click_count = view.getUint32(0, true);
    const screen_width = view.getFloat32(4, true);
    const screen_height = view.getFloat32(8, true);
    return { click_count, screen_width, screen_height };
}

function sendBinaryEvent() {
    const buffer = new ArrayBuffer(12);
    const view = new DataView(buffer);
    view.setUint32(0, 999, true); // send fake click_count
    view.setFloat32(4, 1280.0, true);
    view.setFloat32(8, 720.0, true);
    host_send_binary_event(buffer);
}

// Event Handler for UI Interactions
globalThis.onEvent = function (eventJsonString) {
    const event = JSON.parse(eventJsonString);

    if (event.type === "Click") {
        host_log("JS received CLICK event for ID: " + event.id);
    }

    if (event.type === "Click" && event.id === host_hash("btn-merhaba")) {
        // Phase 1 (Fine-Grained API) Test
        host_set_text("btn-merhaba", "Tıklandı!");
        host_update_style("root", "background_color", "FF00FF00"); // Green
        
        UI_TREE.Container.children[0].Label.text = "Tıklandı!";
        UI_TREE.Container.style.background_color = hexToColor("#00FF00"); 

    } else if (event.type === "Click" && event.id === host_hash("btn-width")) {
        // Phase 2 (ArrayBuffer / Bincode) Test
        let buffer = host_get_binary_state();
        let state = parseAppState(buffer);
        
        let new_text = "Binary: " + state.screen_width + "x" + state.screen_height + " Clicks: " + state.click_count;
        host_set_text("btn-width", new_text);
        
        // Also test sending binary back to Rust
        sendBinaryEvent();
        
        UI_TREE.Container.children[1].Label.text = new_text;
    }

    return "[]";
};
