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
    await loadVocabulary();

    const saveBtn = document.querySelector("#save-btn");
    if (saveBtn) {
        saveBtn.addEventListener("click", async () => {
            await saveSettings();
            await saveVocabulary();
            showSaveStatus("✓ Settings & Vocabulary saved successfully");
        });
    }

    const addVocabBtn = document.querySelector("#add-vocab-btn");
    if (addVocabBtn) addVocabBtn.addEventListener("click", handleAddVocab);

    const exportVocabBtn = document.querySelector("#export-vocab-btn");
    if (exportVocabBtn) exportVocabBtn.addEventListener("click", handleExportVocab);

    const importVocabBtn = document.querySelector("#import-vocab-btn");
    const fileInput = document.querySelector("#vocab-file-input") as HTMLInputElement;
    if (importVocabBtn && fileInput) {
        importVocabBtn.addEventListener("click", () => fileInput.click());
        fileInput.addEventListener("change", handleImportVocab);
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

// --- Vocabulary Logic ---

interface VocabularyEntry {
    spoken: string;
    output: string;
    language: string;
    enabled: boolean;
    case_sensitive: boolean;
}

interface UserVocabulary {
    entries: VocabularyEntry[];
}

let currentVocab: UserVocabulary = { entries: [] };

async function loadVocabulary() {
    try {
        currentVocab = await invoke<UserVocabulary>("get_vocabulary");
        renderVocabTable();
    } catch (err) {
        console.error("Failed to load vocabulary:", err);
    }
}

function renderVocabTable() {
    const tbody = document.querySelector("#vocab-tbody");
    if (!tbody) return;
    tbody.innerHTML = "";

    currentVocab.entries.forEach((entry, index) => {
        const tr = document.createElement("tr");
        
        const tdSpoken = document.createElement("td");
        tdSpoken.textContent = entry.spoken;
        
        const tdOutput = document.createElement("td");
        tdOutput.textContent = entry.output;
        
        const tdCase = document.createElement("td");
        tdCase.textContent = entry.case_sensitive ? "Yes" : "No";

        const tdActions = document.createElement("td");
        const delBtn = document.createElement("button");
        delBtn.className = "btn-secondary btn-sm";
        delBtn.textContent = "Remove";
        delBtn.onclick = () => {
            currentVocab.entries.splice(index, 1);
            renderVocabTable();
        };
        tdActions.appendChild(delBtn);

        tr.appendChild(tdSpoken);
        tr.appendChild(tdOutput);
        tr.appendChild(tdCase);
        tr.appendChild(tdActions);
        tbody.appendChild(tr);
    });
}

function handleAddVocab() {
    const spokenInput = document.querySelector("#new-vocab-spoken") as HTMLInputElement;
    const outputInput = document.querySelector("#new-vocab-output") as HTMLInputElement;
    const caseInput = document.querySelector("#new-vocab-case") as HTMLInputElement;
    const errorMsg = document.querySelector("#vocab-error-msg") as HTMLElement;

    errorMsg.textContent = "";

    let spoken = spokenInput.value.trim().toLowerCase();
    spoken = spoken.replace(/\s+/g, " "); // normalize whitespace

    const output = outputInput.value.trim();
    const caseSensitive = caseInput.checked;

    if (!spoken || !output) {
        errorMsg.textContent = "Spoken phrase and output cannot be empty.";
        return;
    }
    
    if (spoken === output.toLowerCase()) {
        errorMsg.textContent = "Spoken phrase and output are the same.";
        return;
    }

    if (currentVocab.entries.some(e => e.spoken === spoken && e.case_sensitive === caseSensitive)) {
        errorMsg.textContent = "This exact mapping already exists.";
        return;
    }

    currentVocab.entries.push({
        spoken,
        output,
        language: "en",
        enabled: true,
        case_sensitive: caseSensitive
    });

    spokenInput.value = "";
    outputInput.value = "";
    caseInput.checked = false;

    renderVocabTable();
}

async function saveVocabulary() {
    try {
        await invoke("save_vocabulary", { vocab: currentVocab });
    } catch (err) {
        console.error("Failed to save vocabulary:", err);
        throw err;
    }
}

function handleExportVocab() {
    const json = JSON.stringify(currentVocab, null, 2);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "voiceflow_vocabulary.json";
    a.click();
    URL.revokeObjectURL(url);
}

function handleImportVocab(e: Event) {
    const target = e.target as HTMLInputElement;
    if (!target.files || target.files.length === 0) return;

    const file = target.files[0];
    const reader = new FileReader();
    reader.onload = (event) => {
        try {
            const result = event.target?.result as string;
            const imported = JSON.parse(result) as UserVocabulary;
            if (imported && Array.isArray(imported.entries)) {
                let added = 0;
                for (const entry of imported.entries) {
                    if (!currentVocab.entries.some(e => e.spoken === entry.spoken && e.case_sensitive === entry.case_sensitive)) {
                        currentVocab.entries.push(entry);
                        added++;
                    }
                }
                renderVocabTable();
                alert(`Successfully imported ${added} new mappings.`);
            } else {
                alert("Invalid vocabulary file format.");
            }
        } catch (err) {
            alert("Failed to parse JSON file.");
            console.error(err);
        }
        target.value = "";
    };
    reader.readAsText(file);
}
