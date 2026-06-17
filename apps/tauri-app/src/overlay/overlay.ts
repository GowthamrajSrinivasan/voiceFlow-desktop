import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalPosition } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import "./overlay.css";

enum OverlayState {
    Hidden,
    Listening,
    Processing,
    ShowingTranscript,
    Injecting,
    Error
}

let currentState = OverlayState.Hidden;
let transcriptTextEl: HTMLElement | null;
let transcriptBoxEl: HTMLElement | null;
let stopBtnEl: HTMLElement | null;
let hideTimeout: number | null = null;

// Initialize overlay
const overlayContainer = document.querySelector("#overlay-container") as HTMLElement;
if (overlayContainer) {
    overlayContainer.style.display = "flex";
}

transcriptTextEl = document.querySelector("#transcript-text");
transcriptBoxEl = document.querySelector("#transcript-box");
stopBtnEl = document.querySelector("#stop-btn");

if (stopBtnEl) {
    stopBtnEl.addEventListener("click", () => {
        invoke("stop_listening");
    });
}

positionOverlay();
setupEventListeners();
updateUI(OverlayState.Hidden);

async function positionOverlay() {
    const appWindow = getCurrentWindow() as any;
    const monitor = await appWindow.currentMonitor();
    if (monitor) {
        const screenWidth = monitor.size.width;
        const screenHeight = monitor.size.height;
        const windowSize = await appWindow.outerSize();
        
        const x = (screenWidth - windowSize.width) / 2;
        const y = screenHeight - windowSize.height - 120; // 120px from bottom

        await appWindow.setPosition(new LogicalPosition(x, y));
    }
}

function updateUI(newState: OverlayState, text?: string) {
    currentState = newState;
    
    if (hideTimeout) {
        clearTimeout(hideTimeout);
        hideTimeout = null;
    }

    if (!transcriptTextEl || !transcriptBoxEl) return;
    if (stopBtnEl) stopBtnEl.style.display = "none";

    // Reset classes
    transcriptBoxEl.className = "";
    
    switch (currentState) {
        case OverlayState.Hidden:
            transcriptBoxEl.style.opacity = "0";
            transcriptBoxEl.style.pointerEvents = "none";
            break;
        case OverlayState.Listening:
            transcriptBoxEl.style.opacity = "1";
            transcriptBoxEl.classList.add("listening");
            transcriptTextEl.textContent = text || "Listening...";
            if (stopBtnEl) stopBtnEl.style.display = "inline-block";
            transcriptBoxEl.style.pointerEvents = "auto";
            break;
        case OverlayState.Processing:
            transcriptBoxEl.style.opacity = "1";
            transcriptTextEl.textContent = text || "Processing...";
            break;
        case OverlayState.ShowingTranscript:
            transcriptBoxEl.style.opacity = "1";
            transcriptTextEl.textContent = text || "";
            break;
        case OverlayState.Injecting:
            transcriptBoxEl.style.opacity = "1";
            transcriptTextEl.textContent = text || "Injecting...";
            break;
        case OverlayState.Error:
            transcriptBoxEl.style.opacity = "1";
            transcriptBoxEl.classList.add("error");
            transcriptTextEl.textContent = text || "Error occurred";
            break;
    }
}

function scheduleHide(ms: number) {
    if (hideTimeout) clearTimeout(hideTimeout);
    hideTimeout = window.setTimeout(() => {
        updateUI(OverlayState.Hidden);
    }, ms);
}

async function setupEventListeners() {
    await listen("ListeningStarted", () => {
        updateUI(OverlayState.Listening);
    });

    await listen("ListeningStopped", () => {
        updateUI(OverlayState.Processing);
    });

    await listen<string>("PartialTranscript", (event) => {
        if (currentState === OverlayState.Listening || currentState === OverlayState.Processing) {
            updateUI(OverlayState.ShowingTranscript, event.payload);
            currentState = OverlayState.Listening; // maintain listening glow for partials
            if (transcriptBoxEl) transcriptBoxEl.classList.add("listening");
        }
    });

    await listen<string>("FinalTranscript", (event) => {
        updateUI(OverlayState.ShowingTranscript, event.payload);
        scheduleHide(2000);
    });

    await listen("InjectionStarted", () => {
        updateUI(OverlayState.Injecting);
    });

    await listen("InjectionCompleted", () => {
        updateUI(OverlayState.Hidden); // Instantly hide on success
    });

    await listen<string>("ErrorOccurred", (event) => {
        updateUI(OverlayState.Error, event.payload);
        scheduleHide(3000);
    });
}
