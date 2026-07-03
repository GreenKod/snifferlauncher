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
            },
            {
                "Container": {
                    "id": "clip-container",
                    "style": {
                        "width": { "Pixels": 500.0 },
                        "height": { "Pixels": 300.0 },
                        "background_color": hexToColor("#FFFFFF"),
                        "overflow_hidden": true,
                        "border_radius": 12.0,
                        "margin": { "top": 20.0, "bottom": 20.0, "left": 0.0, "right": 0.0 }
                    },
                    "children": [
                        {
                            "Image": {
                                "id": "test_img",
                                "src": "assets/test.png", // TEST İÇİN: Buraya bir resim (PNG) koymalısınız
                                "style": {
                                    "width": { "Percent": 100.0 },
                                    "height": { "Percent": 100.0 },
                                    "border_radius": 12.0,
                                    "object_fit": "Cover"
                                }
                            }
                        }
                    ]
                }
            },
            {
                "TextInput": {
                    "id": "search_input",
                    "value": "Buraya tıkla ve yaz...",
                    "focused": false,
                    "style": {
                        "text_color": hexToColor("#0000FF"),
                        "text_size": 24.0,
                        "width": "Auto",
                        "height": "Auto",
                        "background_color": hexToColor("#E0E0E0"),
                        "border_radius": 8.0,
                        "padding": { "top": 10.0, "bottom": 10.0, "left": 10.0, "right": 10.0 }
                    }
                }
            }
        ]
    }
};

// Log hashes to console so we can debug which ID is which
host_log("Hash of 'root': " + host_hash("root"));
host_log("Hash of 'btn-merhaba': " + host_hash("btn-merhaba"));
host_log("Hash of 'search_input': " + host_hash("search_input"));

// Arka planda resmi GPU'ya yükle
host_create_image("test_img", "assets/test.png");

// Send UI to Rust Host
host_set_ui(JSON.stringify(UI_TREE));

// Phase 2: Binary ArrayBuffer Helpers (Postcard Format)
function parseAppState(buffer) {
    // AppState in Postcard:
    // click_count: unsigned varint
    // screen_width: f32 (4 bytes, Little-Endian)
    // screen_height: f32 (4 bytes, Little-Endian)
    const view = new DataView(buffer);
    let offset = 0;

    // Read varint for u32
    let click_count = 0;
    let shift = 0;
    while (true) {
        let byte = view.getUint8(offset++);
        click_count |= (byte & 0x7f) << shift;
        if ((byte & 0x80) === 0) break;
        shift += 7;
    }

    const screen_width = view.getFloat32(offset, true);
    offset += 4;
    const screen_height = view.getFloat32(offset, true);

    return { click_count, screen_width, screen_height };
}

// Postcard formatında (varint) Rust'a state göndermek için yardımcı fonksiyon
function sendBinaryState(clickCount, screenWidth, screenHeight) {
    // 1. clickCount değerini varint olarak kodla
    let varintBytes = [];
    let temp = clickCount;
    while (temp >= 0x80) {
        varintBytes.push((temp & 0x7F) | 0x80);
        temp >>>= 7;
    }
    varintBytes.push(temp);

    // 2. Buffer oluştur (varint uzunluğu + 8 byte f32)
    const buffer = new ArrayBuffer(varintBytes.length + 8);
    const view = new DataView(buffer);

    // 3. Verileri yaz
    let offset = 0;
    for (let i = 0; i < varintBytes.length; i++) {
        view.setUint8(offset++, varintBytes[i]);
    }

    view.setFloat32(offset, screenWidth, true); // Little-endian
    offset += 4;
    view.setFloat32(offset, screenHeight, true); // Little-endian

    // Rust tarafındaki eklenti hostuna gönder
    host_send_binary_event(buffer);
}

let click_counter = 0;

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
        click_counter += 1;

        // Phase 2 (ArrayBuffer / Postcard) Test
        // Normalde Rust'tan da host_get_binary_state() ile çekebilirsiniz 
        // ancak biz burada kendi click_counter değerimizi kullanıyoruz:
        let buffer = host_get_binary_state();
        let state = parseAppState(buffer);

        let new_text = "Binary: " + state.screen_width + "x" + state.screen_height + " Tıklama: " + click_counter;
        host_set_text("btn-width", new_text);

        UI_TREE.Container.children[1].Label.text = new_text;
        host_set_ui(JSON.stringify(UI_TREE));

        // Rust'a binary formatında varint gönder
        sendBinaryState(click_counter, state.screen_width, state.screen_height);
    } else if (event.type === "Click" && event.id === host_hash("search_input")) {
        // TextInput Focus Test
        host_focus_input("search_input");
        UI_TREE.Container.children[3].TextInput.focused = true;

        // İlk tıklamada içeriği temizle
        if (UI_TREE.Container.children[3].TextInput.value === "Buraya tıkla ve yaz...") {
            UI_TREE.Container.children[3].TextInput.value = "";
        }
        host_set_ui(JSON.stringify(UI_TREE));
    } else if (event.type === "ClickOutside" || event.type === "Click") {
        // Başka bir yere tıklanırsa blur yap
        UI_TREE.Container.children[3].TextInput.focused = false;
        host_set_ui(JSON.stringify(UI_TREE));
        host_blur_input();
    }

    if (event.type === "TextInput") {
        // Gelen karakterleri arama kutusuna ekle
        UI_TREE.Container.children[3].TextInput.value += event.text;
        host_set_text("search_input", UI_TREE.Container.children[3].TextInput.value);
    } else if (event.type === "Backspace") {
        let val = UI_TREE.Container.children[3].TextInput.value;
        if (val.length > 0) {
            UI_TREE.Container.children[3].TextInput.value = val.substring(0, val.length - 1);
            host_set_text("search_input", UI_TREE.Container.children[3].TextInput.value);
        }
    }

    return "[]";
};
