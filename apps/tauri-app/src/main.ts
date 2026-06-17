import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();

if (appWindow.label === "main") {
    import("./overlay/overlay");
} else if (appWindow.label === "settings") {
    import("./settings/settings");
}
