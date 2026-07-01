// main.js - Declarative UI in Vanilla JS for SnifferLauncher

// Helper to convert #RRGGBB to Rust's u32 color
function hexToColor(hex) {
    hex = hex.replace(/^#/, '');
    if (hex.length === 6) hex = "FF" + hex; // Add alpha if missing
    return parseInt(hex, 16);
}

const UI_TREE = {
    "Container": {
        "id": "root",
        "style": {
            "display": "Flex",
            "flex_direction": "Column",
            "justify_content": "Center",
            "align_items": "Center",
            "width": "Auto",
            "height": "Auto",
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
            }
        ]
    }
};

// Log hashes to console so we can debug which ID is which
host_log("Hash of 'root': " + host_hash("root"));
host_log("Hash of 'btn-merhaba': " + host_hash("btn-merhaba"));

// Send UI to Rust Host
host_set_ui(JSON.stringify(UI_TREE));

// Event Handler for UI Interactions
globalThis.onEvent = function (eventJsonString) {
    const event = JSON.parse(eventJsonString);

    // Only log Click events to avoid spamming the console with Hovers
    if (event.type === "Click") {
        host_log("JS received CLICK event for ID: " + event.id);
    }

    // host_hash converts the string "btn-merhaba" to its matching u64 Hash ID (as a string)
    if (event.type === "Click" && event.id === host_hash("btn-merhaba")) {
        // Change the text and background
        UI_TREE.children[0].Label.text = "Tıklandı!";
        UI_TREE.style.background_color = hexToColor("#00FF00"); // Green

        // Send updated UI to Rust
        host_set_ui(JSON.stringify(UI_TREE));
    }

    return "[]";
};
