import { invoke } from "@tauri-apps/api/core";
import "./settings.css";

interface AppSettings {
    version: number;
    launch_at_login: boolean;
    overlay_enabled: boolean;
    theme: string;
    hotkey: string;
    language: string;
    vocabulary_count: number;
    auto_paste: boolean;
    clipboard_fallback: boolean;
    show_notifications: boolean;
}

let currentSettings: AppSettings | null = null;

async function init() {
    // Show settings container
    const settingsContainer = document.querySelector("#settings-container") as HTMLElement;
    if (settingsContainer) {
        settingsContainer.style.display = "flex";
    }

    setupTabs();
    await loadSettings();

    const saveBtn = document.querySelector("#save-btn");
    if (saveBtn) {
        saveBtn.addEventListener("click", saveSettings);
    }
}

if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
} else {
    init();
}

function setupTabs() {
    const tabBtns = document.querySelectorAll(".tab-btn");
    const tabContents = document.querySelectorAll(".tab-content");

    tabBtns.forEach(btn => {
        btn.addEventListener("click", () => {
            const tabId = btn.getAttribute("data-tab");
            if (!tabId) return;

            // Remove active class from buttons
            tabBtns.forEach(b => b.classList.remove("active"));
            // Add active class to clicked button
            btn.classList.add("active");

            // Hide all contents and show selected
            tabContents.forEach(content => {
                content.classList.remove("active");
                if (content.getAttribute("id") === `${tabId}-tab`) {
                    content.classList.add("active");
                }
            });
        });
    });
}

async function loadSettings() {
    try {
        currentSettings = await invoke<AppSettings>("get_settings");
        
        // Populate inputs
        setCheckbox("#launch-at-login", currentSettings.launch_at_login);
        setCheckbox("#overlay-enabled", currentSettings.overlay_enabled);
        setSelect("#theme", currentSettings.theme);
        setInputValue("#hotkey", currentSettings.hotkey);
        setSelect("#language", currentSettings.language);
        setCheckbox("#auto-paste", currentSettings.auto_paste);
        setCheckbox("#clipboard-fallback", currentSettings.clipboard_fallback);
        setCheckbox("#show-notifications", currentSettings.show_notifications);
        
        const vocabCountEl = document.querySelector("#vocab-count-val");
        if (vocabCountEl) {
            vocabCountEl.textContent = currentSettings.vocabulary_count.toString();
        }
    } catch (err) {
        console.error("Failed to load settings from Rust:", err);
    }
}

async function saveSettings() {
    if (!currentSettings) return;

    // Read values
    currentSettings.launch_at_login = getCheckbox("#launch-at-login");
    currentSettings.overlay_enabled = getCheckbox("#overlay-enabled");
    currentSettings.theme = getSelect("#theme");
    // hotkey is read-only
    currentSettings.language = getSelect("#language");
    currentSettings.auto_paste = getCheckbox("#auto-paste");
    currentSettings.clipboard_fallback = getCheckbox("#clipboard-fallback");
    currentSettings.show_notifications = getCheckbox("#show-notifications");

    try {
        await invoke("save_settings", { settings: currentSettings });
        showSaveStatus("✓ Settings saved successfully");
    } catch (err) {
        console.error("Failed to save settings:", err);
        showSaveStatus(`Error saving: ${err}`);
    }
}

function setCheckbox(selector: string, val: boolean) {
    const el = document.querySelector(selector) as HTMLInputElement | null;
    if (el) el.checked = val;
}

function getCheckbox(selector: string): boolean {
    const el = document.querySelector(selector) as HTMLInputElement | null;
    return el ? el.checked : false;
}

function setSelect(selector: string, val: string) {
    const el = document.querySelector(selector) as HTMLSelectElement | null;
    if (el) el.value = val;
}

// Fixed parameter types: cast select element value
function getSelect(selector: string): string {
    const el = document.querySelector(selector) as HTMLSelectElement | null;
    return el ? el.value : "";
}

function setInputValue(selector: string, val: string) {
    const el = document.querySelector(selector) as HTMLInputElement | null;
    if (el) el.value = val;
}

function showSaveStatus(msg: string) {
    const statusEl = document.querySelector("#save-status") as HTMLElement | null;
    if (statusEl) {
        statusEl.textContent = msg;
        statusEl.classList.add("show");
        setTimeout(() => {
            statusEl.classList.remove("show");
        }, 2500);
    }
}
