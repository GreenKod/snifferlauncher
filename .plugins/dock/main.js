// Dock Plugin - Main Entry Point
// İki eklenti arası IPC haberleşmesi ve yazı yazma (INPUT) iznini yönetir.

host_log("[Dock Plugin] Initializing dock interface plugin...");

// 1. İzinleri Talep Et (INPUT, IPC, UI)
const granted = requestPermissions([
    "plugin.permission.INPUT",
    "plugin.permission.IPC",
    "plugin.permission.UI"
]);
host_log("[Dock Plugin] Granted permissions: " + JSON.stringify(granted));

// Dock Durumu (State)
const dockState = {
    inputText: "",
    isFocused: false,
    receivedFromDefaultUI: "Hazır (Bağlı)",
    appCount: 0,
};

// DataVault üzerinden anlık uygulama sayısını al
if (typeof Vault !== "undefined" && typeof Vault.queryApps === "function") {
    const initialApps = Vault.queryApps({ limit: 1 });
    if (initialApps.total_count > 0) {
        dockState.appCount = initialApps.total_count;
        dockState.receivedFromDefaultUI = initialApps.total_count + " Uygulama Senkronize";
    }
}

// 2. Eklentiler Arası API Kayıtları (Inter-Plugin APIs)
registerApi("dock.getUI", function(payload) {
    const isLandscape = payload && payload.isLandscape;
    return renderDockUI(isLandscape);
});

registerApi("dock.setText", function(payload) {
    if (payload && typeof payload.text === "string") {
        dockState.inputText = payload.text;
        broadcastEvent("dock.textChanged", { text: dockState.inputText });
        broadcastEvent("dock.stateChanged", {});
        return { success: true, currentText: dockState.inputText };
    }
    return { success: false };
});

registerApi("dock.getText", function() {
    return { text: dockState.inputText };
});

// 3. default_ui Eklentisinden Gelen Broadcast Mesajlarını Dinle
subscribeChannel("default_ui.stats", function(data) {
    if (data && typeof data.appCount === "number") {
        dockState.appCount = data.appCount;
        dockState.receivedFromDefaultUI = data.appCount + " Uygulama Senkronize";
        broadcastEvent("dock.stateChanged", {});
    }
});

subscribeChannel("default_ui.message", function(data) {
    if (data && data.message) {
        dockState.receivedFromDefaultUI = String(data.message);
        broadcastEvent("dock.stateChanged", {});
    }
});

// Başlangıçta hazır olduğunu diğer eklentilere duyur
broadcastEvent("dock.ready", { ready: true });

// 4. UI Çizim Fonksiyonu
function renderDockUI(isLandscape) {
    const textToShow = dockState.inputText || "Uygulama ara veya komut yaz...";
    const isPlaceholder = !dockState.inputText;

    if (isLandscape) {
        return Container("dock_root_container", {
            width: px(vw(16.0)),
            height: pct(100),
            background_color: hex("#111827EE"),
            border_left_width: px(1.0),
            border_left_color: hex("#374151"),
            flex_direction: "Column",
            justify_content: "SpaceBetween",
            align_items: "Center",
            padding: padXY(vmin(1.5), vmin(2.0)),
        }, [
            // Üst Başlık / Durum
            Container("dock_status_box_land", {
                width: pct(100),
                height: "Auto",
                flex_direction: "Column",
                align_items: "Center",
                gap: vh(0.8),
            }, [
                Label("dock_title_land", "⚡ DOCK", {
                    text_color: hex("#38BDF8"),
                    text_size: vw(1.6),
                    width: "Auto",
                }),
                Label("dock_sync_label_land", dockState.receivedFromDefaultUI, {
                    text_color: hex("#9CA3AF"),
                    text_size: vw(1.0),
                    width: "Auto",
                })
            ]),

            // Giriş Alanı Butonu
            Container("dock_input_bar_land", {
                width: pct(100),
                height: px(vh(25.0)),
                background_color: hex("#1F2937"),
                border_radius: vmin(2.0),
                border_width: px(1.0),
                border_color: dockState.isFocused ? hex("#38BDF8") : hex("#4B5563"),
                flex_direction: "Column",
                justify_content: "Center",
                align_items: "Center",
                padding: pad(vmin(1.5)),
            }, [
                Label("dock_input_text_land", textToShow, {
                    text_color: isPlaceholder ? hex("#6B7280") : hex("#F9FAFB"),
                    text_size: vw(1.2),
                    width: "Auto",
                })
            ]),

            // Alt Kısayol Eylem Butonları
            Container("dock_actions_land", {
                width: pct(100),
                height: "Auto",
                flex_direction: "Column",
                align_items: "Center",
                gap: vh(1.0),
            }, [
                Container("dock_btn_clear", {
                    width: pct(90),
                    height: px(vh(6.0)),
                    background_color: hex("#EF4444"),
                    border_radius: vmin(1.5),
                    justify_content: "Center",
                    align_items: "Center",
                }, [
                    Label("dock_clear_lbl", "Temizle", {
                        text_color: hex("#FFFFFF"),
                        text_size: vw(1.1),
                        width: "Auto",
                    })
                ])
            ])
        ]);
    }

    // Dikey (Portrait) Mod
    return Container("dock_root_container", {
        width: pct(100),
        height: px(vh(16.0)),
        background_color: hex("#111827EE"),
        border_top_width: px(1.0),
        border_top_color: hex("#374151"),
        flex_direction: "Column",
        justify_content: "SpaceBetween",
        align_items: "Center",
        padding: padXY(vmin(2.5), vmin(1.5)),
    }, [
        // Üst Bilgi & Senkronizasyon Durumu (İki eklentinin konuştuğunu gösterir)
        Container("dock_header_row", {
            width: pct(100),
            height: "Auto",
            flex_direction: "Row",
            justify_content: "SpaceBetween",
            align_items: "Center",
        }, [
            Label("dock_title_port", "⚡ DOCK IPC", {
                text_color: hex("#38BDF8"),
                text_size: vmin(3.2),
                width: "Auto",
            }),
            Label("dock_sync_label_port", dockState.receivedFromDefaultUI, {
                text_color: hex("#10B981"),
                text_size: vmin(2.8),
                width: "Auto",
            })
        ]),

        // Arama & Yazı Yazma Giriş Çubuğu (Interactive Input / Search Bar)
        Container("dock_input_bar_port", {
            width: pct(100),
            height: px(vh(7.5)),
            background_color: hex("#1F2937"),
            border_radius: vmin(3.0),
            border_width: px(1.5),
            border_color: dockState.isFocused ? hex("#38BDF8") : hex("#4B5563"),
            flex_direction: "Row",
            justify_content: "SpaceBetween",
            align_items: "Center",
            padding: padXY(vmin(3.5), vmin(1.0)),
        }, [
            Container("dock_text_wrapper", {
                width: "Auto",
                height: "Auto",
                flex_direction: "Row",
                align_items: "Center",
                gap: vmin(2.0),
            }, [
                Label("dock_search_icon", "🔍", {
                    text_color: hex("#38BDF8"),
                    text_size: vmin(4.0),
                    width: "Auto",
                }),
                Label("dock_input_text_port", textToShow, {
                    text_color: isPlaceholder ? hex("#6B7280") : hex("#F9FAFB"),
                    text_size: vmin(3.6),
                    width: "Auto",
                })
            ]),

            // Temizle / Klavye Butonu
            Container("dock_btn_clear", {
                width: px(vmin(7.0)),
                height: px(vmin(7.0)),
                background_color: dockState.inputText ? hex("#EF444433") : hex("#374151"),
                border_radius: vmin(3.5),
                justify_content: "Center",
                align_items: "Center",
            }, [
                Label("dock_clear_lbl_icon", dockState.inputText ? "✕" : "⌨", {
                    text_color: dockState.inputText ? hex("#EF4444") : hex("#9CA3AF"),
                    text_size: vmin(3.5),
                    width: "Auto",
                })
            ])
        ]),

        // Alt Bilgi Çizgisi
        Container("dock_bottom_indicator", {
            width: px(vmin(25.0)),
            height: px(3.0),
            background_color: hex("#4B5563"),
            border_radius: px(1.5),
        }, [])
    ]);
}

// 5. Olay Dinleyicisi (Tıklamalar, Klavye Girdileri)
globalThis.onEvent = function(eventJsonString) {
    const e = typeof eventJsonString === "string" ? JSON.parse(eventJsonString) : eventJsonString;

    if (e.type === "Click") {
        const idStr = String(e.id || "");

        // Input barına tıklandığında klavyeyi aç (Focus Input)
        if (idStr.includes("dock_input_bar") || idStr.includes("dock_text_wrapper") || idStr.includes("dock_search_icon")) {
            dockState.isFocused = true;
            if (typeof focusInput === "function") {
                focusInput("dock_search_input");
            }
            broadcastEvent("dock.stateChanged", {});
            host_log("[Dock Plugin] Soft keyboard focus requested");
            return "[]";
        }

        // Temizle butonuna tıklandığında
        if (idStr.includes("dock_btn_clear")) {
            dockState.inputText = "";
            dockState.isFocused = false;
            if (typeof blurInput === "function") {
                blurInput();
            }
            // default_ui eklentisine arama metninin sıfırlandığını bildir!
            broadcastEvent("dock.textChanged", { text: "" });
            broadcastEvent("dock.stateChanged", {});
            host_log("[Dock Plugin] Input cleared and broadcasted to default_ui");
            return "[]";
        }
    }

    // Metin Giriş Olayları (Klavyeden harf yazıldığında)
    if (e.type === "TextInput" && typeof e.text === "string") {
        dockState.inputText += e.text;
        // default_ui eklentisine metin değişikliğini anında bildir!
        broadcastEvent("dock.textChanged", { text: dockState.inputText });
        broadcastEvent("dock.stateChanged", {});
        host_log("[Dock Plugin] TextInput received: '" + e.text + "' => Total: '" + dockState.inputText + "'");
        return "[]";
    }

    // Backspace (Silme tuşu)
    if (e.type === "Backspace") {
        if (dockState.inputText.length > 0) {
            dockState.inputText = dockState.inputText.slice(0, -1);
            broadcastEvent("dock.textChanged", { text: dockState.inputText });
            broadcastEvent("dock.stateChanged", {});
            host_log("[Dock Plugin] Backspace applied => Total: '" + dockState.inputText + "'");
        }
        return "[]";
    }

    if (e.type === "ClickOutside") {
        dockState.isFocused = false;
        if (typeof blurInput === "function") {
            blurInput();
        }
        broadcastEvent("dock.stateChanged", {});
        return "[]";
    }

    return "[]";
};
