// =============================================================================
// Sniffer Launcher - DevKit Performance & Telemetry HUD Plugin
// =============================================================================

if (typeof host_log === "function") {
    host_log("[DevKit HUD] Initializing DevKit HUD plugin...");
}

// 1. Request Permissions
if (typeof requestPermissions === "function") {
    requestPermissions([
        "plugin.permission.IPC",
        "plugin.permission.UI"
    ]);
}

/**
 * Generates the DevKit HUD UI Card by querying telemetry from pkg_perfmon.
 */
let isDevKitExpanded = false;

function getDevKitHUDUI() {
    let telemetry = {
        fps: 60.0,
        frame_time_ms: 16.6,
        cpu_ms: 0.0,
        gpu_ms: 0.0,
        swap_ms: 16.0,
        cpu_usage: 0.0,
        mem_mb: 0.0,
        touch: {
            active_pointers: 0,
            gesture: "IDLE",
            target: "None",
            touch_x: 0.0,
            touch_y: 0.0
        }
    };

    if (typeof host_pkg_query === "function") {
        try {
            const raw = host_pkg_query("com.sniffer.perf", "getDevKitTelemetry", "{}");
            if (raw) {
                const parsed = JSON.parse(raw);
                if (parsed && parsed.ok) {
                    telemetry = parsed;
                }
            }
        } catch (e) {
            // Fallback
        }
    }

    const fps = telemetry.fps || 0.0;
    const avgMs = telemetry.frame_time_ms || 0.0;
    const cpuMs = telemetry.cpu_ms || 0.0;
    const gpuMs = telemetry.gpu_ms || 0.0;
    const swapMs = telemetry.swap_ms || 0.0;
    const entities = telemetry.entities || { rendered: 0, total: 0 };
    const renderedEntities = entities.rendered !== undefined ? entities.rendered : 0;
    const totalEntities = entities.total !== undefined ? entities.total : 0;
    const touch = telemetry.touch || {};
    const pointers = touch.active_pointers || 0;
    const gesture = touch.gesture || "IDLE";
    const target = touch.target || "None";
    const tx = touch.touch_x || 0.0;
    const ty = touch.touch_y || 0.0;

    const fpsColor = fps >= 55.0 ? "#4ADE80" : (fps >= 30.0 ? "#FBBF24" : "#F87171");

    if (!isDevKitExpanded) {
        return Container("devkit_hud_card", {
            position: "Absolute",
            top: px(vh(0.5)),
            left: px(16.0),
            height: px(28.0),
            background_color: hex("#CC0F172A"),
            border_radius: 14.0,
            border_color: hex("#38BDF8"),
            border_width: 1.0,
            padding: padXY(4.0, 10.0),
            flex_direction: "Row",
            align_items: "Center",
            justify_content: "Center",
            gap: 6.0,
        }, [
            Label("devkit_fps", "⚡ " + fps.toFixed(1) + " FPS", {
                text_color: hex(fpsColor),
                text_size: 11.5,
                width: "Auto"
            }),
            Label("devkit_expand_icon", "▾", {
                text_color: hex("#94A3B8"),
                text_size: 11.0,
                width: "Auto"
            })
        ]);
    }

    return Container("devkit_hud_card", {
        position: "Absolute",
        top: px(vh(0.5)),
        left: px(16.0),
        width: px(300.0),
        background_color: hex("#EE0F172A"),
        border_radius: 14.0,
        border_color: hex("#38BDF8"),
        border_width: 2.0,
        padding: padXY(14.0, 16.0),
        flex_direction: "Column",
        gap: 6.0,
    }, [
        Container("devkit_hud_header", {
            width: pct(100),
            flex_direction: "Row",
            justify_content: "SpaceBetween",
            align_items: "Center",
        }, [
            Label("devkit_title", "⚡ DEVKIT MONITOR", {
                text_color: hex("#38BDF8"),
                text_size: 14.0,
                width: "Auto"
            }),
            Label("devkit_close_btn", "✕", {
                text_color: hex("#94A3B8"),
                text_size: 13.0,
                width: "Auto"
            })
        ]),
        Label("devkit_fps", "FPS: " + fps.toFixed(1) + " (" + avgMs.toFixed(1) + "ms)", {
            text_color: hex(fpsColor),
            text_size: 15.0,
            width: "Auto"
        }),
        Label("devkit_entities", "📦 Entities: " + renderedEntities + " / " + totalEntities + " rendered", {
            text_color: hex("#38BDF8"),
            text_size: 12.5,
            width: "Auto"
        }),
        Label("devkit_cpu_gpu", "CPU: " + cpuMs.toFixed(1) + "ms | GPU: " + gpuMs.toFixed(1) + "ms", {
            text_color: hex("#E2E8F0"),
            text_size: 12.5,
            width: "Auto"
        }),
        Label("devkit_vsync", "VSync: " + swapMs.toFixed(1) + "ms", {
            text_color: hex("#94A3B8"),
            text_size: 12.5,
            width: "Auto"
        }),
        Label("devkit_pointers", "👉 Pointers: " + pointers + " | " + gesture, {
            text_color: hex("#FACC15"),
            text_size: 12.5,
            width: "Auto"
        }),
        Label("devkit_target", "Target: " + target, {
            text_color: hex("#E2E8F0"),
            text_size: 12.5,
            width: "Auto"
        }),
        Label("devkit_pos", "Pos: (" + tx.toFixed(1) + ", " + ty.toFixed(1) + ")", {
            text_color: hex("#94A3B8"),
            text_size: 12.5,
            width: "Auto"
        })
    ]);
}

// 2. Register API
if (typeof registerApi === "function") {
    registerApi("devkit_hud.getUI", getDevKitHUDUI);
    registerApi("devkit_hud.toggle", function() {
        isDevKitExpanded = !isDevKitExpanded;
        return { expanded: isDevKitExpanded };
    });
}

// 3. Broadcast ready event
if (typeof broadcastEvent === "function") {
    broadcastEvent("devkit_hud.ready", {});
}

// 4. Handle direct click events
globalThis.onEvent = function(eventJsonString) {
    const e = typeof eventJsonString === "string" ? JSON.parse(eventJsonString) : eventJsonString;
    if (e && e.type === "Click" && e.id && String(e.id).startsWith("devkit_")) {
        isDevKitExpanded = !isDevKitExpanded;
        if (typeof SnifferUI !== "undefined" && typeof SnifferUI.forceUpdate === "function") {
            SnifferUI.forceUpdate();
        }
        return "[]";
    }
    return "[]";
};
