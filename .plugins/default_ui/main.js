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
            "background_color": hexToColor("#FFD700"), // Yellow BG
            "gap": 20.0 // Flexbox Gap özelliği! Öğeler arasına 20 piksel boşluk koyar
        },
        "children": [
            {
                "Label": {
                    "id": "btn-merhaba",
                    "text": "merhaba",
                    "style": {
                        "text_color": hexToColor("#000000"), // Black Text
                        "text_size": 48.0,
                        "width": "Auto" // Metin genişliğine göre otomatik ayarlanır
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
                        "width": "Auto"
                    }
                }
            },
            {
                "ScrollView": {
                    "id": "scroll-container",
                    "scroll_x": 0.0,
                    "scroll_y": 0.0,
                    "scroll_sensitivity": 1.0,
                    "dynamic_sensitivity": true,
                    "momentum_scrolling": true,
                    "capture_drag": true,
                    "style": {
                        "width": { "Percent": 90.0 }, // Ekranın genişliğinin %90'ını kaplar (Responsive)
                        "height": { "Percent": 50.0 }, // Ekran yüksekliğinin %50'sini kaplar (Responsive)
                        "background_color": hexToColor("#FFFFFF"),
                        "overflow_hidden": true,
                        "border_radius": 12.0,
                        "margin": { "top": 0.0, "bottom": 0.0, "left": 0.0, "right": 0.0 },
                        "gap": 10.0 // ScrollView içindeki liste elemanları arasına 10px boşluk
                    },
                    "children": [
                        {
                            "Image": {
                                "id": "test_img",
                                "src": ".plugins/default_ui/test.jpg",
                                "style": {
                                    "width": { "Percent": 100.0 },
                                    "height": { "Percent": 40.0 }, // Scroll container'ın %40'ı kadar yükseklik
                                    "border_radius": 12.0,
                                    "object_fit": "Cover"
                                }
                            }
                        },
                        {
                            "Label": {
                                "id": "scroll-item-1",
                                "text": "Kaydırılabilir Öğe 1",
                                "style": { "text_size": 32.0, "text_color": hexToColor("#000000"), "padding": { "top": 10.0, "bottom": 10.0, "left": 10.0, "right": 10.0 }, "height": "Auto" }
                            }
                        },
                        {
                            "Label": {
                                "id": "scroll-item-2",
                                "text": "Kaydırılabilir Öğe 2",
                                "style": { "text_size": 32.0, "text_color": hexToColor("#000000"), "padding": { "top": 10.0, "bottom": 10.0, "left": 10.0, "right": 10.0 }, "height": "Auto" }
                            }
                        },
                        {
                            "Label": {
                                "id": "scroll-item-3",
                                "text": "Kaydırılabilir Öğe 3",
                                "style": { "text_size": 32.0, "text_color": hexToColor("#000000"), "padding": { "top": 10.0, "bottom": 10.0, "left": 10.0, "right": 10.0 }, "height": "Auto" }
                            }
                        },
                        {
                            "Label": {
                                "id": "scroll-item-4",
                                "text": "Kaydırılabilir Öğe 4",
                                "style": { "text_size": 32.0, "text_color": hexToColor("#000000"), "padding": { "top": 10.0, "bottom": 10.0, "left": 10.0, "right": 10.0 }, "height": "Auto" }
                            }
                        },
                        {
                            "Label": {
                                "id": "scroll-item-5",
                                "text": "Kaydırılabilir Öğe 5",
                                "style": { "text_size": 32.0, "text_color": hexToColor("#000000"), "padding": { "top": 10.0, "bottom": 10.0, "left": 10.0, "right": 10.0 }, "height": "Auto" }
                            }
                        },
                        {
                            "Label": {
                                "id": "scroll-item-6",
                                "text": "Kaydırılabilir Öğe 6",
                                "style": { "text_size": 32.0, "text_color": hexToColor("#000000"), "padding": { "top": 10.0, "bottom": 10.0, "left": 10.0, "right": 10.0 }, "height": "Auto" }
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
                        "width": { "Percent": 80.0 }, // Genişliği ekranın %80'i yap (Responsive)
                        "height": "Auto",
                        "background_color": hexToColor("#E0E0E0"),
                        "border_radius": 8.0,
                        "padding": { "top": 10.0, "bottom": 10.0, "left": 10.0, "right": 10.0 }
                    }
                }
            },
            {
                "Container": {
                    "id": "fab_button",
                    "style": {
                        "position": "Absolute",
                        "bottom": { "Pixels": 30.0 },
                        "right": { "Pixels": 30.0 },
                        "width": { "Pixels": 60.0 },
                        "height": { "Pixels": 60.0 },
                        "border_radius": 30.0,
                        "background_gradient": [hexToColor("#FF0055"), hexToColor("#FF00AA")],
                        "shadow_color": hexToColor("#88000000"),
                        "shadow_spread": 4.0,
                        "shadow_offset_y": 4.0,
                        "opacity": 0.9,
                        "justify_content": "Center",
                        "align_items": "Center"
                    },
                    "children": [
                        {
                            "Label": {
                                "id": "fab_text",
                                "text": "+",
                                "style": {
                                    "text_color": hexToColor("#FFFFFF"),
                                    "text_size": 36.0
                                }
                            }
                        }
                    ]
                }
            }
        ]
    }
};

// Log hashes to console so we can debug which ID is which
host_log("Hash of 'root': " + host_hash("root"));
host_log("Hash of 'btn-merhaba': " + host_hash("btn-merhaba"));
host_log("Hash of 'scroll-container': " + host_hash("scroll-container"));
host_log("Hash of 'search_input': " + host_hash("search_input"));

// Arka planda resmi GPU'ya yükle
host_create_image("test_img", ".plugins/default_ui/test.jpg");

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
    } else if (event.type === "PointerDown" && (String(event.id) === host_hash("fab_button") || String(event.id) === host_hash("fab_text"))) {
        UI_TREE.Container.children[4].Container.style.opacity = 0.5;
        host_set_ui(JSON.stringify(UI_TREE));
        return "[]";
    } else if (event.type === "PointerUp" || event.type === "ClickOutside") {
        if (UI_TREE.Container.children[4].Container.style.opacity === 0.5) {
            UI_TREE.Container.children[4].Container.style.opacity = 0.9;

            // Eğer PointerUp event ise ve FAB butonuna/metnine aitse yeni öğe ekle
            if (event.type === "PointerUp" && (String(event.id) === host_hash("fab_button") || String(event.id) === host_hash("fab_text"))) {
                let scrollview = UI_TREE.Container.children[2].ScrollView;
                let numItems = scrollview.children.length; // Resim dahil olduğu için +1 gibi
                scrollview.children.push({
                    "Label": {
                        "id": "scroll-item-" + numItems,
                        "text": "Kaydırılabilir Öğe " + numItems,
                        "style": { "text_size": 32.0, "text_color": hexToColor("#000000"), "padding": { "top": 10.0, "bottom": 10.0, "left": 10.0, "right": 10.0 }, "height": "Auto" }
                    }
                });
            }
        }
        // Başka bir yere tıklanırsa blur yap
        if (event.type === "ClickOutside") {
            UI_TREE.Container.children[3].TextInput.focused = false;
            host_blur_input();
        }
        host_set_ui(JSON.stringify(UI_TREE));
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

    if (event.type === "Scroll" && event.id === host_hash("scroll-container")) {
        let scrollview = UI_TREE.Container.children[2].ScrollView;
        scrollview.scroll_y += event.dy;
        // Sınırlandırma dinamik olarak hesaplanıyor
        let num_children = scrollview.children.length;
        let num_labels = num_children - 1; // 1 tane resim var
        let content_height = 150.0 + (num_labels * 100.0) + ((num_children - 1) * 10.0);
        let max_scroll = Math.max(0.0, content_height - 300.0);

        if (scrollview.scroll_y < 0) scrollview.scroll_y = 0;
        if (scrollview.scroll_y > max_scroll) scrollview.scroll_y = max_scroll;

        host_set_ui(JSON.stringify(UI_TREE));
    }

    return "[]";
};
