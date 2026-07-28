/**
 * SnifferLauncher Plugin Framework — TypeScript & IntelliSense Type Definitions
 * 
 * Ambient script declaration file. No top-level export/import so all declarations
 * are placed into the global namespace for vanilla JavaScript plugins.
 */

interface Style {
    display?: "flex" | "none";
    position?: "relative" | "absolute";
    left?: number | string;
    right?: number | string;
    top?: number | string;
    bottom?: number | string;

    flexDirection?: "row" | "column" | "row-reverse" | "column-reverse";
    flexWrap?: "no-wrap" | "wrap" | "wrap-reverse";
    justifyContent?: "flex-start" | "flex-end" | "center" | "space-between" | "space-around" | "space-evenly";
    alignItems?: "flex-start" | "flex-end" | "center" | "stretch" | "baseline";
    gap?: number;

    flexGrow?: number;
    flexShrink?: number;
    flexBasis?: number | string;

    width?: number | string;
    height?: number | string;
    minWidth?: number | string;
    minHeight?: number | string;
    maxWidth?: number | string;
    maxHeight?: number | string;

    padding?: number;
    paddingLeft?: number;
    paddingRight?: number;
    paddingTop?: number;
    paddingBottom?: number;

    margin?: number;
    marginLeft?: number;
    marginRight?: number;
    marginTop?: number;
    marginBottom?: number;

    backgroundColor?: number | string;
    color?: number | string;
    fontSize?: number;
    borderRadius?: number;
    borderWidth?: number;
    borderColor?: number | string;
    overflowHidden?: boolean;
}

type UIElement =
    | { Container: { id?: string | null; style?: Style; children?: UIElement[] } }
    | { Label: { id?: string | null; text: string; style?: Style } }
    | { TextInput: { id?: string | null; value: string; placeholder?: string; style?: Style; focused?: boolean } }
    | { Image: { id?: string | null; src: string; style?: Style } }
    | { ScrollView: { id?: string | null; style?: Style; children?: UIElement[]; scroll_x?: number; scroll_y?: number } }
    | { Checkbox: { id?: string | null; checked: boolean; label?: string; style?: Style } }
    | { Slider: { id?: string | null; value: number; min?: number; max?: number; style?: Style } }
    | { ProgressBar: { id?: string | null; progress: number; style?: Style } };

interface AppInfo {
    name: string;
    package_name: string;
    icon?: string;
}

interface SnifferUIFramework {
    start(renderFn: () => UIElement, initialState?: any): void;
    setState(patch: any): void;
    forceUpdate(): void;
}

interface UIEventData {
    type: "Click" | "Hover" | "HoverEnd" | "PointerDown" | "PointerUp" | "TextInput" | "Backspace" | "Scroll" | "ClickOutside";
    id?: string;
    width?: number;
    height?: number;
    x?: number;
    y?: number;
    text?: string;
    dx?: number;
    dy?: number;
    max_y?: number;
}

// -----------------------------------------------------------------------------
// Global Inter-Plugin API & Communication
// -----------------------------------------------------------------------------

declare function registerApi<TPayload = any, TResponse = any>(
    name: string,
    handler: (payload: TPayload) => TResponse
): void;

declare function callApi<TResponse = any, TPayload = any>(
    name: string,
    payload?: TPayload
): TResponse | null;

declare function broadcastEvent<TData = any>(
    channel: string,
    data: TData
): void;

declare function onBroadcast<TData = any>(
    channel: string,
    callback: (data: TData) => void
): void;

declare function requestPermissions(permissions: string[]): string[];
declare function hasPermission(permission: string): boolean;
declare function getApplicationList(): AppInfo[];
declare function getLocalTime(): { time: string; date: string; hours: number; minutes: number; seconds: number; timestamp: number };

/** Default settings loaded from manifest.json ("defaultSettings" key) */
declare var defaultSettings: Record<string, any>;

// -----------------------------------------------------------------------------
// Low-level Host Functions
// -----------------------------------------------------------------------------

declare function host_set_ui(jsonStr: string): void;
declare function host_update_style(id: string, property: string, value: string): void;
declare function host_set_text(id: string, text: string): void;
declare function host_insert_child(parentId: string, childJson: string): void;
declare function host_remove_node(id: string): void;
declare function host_log(msg: string): void;
declare function host_hash(str: string): string;
declare function host_screen_width(): number;
declare function host_screen_height(): number;
declare function host_get_local_time(): string;
declare function host_get_default_settings(): string;
declare function host_create_image(id: string, src: string): void;
declare function host_focus_input(id: string): void;
declare function host_blur_input(): void;
declare function host_has_permission(permission: string): boolean;
declare function host_request_permissions(permissions: string[]): string;
declare function host_register_api(name: string): void;
declare function host_call_api(name: string, payloadJson: string): string | null;
declare function host_broadcast(channel: string, payloadJson: string): void;

// -----------------------------------------------------------------------------
// SnifferUI Framework & Component Builders
// -----------------------------------------------------------------------------

declare const SnifferUI: SnifferUIFramework;
declare function hex(hexStr: string): number;
declare function hexToColor(hexStr: string): number;

declare function Container(id?: string | null, style?: Style, children?: UIElement[]): UIElement;
declare function Label(id?: string | null, text?: string, style?: Style): UIElement;
declare function TextInput(id?: string | null, value?: string, placeholder?: string, style?: Style): UIElement;
declare function Image(id?: string | null, src?: string, style?: Style): UIElement;
declare function ScrollView(id?: string | null, style?: Style, children?: UIElement[]): UIElement;
declare function Checkbox(id?: string | null, checked?: boolean, label?: string, style?: Style): UIElement;
declare function Slider(id?: string | null, value?: number, min?: number, max?: number, style?: Style): UIElement;
declare function ProgressBar(id?: string | null, progress?: number, style?: Style): UIElement;
declare function SharedView(id?: string | null, targetPluginId?: string | null, slotName?: string | null, style?: Style, children?: UIElement[]): UIElement;

declare function requestSharedView(targetPluginId: string, slotName: string, payload?: any, timeoutMs?: number): Promise<{ accepted: boolean; status: string; reason?: string; uiTree?: UIElement }>;
declare function acceptSharedView(invitationId: string, uiTree: UIElement): void;
declare function rejectSharedView(invitationId: string, reason?: string): void;
declare var onRequestSharedView: (invitation: { invitationId: string; slotName: string; payload: any }) => void;

declare function subscribeChannel(channelName: string, callback: (data: any) => void): void;
declare function unsubscribeChannel(channelName: string): void;

declare const Theme: {
    colors: {
        bgCard: number;
        bgCardAlt: number;
        textMain: number;
        textSub: number;
        accent: number;
        success: number;
        border: number;
    };
    spacing: { xs: number; sm: number; md: number; lg: number };
    radius: { sm: number; md: number; lg: number };
};

declare function CardWidget(opts?: {
    id?: string;
    title?: string;
    subtitle?: string;
    content?: UIElement | UIElement[];
    style?: Style;
}): UIElement;

declare var onEvent: (event: UIEventData) => string | void;

// Standard timers
declare function setTimeout(handler: Function | string, timeout?: number, ...args: any[]): number;
declare function clearTimeout(id?: number): void;
declare function setInterval(handler: Function | string, timeout?: number, ...args: any[]): number;
declare function clearInterval(id?: number): void;
