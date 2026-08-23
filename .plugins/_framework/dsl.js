// =============================================================================
// Functional UI builders — the "JSX" of SnifferUI
// =============================================================================

function Container(id, style, children) {
    return { Container: { id: id, style: style || {}, children: children || [] } };
}

function Label(id, text, style) {
    return { Label: { id: id, text: String(text), style: style || {} } };
}

function TextInput(id, value, opts) {
    opts = opts || {};
    return {
        TextInput: {
            id: id,
            value: String(value),
            placeholder: String(opts.placeholder || ""),
            focused: !!opts.focused,
            style: opts.style || {},
        },
    };
}

function Image(id, src, style) {
    return { Image: { id: id, src: String(src), style: style || {} } };
}

function ScrollView(id, opts, children) {
    opts = opts || {};
    return {
        ScrollView: {
            id: id,
            scroll_x: opts.scroll_x !== undefined ? opts.scroll_x : 0.0,
            scroll_y: opts.scroll_y !== undefined ? opts.scroll_y : 0.0,
            scroll_sensitivity: opts.scroll_sensitivity !== undefined ? opts.scroll_sensitivity : 1.0,
            dynamic_sensitivity: opts.dynamic_sensitivity !== undefined ? opts.dynamic_sensitivity : true,
            momentum_scrolling: opts.momentum_scrolling !== undefined ? opts.momentum_scrolling : true,
            capture_drag: opts.capture_drag !== undefined ? opts.capture_drag : true,
            style: opts.style || {},
            children: children || [],
            snap_x: opts.snap_x !== undefined ? opts.snap_x : null,
            snap_y: opts.snap_y !== undefined ? opts.snap_y : null,
            rubber_band: opts.rubber_band !== undefined ? opts.rubber_band : null,
            page_count: opts.page_count !== undefined ? opts.page_count : null,
            on_snap: opts.on_snap !== undefined ? opts.on_snap : null,
        },
    };
}

function Checkbox(id, checked, style) {
    return { Checkbox: { id: id, checked: !!checked, style: style || {} } };
}

function Slider(id, value, min, max, style) {
    return { Slider: { id: id, value: value, min: min, max: max, style: style || {} } };
}

function ProgressBar(id, value, max, style) {
    return { ProgressBar: { id: id, value: value, max: max, style: style || {} } };
}

function SharedView(id, targetPluginId, slotName, style, children) {
    return {
        SharedView: {
            id: id,
            target_plugin: targetPluginId ? String(targetPluginId) : null,
            slot_name: slotName ? String(slotName) : null,
            style: style || {},
            children: children || []
        }
    };
}

function CardWidget(opts) {
    opts = opts || {};
    const id = opts.id || "card-widget";
    const title = opts.title || "";
    const subtitle = opts.subtitle || "";
    const content = opts.content || [];
    const customStyle = opts.style || {};

    const children = [];
    if (title) {
        children.push(Label(id + "-title", title, {
            text_color: Theme.colors.accent,
            text_size: vmin(4.0),
            width: "Auto"
        }));
    }
    if (subtitle) {
        children.push(Label(id + "-sub", subtitle, {
            text_color: Theme.colors.textSub,
            text_size: vmin(3.0),
            width: "Auto"
        }));
    }
    if (Array.isArray(content)) {
        for (let i = 0; i < content.length; i++) {
            children.push(content[i]);
        }
    } else if (content) {
        children.push(content);
    }

    const mergedStyle = Object.assign({
        background_color: Theme.colors.bgCard,
        border_radius: vmin(4.0),
        border_width: vmin(0.2),
        border_color: Theme.colors.border,
        padding: pad(vmin(3.5)),
        flex_direction: "Column",
        gap: vmin(2.0),
        overflow_hidden: true
    }, customStyle);

    return Container(id, mergedStyle, children);
}
