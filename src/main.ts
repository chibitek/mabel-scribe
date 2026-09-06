import { invoke } from "@tauri-apps/api/core";
import { listen, emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, Update } from "@tauri-apps/plugin-updater";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

interface Settings {
  microphone: string;
  engine: string;
  localEngine: string;
  whisperModel: string;
  groqApiKey: string;
  recordingMode: string;
  hotkey: string;
  streaming: boolean;
  groqKeyConfigured: boolean;
  launchAtLogin: boolean;
  showInDock: boolean;
  dictationSounds: boolean;
  pressEnterCommand: boolean;
  cleanupMode: string;
  llmModel: string;
  polishMode: string;
  companionEnabled: boolean;
  companionSize: string;
  companionFrequency: string;
  companionVisit: string;
  lastSeenVersion: string;
  whisperLanguage: string;
  dictionary: string[];
  clipboardHistoryEnabled: boolean;
}

interface VersionInfo {
  version: string;
  gitHash: string;
  dirty: boolean;
}

interface StatsSummary {
  today: number;
  total: number;
  streak: number;
  total_words: number;
  wpm: number;
  last30: number[];
  time_saved_minutes: number;
}

interface MicDevice {
  name: string;
  is_default: boolean;
}

interface DownloadProgress {
  downloaded: number;
  total: number;
  percent: number;
}

interface TranscriptSegment {
  startMs: number;
  endMs: number;
  text: string;
  confidence: number | null;
}

interface FileTranscription {
  transcript: {
    text: string;
    language: string;
    confidence: number | null;
    durationMs: number;
    segments: TranscriptSegment[];
  };
  txt: string;
  json: string;
  srt: string;
  vtt: string;
}

interface TranscriptionQuality {
  confidence: number;
  lowConfidence: boolean;
}

const $ = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;

const statusDot = $("status-dot");
const statusText = $("status-text");
const homeHotkey = $("home-hotkey");
const micSelect = $<HTMLSelectElement>("mic-select");
const engineLocal = $("engine-local");
const engineCloud = $("engine-cloud");
const localSettings = $("local-settings");
const cloudSettings = $("cloud-settings");
const localEngineSelect = $<HTMLSelectElement>("local-engine-select");
const whisperCppSettings = $("whisper-cpp-settings");
const modelSelect = $<HTMLSelectElement>("model-select");
const languageSelect = $<HTMLSelectElement>("language-select");
const downloadBtn = $<HTMLButtonElement>("download-btn");
const downloadProgress = $("download-progress");
const progressFill = $("progress-fill");
const cleanupModeSelect = $<HTMLSelectElement>("cleanup-mode-select");
const polishToggle = $<HTMLButtonElement>("polish-toggle");
const polishModeRow = $("polish-mode-row");
const polishModeSelect = $<HTMLSelectElement>("polish-mode-select");
const llmSettings = $("llm-settings");
const llmModelSelect = $<HTMLSelectElement>("llm-model-select");
const llmDownloadBtn = $<HTMLButtonElement>("llm-download-btn");
const llmDownloadProgress = $("llm-download-progress");
const llmProgressFill = $("llm-progress-fill");
const groqKey = $<HTMLInputElement>("groq-key");
const keySave = $<HTMLButtonElement>("key-save");
const keyStatus = $("key-status");
const modeToggle = $("mode-toggle");
const modePtt = $("mode-ptt");
const hotkeyText = $("hotkey-text");
const streamingToggle = $<HTMLButtonElement>("streaming-toggle");
const checkUpdatesBtn = $<HTMLButtonElement>("check-updates-btn");
const showWhatsNewBtn = $<HTMLButtonElement>("show-whatsnew-btn");
const updateStatus = $("update-status");
const updateModal = $("update-modal");
const updateVersion = $("update-version");
const updateBody = $("update-body");
const updateInstallBtn = $<HTMLButtonElement>("update-install");
const updateLaterBtn = $<HTMLButtonElement>("update-later");
let pendingUpdate: Update | null = null;
const fileTranscribeChoose = $<HTMLButtonElement>("file-transcribe-choose");
const fileTranscribeName = $("file-transcribe-name");
const fileTranscribeStatus = $("file-transcribe-status");
const fileTranscribeResult = $("file-transcribe-result");
const fileTranscribeMeta = $("file-transcribe-meta");
const fileTranscribeText = $<HTMLTextAreaElement>("file-transcribe-text");
let selectedAudioPath = "";
let fileTranscription: FileTranscription | null = null;

const appWindow = getCurrentWindow();
$("titlebar").addEventListener("mousedown", (e) => {
  if ((e.target as HTMLElement).closest("button, select, input, a, kbd")) return;
  appWindow.startDragging();
});

// Sidebar nav (Home + locked Pro views)
document.querySelectorAll<HTMLElement>(".nav-item").forEach((item) => {
  item.addEventListener("click", () => {
    if (item.classList.contains("locked")) {
      askProUnlock();
      return;
    }
    const view = item.dataset.view!;
    document.querySelectorAll(".nav-item").forEach((n) => n.classList.remove("active"));
    document.querySelectorAll<HTMLElement>(".view").forEach((s) => s.classList.remove("active"));
    item.classList.add("active");
    document.querySelector(`.view[data-view="${view}"]`)?.classList.add("active");
  });
});

function displayFilename(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

function formatDuration(milliseconds: number): string {
  const totalSeconds = Math.round(milliseconds / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return minutes > 0 ? `${minutes}m ${seconds}s` : `${seconds}s`;
}

fileTranscribeChoose.addEventListener("click", async () => {
  const selected = await openDialog({
    multiple: false,
    directory: false,
    filters: [{ name: "Audio", extensions: ["wav", "mp3", "ogg", "flac"] }],
  });
  if (!selected || Array.isArray(selected)) return;

  selectedAudioPath = selected;
  fileTranscribeName.textContent = displayFilename(selected);
  fileTranscribeChoose.disabled = true;
  fileTranscribeChoose.textContent = "Transcribing...";
  fileTranscribeStatus.textContent = "Removing silence and transcribing locally. Longer recordings may take a few minutes.";
  fileTranscribeStatus.className = "file-transcribe-status busy";
  fileTranscribeResult.classList.add("hidden");

  try {
    fileTranscription = await invoke<FileTranscription>("transcribe_audio_file", { path: selected });
    fileTranscribeText.value = fileTranscription.transcript.text;
    const confidence = fileTranscription.transcript.confidence;
    const confidencePercent = confidence == null ? null : Math.round(confidence * 100);
    const confidenceClass = confidencePercent != null && confidencePercent < 55 ? "low-confidence" : "";
    const confidenceLabel = confidencePercent == null
      ? "Confidence unavailable"
      : `${confidencePercent}% confidence`;
    const metadata = [
      fileTranscription.transcript.language.toUpperCase(),
      formatDuration(fileTranscription.transcript.durationMs),
      `${fileTranscription.transcript.segments.length} segment${fileTranscription.transcript.segments.length === 1 ? "" : "s"}`,
      confidenceLabel,
    ];
    fileTranscribeMeta.replaceChildren(...metadata.map((label, index) => {
      const item = document.createElement("span");
      item.textContent = label;
      if (index === metadata.length - 1 && confidenceClass) item.classList.add(confidenceClass);
      return item;
    }));
    fileTranscribeStatus.textContent = "Transcription complete. Nothing has been saved yet.";
    fileTranscribeStatus.className = "file-transcribe-status";
    fileTranscribeResult.classList.remove("hidden");
  } catch (error) {
    fileTranscription = null;
    fileTranscribeStatus.textContent = `Transcription failed: ${String(error)}`;
    fileTranscribeStatus.className = "file-transcribe-status error";
  } finally {
    fileTranscribeChoose.disabled = false;
    fileTranscribeChoose.textContent = "Choose another";
  }
});

document.querySelectorAll<HTMLButtonElement>(".transcript-export").forEach((button) => {
  button.addEventListener("click", async () => {
    if (!fileTranscription) return;
    const format = button.dataset.format as "txt" | "json" | "srt" | "vtt";
    const sourceName = displayFilename(selectedAudioPath).replace(/\.[^.]+$/, "");
    try {
      const savedName = await invoke<string | null>("save_transcript_export", {
        defaultName: `${sourceName}.${format}`,
        format,
        contents: fileTranscription[format],
      });
      if (!savedName) return;
      fileTranscribeStatus.textContent = `${format.toUpperCase()} saved to ${savedName}.`;
      fileTranscribeStatus.className = "file-transcribe-status";
    } catch (error) {
      fileTranscribeStatus.textContent = `Export failed: ${String(error)}`;
      fileTranscribeStatus.className = "file-transcribe-status error";
    }
  });
});

// Settings modal open/close
const modal = $("settings-modal");
$("open-settings").addEventListener("click", () => {
  modal.classList.remove("hidden");
  // Reconcile the keychain status only when Settings opens, not on every
  // recording-state poll. On dev builds, has_groq_key() prompts for keychain
  // access (signature changes per rebuild), so we want this fired only at a
  // moment the user expects keychain interaction.
  invoke<boolean>("reconcile_groq_keychain")
    .then((found) => {
      if (found && !currentSettings.groqKeyConfigured) {
        currentSettings.groqKeyConfigured = true;
        groqKey.placeholder = "•••••••••••••••• (stored)";
        keyStatus.textContent = "Saved";
        keyStatus.classList.remove("hidden");
      }
    })
    .catch((e) => console.error("reconcile_groq_keychain:", e));
});
$("close-settings").addEventListener("click", () => modal.classList.add("hidden"));
modal.querySelector(".modal-backdrop")?.addEventListener("click", () => modal.classList.add("hidden"));
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !modal.classList.contains("hidden") && !capturingHotkey) {
    modal.classList.add("hidden");
  }
});

// Modal pane nav
document.querySelectorAll<HTMLElement>(".modal-nav-item").forEach((item) => {
  item.addEventListener("click", () => {
    if (item.classList.contains("locked")) return;
    const pane = item.dataset.pane!;
    document.querySelectorAll(".modal-nav-item").forEach((n) => n.classList.remove("active"));
    document.querySelectorAll(".modal-pane").forEach((p) => p.classList.remove("active"));
    item.classList.add("active");
    document.querySelector(`.modal-pane[data-pane="${pane}"]`)?.classList.add("active");
  });
});

function openSettingsPane(pane: string) {
  modal.classList.remove("hidden");
  document.querySelectorAll(".modal-nav-item").forEach((n) => n.classList.remove("active"));
  document.querySelectorAll(".modal-pane").forEach((p) => p.classList.remove("active"));
  document.querySelector(`.modal-nav-item[data-pane="${pane}"]`)?.classList.add("active");
  document.querySelector(`.modal-pane[data-pane="${pane}"]`)?.classList.add("active");
}

function openPlans(e?: Event) {
  e?.preventDefault();
  openSettingsPane("plans");
  refreshStorefront().catch((err) => console.error("refreshStorefront:", err));
}

function openDictionary() {
  modal.classList.add("hidden");
  document.querySelectorAll(".nav-item").forEach((n) => n.classList.remove("active"));
  document.querySelectorAll<HTMLElement>(".view").forEach((s) => s.classList.remove("active"));
  document.querySelector('.nav-item[data-view="dictionary"]')?.classList.add("active");
  document.querySelector('.view[data-view="dictionary"]')?.classList.add("active");
}

function openSnippets() {
  modal.classList.add("hidden");
  document.querySelectorAll(".nav-item").forEach((n) => n.classList.remove("active"));
  document.querySelectorAll<HTMLElement>(".view").forEach((s) => s.classList.remove("active"));
  document.querySelector('.nav-item[data-view="snippets"]')?.classList.add("active");
  document.querySelector('.view[data-view="snippets"]')?.classList.add("active");
}

function openStyle() {
  modal.classList.add("hidden");
  document.querySelectorAll(".nav-item").forEach((n) => n.classList.remove("active"));
  document.querySelectorAll<HTMLElement>(".view").forEach((s) => s.classList.remove("active"));
  document.querySelector('.nav-item[data-view="style"]')?.classList.add("active");
  document.querySelector('.view[data-view="style"]')?.classList.add("active");
}

function openTransforms() {
  modal.classList.add("hidden");
  document.querySelectorAll(".nav-item").forEach((n) => n.classList.remove("active"));
  document.querySelectorAll<HTMLElement>(".view").forEach((s) => s.classList.remove("active"));
  document.querySelector('.nav-item[data-view="transforms"]')?.classList.add("active");
  document.querySelector('.view[data-view="transforms"]')?.classList.add("active");
}

function openScratchpad() {
  modal.classList.add("hidden");
  document.querySelectorAll(".nav-item").forEach((n) => n.classList.remove("active"));
  document.querySelectorAll<HTMLElement>(".view").forEach((s) => s.classList.remove("active"));
  document.querySelector('.nav-item[data-view="scratchpad"]')?.classList.add("active");
  document.querySelector('.view[data-view="scratchpad"]')?.classList.add("active");
}

function openInsights() {
  modal.classList.add("hidden");
  document.querySelectorAll(".nav-item").forEach((n) => n.classList.remove("active"));
  document.querySelectorAll<HTMLElement>(".view").forEach((s) => s.classList.remove("active"));
  document.querySelector('.nav-item[data-view="insights"]')?.classList.add("active");
  document.querySelector('.view[data-view="insights"]')?.classList.add("active");
}

function openAccount(e?: Event) {
  e?.preventDefault();
  openSettingsPane("account");
  refreshStikiSession().catch((err) => console.error("refreshStikiSession:", err));
}
$("open-pro").addEventListener("click", openPlans);
$("cta-pro").addEventListener("click", openPlans);
document.querySelectorAll(".pro-activate").forEach((b) =>
  b.addEventListener("click", (e) => {
    e.preventDefault();
    askProUnlock();
  })
);

// Help button in sidebar footer → switch main view to help
$("open-help").addEventListener("click", () => {
  document.querySelectorAll(".nav-item").forEach((n) => n.classList.remove("active"));
  document.querySelectorAll<HTMLElement>(".view").forEach((s) => s.classList.remove("active"));
  document.querySelector('.view[data-view="help"]')?.classList.add("active");
});

// Auto-hide first-time setup card once the user has completed a full dictation.
// A successful Recording → Transcribing → Ready cycle proves mic + accessibility +
// automation permissions all worked, so the prompts won't fire again.
const SETUP_DONE_KEY = "mabel.setupComplete";
const setupCard = document.getElementById("setup-card");
function hideSetupCardIfDone() {
  if (setupCard && localStorage.getItem(SETUP_DONE_KEY) === "1") {
    setupCard.style.display = "none";
  }
}
hideSetupCardIfDone();

let currentSettings: Settings;
let llmRuntimeAvailable = false;
const STREAMING_AVAILABLE = false;

const WHISPER_VARIANTS = [
  { modelSize: "large-v3", language: "en" },
  { modelSize: "large-v3", language: "multi" },
  { modelSize: "small", language: "en" },
  { modelSize: "small", language: "multi" },
  { modelSize: "medium", language: "en" },
  { modelSize: "medium", language: "multi" },
];

async function loadSettings() {
  currentSettings = await invoke<Settings>("get_settings");

  const mics = await invoke<MicDevice[]>("list_microphones");
  micSelect.innerHTML = "";
  mics.forEach((mic) => {
    const option = document.createElement("option");
    option.value = mic.name;
    option.textContent = mic.name + (mic.is_default ? " (default)" : "");
    micSelect.appendChild(option);
  });
  micSelect.value = currentSettings.microphone || mics.find((m) => m.is_default)?.name || "";

  setEngine(currentSettings.engine);
  await populateLocalEngines();
  localEngineSelect.value = currentSettings.localEngine || "parakeet";
  applyLocalEngineUi();
  modelSelect.value = currentSettings.whisperModel;
  languageSelect.value = currentSettings.whisperLanguage || "multi";
  await checkModelStatus();
  const whisperCppOnDisk = await invoke<boolean>("check_model_downloaded", {
    modelSize: modelSelect.value,
    language: languageSelect?.value || currentSettings.whisperLanguage || "multi",
  });
  if (whisperCppOnDisk) {
    invoke("ensure_vad_model").catch((e) => console.error("VAD model download:", e));
  }
  renderDictionary();
  llmRuntimeAvailable = await invoke<boolean>("llm_runtime_available");
  currentSettings.polishMode = currentSettings.polishMode || "off";
  applyPolishUi();
  llmModelSelect.value = currentSettings.llmModel || "standard";
  applyCleanupModeUi();
  await checkLlmModelStatus();
  // The actual key is never echoed back from the keychain. We just show
  // "Saved" if a key was previously stored, and let the user overwrite it.
  groqKey.value = "";
  groqKey.placeholder = currentSettings.groqKeyConfigured ? "•••••••••••••••• (stored)" : "gsk_...";
  keyStatus.classList.toggle("hidden", !currentSettings.groqKeyConfigured);
  setRecordingMode(currentSettings.recordingMode);
  if (!STREAMING_AVAILABLE && currentSettings.streaming) {
    currentSettings.streaming = false;
    await saveSettings();
  }
  streamingToggle.disabled = !STREAMING_AVAILABLE;
  streamingToggle.setAttribute("aria-disabled", String(!STREAMING_AVAILABLE));
  streamingToggle.setAttribute("aria-checked", String(STREAMING_AVAILABLE && currentSettings.streaming));
  setSwitch(autostartToggle, currentSettings.launchAtLogin);
  setSwitch(dockToggle, currentSettings.showInDock);
  setSwitch(companionToggle, currentSettings.companionEnabled);
  companionSizeSelect.value = currentSettings.companionSize || "medium";
  companionFrequencySelect.value = currentSettings.companionFrequency || "30min";
  companionVisitSelect.value = currentSettings.companionVisit || "medium";
  applyCompanionUi();
  setSwitch(soundsToggle, currentSettings.dictationSounds);
  setSwitch(pressEnterToggle, currentSettings.pressEnterCommand);
  setSwitch(clipboardHistoryToggle, !!currentSettings.clipboardHistoryEnabled);

  const formatted = formatHotkey(currentSettings.hotkey);
  hotkeyText.textContent = formatted;
  homeHotkey.textContent = formatted;
}

async function loadVersion() {
  try {
    const info = await invoke<VersionInfo>("get_version");
    const footer = document.getElementById("footer-version");
    if (footer) footer.textContent = `v${info.version} · ${info.gitHash} · © Chibitek Labs`;
    const about = document.getElementById("about-version");
    if (about) about.textContent = `Version ${info.version} · ${info.gitHash}`;
  } catch (e) {
    console.error("get_version failed:", e);
  }
}

async function installUpdate(update: Update, statusEl: HTMLElement) {
  const next = update.version ? `v${update.version}` : "the latest version";
  statusEl.textContent = `Downloading ${next}...`;
  updateInstallBtn.disabled = true;
  let downloaded = 0;
  let contentLength = 0;
  await update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      contentLength = event.data.contentLength ?? 0;
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      if (contentLength > 0) {
        const pct = Math.min(100, Math.round((downloaded / contentLength) * 100));
        statusEl.textContent = `Downloading ${next}... ${pct}%`;
      }
    } else if (event.event === "Finished") {
      statusEl.textContent = "Installing update...";
    }
  });

  statusEl.textContent = "Update installed. Relaunching...";
  await relaunch();
}

function showUpdatePrompt(update: Update) {
  pendingUpdate = update;
  updateVersion.textContent = `v${update.version} is available`;
  updateBody.innerHTML = renderChangelog(
    update.body || "A new signed Mabel update is ready to install."
  );
  updateInstallBtn.disabled = false;
  updateModal.classList.remove("hidden");
}

async function checkForUpdates(showPrompt: boolean) {
  if (!showPrompt) {
    checkUpdatesBtn.disabled = true;
    updateStatus.textContent = "Checking...";
  }
  try {
    const update = await check({ timeout: 30000 });
    if (!update) {
      if (!showPrompt) updateStatus.textContent = "Mabel is up to date.";
      return;
    }

    if (showPrompt) {
      showUpdatePrompt(update);
    } else {
      await installUpdate(update, updateStatus);
    }
  } catch (e) {
    console.error("update check failed:", e);
    if (!showPrompt) updateStatus.textContent = `Update failed: ${String(e)}`;
  } finally {
    if (!showPrompt) checkUpdatesBtn.disabled = false;
  }
}

checkUpdatesBtn.addEventListener("click", () => {
  checkForUpdates(false);
});

updateLaterBtn.addEventListener("click", () => {
  updateModal.classList.add("hidden");
});

updateInstallBtn.addEventListener("click", () => {
  if (!pendingUpdate) return;
  installUpdate(pendingUpdate, updateBody).catch((e) => {
    console.error("update install failed:", e);
    updateBody.textContent = `Update failed: ${String(e)}`;
    updateInstallBtn.disabled = false;
  });
});

const insWpm = document.getElementById("ins-wpm");
const insTotalWords = document.getElementById("ins-total-words");
const insToday = document.getElementById("ins-today");
const insTotal = document.getElementById("ins-total");
const insStreak = document.getElementById("ins-streak");
const insTimeSaved = document.getElementById("ins-time-saved");
const streakGrid = document.getElementById("streak-grid");
const statToday = document.getElementById("stat-today");
const statTotal = document.getElementById("stat-total");
const statStreak = document.getElementById("stat-streak");

function fmt(n: number): string {
  return n.toLocaleString();
}

function resetStatsDisplay() {
  if (statToday) statToday.textContent = "0";
  if (statTotal) statTotal.textContent = "0";
  if (statStreak) statStreak.textContent = "0";
  if (insToday) insToday.textContent = "0";
  if (insWpm) insWpm.textContent = "0";
  if (insTotalWords) insTotalWords.textContent = "0";
  if (insTotal) insTotal.textContent = "0";
  if (insStreak) insStreak.textContent = "0";
  if (insTimeSaved) insTimeSaved.textContent = "0";
  if (streakGrid) streakGrid.innerHTML = "";
}

function renderStatsSummary(s: StatsSummary) {
  if (statToday) statToday.textContent = fmt(s.today);
  if (statTotal) statTotal.textContent = fmt(s.total);
  if (statStreak) statStreak.textContent = fmt(s.streak);
  if (insToday) insToday.textContent = fmt(s.today);
  if (insWpm) insWpm.textContent = fmt(s.wpm);
  if (insTotalWords) insTotalWords.textContent = fmt(s.total_words);
  if (insTotal) insTotal.textContent = fmt(s.total);
  if (insStreak) insStreak.textContent = fmt(s.streak);
  if (insTimeSaved) insTimeSaved.textContent = fmt(s.time_saved_minutes);
}

async function loadStats() {
  if (!insightsSurfaceReady()) {
    resetStatsDisplay();
    return;
  }
  try {
    const s = await invoke<StatsSummary>("get_stats");
    renderStatsSummary(s);
    if (streakGrid) {
      const max = Math.max(1, ...s.last30);
      streakGrid.innerHTML = "";
      s.last30.forEach((count) => {
        const cell = document.createElement("span");
        cell.className = "streak-cell";
        if (count === 0) cell.classList.add("l0");
        else {
          const pct = count / max;
          if (pct > 0.75) cell.classList.add("l4");
          else if (pct > 0.5) cell.classList.add("l3");
          else if (pct > 0.25) cell.classList.add("l2");
          else cell.classList.add("l1");
        }
        cell.title = `${count} dictation${count === 1 ? "" : "s"}`;
        streakGrid.appendChild(cell);
      });
    }
  } catch (e) {
    console.error("get_stats failed:", e);
  }
}

function setEngine(engine: string) {
  currentSettings.engine = engine;
  engineLocal.classList.toggle("active", engine === "local");
  engineCloud.classList.toggle("active", engine === "cloud");
  localSettings.classList.toggle("hidden", engine !== "local");
  cloudSettings.classList.toggle("hidden", engine !== "cloud");
}

function setRecordingMode(mode: string) {
  currentSettings.recordingMode = mode;
  modeToggle.classList.toggle("active", mode === "toggle");
  modePtt.classList.toggle("active", mode === "push-to-talk");
}

interface LocalEngineInfo {
  id: string;
  label: string;
  available: boolean;
  mas_clean: boolean;
}

async function populateLocalEngines() {
  const engines = await invoke<LocalEngineInfo[]>("list_local_engines");
  const current = localEngineSelect.value;
  localEngineSelect.innerHTML = "";
  for (const engine of engines) {
    const option = document.createElement("option");
    option.value = engine.id;
    option.textContent = engine.label;
    option.disabled = !engine.available;
    localEngineSelect.appendChild(option);
  }
  if (engines.some((e) => e.id === current && e.available)) {
    localEngineSelect.value = current;
  } else {
    const fallback = engines.find((e) => e.available);
    if (fallback) localEngineSelect.value = fallback.id;
  }
}

function applyLocalEngineUi() {
  const engine = localEngineSelect.value || "parakeet";
  currentSettings.localEngine = engine;
  whisperCppSettings.classList.toggle("hidden", engine !== "whisper-cpp");
}

async function checkModelStatus(): Promise<boolean> {
  const engine = localEngineSelect?.value || currentSettings.localEngine || "parakeet";
  const downloaded = await invoke<boolean>("check_local_engine_ready", {
    engine,
    language: languageSelect?.value || currentSettings.whisperLanguage || "en",
  });
  downloadBtn.textContent = downloaded ? "Downloaded" : "Download";
  downloadBtn.disabled = downloaded;
  return downloaded;
}

async function checkLlmModelStatus() {
  const downloaded = await invoke<boolean>("check_llm_model_downloaded", {
    model: llmModelSelect.value,
  });
  llmDownloadBtn.textContent = downloaded ? "Downloaded" : "Download";
  llmDownloadBtn.disabled = downloaded;
}

function applyCleanupModeUi() {
  const live = isLivePolish(displayedPolishMode());
  cleanupModeSelect.value = live ? "llm" : "rules";
  const llmOption = cleanupModeSelect.querySelector('option[value="llm"]') as HTMLOptionElement | null;
  if (llmOption) {
    llmOption.disabled = !llmRuntimeAvailable;
    llmOption.textContent = llmRuntimeAvailable ? "AI cleanup (local)" : "AI cleanup (runtime unavailable)";
  }
  llmSettings.classList.toggle("hidden", !live || !llmRuntimeAvailable);
}

function isLivePolish(mode: string): boolean {
  return mode === "casual" || mode === "professional" || mode === "polite";
}

function displayedPolishMode(): string {
  const stored = currentSettings.polishMode || "off";
  if (isLivePolish(stored) && !proSurfacesUnlocked()) return "off";
  return stored;
}

function applyPolishUi() {
  if (!currentSettings || !polishModeSelect) return;
  const entitled = proSurfacesUnlocked();
  const mode = displayedPolishMode();
  const live = isLivePolish(mode) && entitled;
  polishModeSelect.value = entitled ? mode : "off";
  if (polishToggle) setSwitch(polishToggle, live);
  if (polishModeRow) polishModeRow.classList.toggle("hidden", !entitled);
  applyCleanupModeUi();
}

async function saveSettings() {
  // Most settings saves should never carry the key — we don't want every mic
  // change to trip the keychain prompt on unsigned builds. The key is saved
  // explicitly via the Save button next to the input.
  currentSettings.microphone = micSelect.value;
  currentSettings.localEngine = localEngineSelect.value;
  currentSettings.whisperModel = modelSelect.value;
  currentSettings.whisperLanguage = languageSelect.value;
  currentSettings.cleanupMode = cleanupModeSelect.value;
  currentSettings.llmModel = llmModelSelect.value;
  currentSettings.polishMode = displayedPolishMode();
  currentSettings.companionSize = companionSizeSelect.value;
  currentSettings.companionFrequency = companionFrequencySelect.value;
  currentSettings.companionVisit = companionVisitSelect.value;
  const previousKey = currentSettings.groqApiKey;
  currentSettings.groqApiKey = "";
  await invoke("save_settings", { settings: currentSettings });
  currentSettings.groqApiKey = previousKey;
}

async function saveGroqKey() {
  const value = groqKey.value.trim();
  if (!value) return;
  const settingsWithKey = { ...currentSettings, groqApiKey: value };
  try {
    await invoke("save_settings", { settings: settingsWithKey });
    currentSettings.groqKeyConfigured = true;
    groqKey.value = "";
    groqKey.placeholder = "•••••••••••••••• (stored)";
    keyStatus.textContent = "Saved";
    keyStatus.classList.remove("hidden");
    keyStatus.classList.remove("flash");
    void keyStatus.offsetWidth; // restart animation
    keyStatus.classList.add("flash");
  } catch (e) {
    console.error("save groq key failed:", e);
    keyStatus.textContent = "Error";
    keyStatus.classList.remove("hidden");
  }
}

engineLocal.addEventListener("click", () => { setEngine("local"); saveSettings(); });
engineCloud.addEventListener("click", () => { setEngine("cloud"); saveSettings(); });
micSelect.addEventListener("change", () => saveSettings());
localEngineSelect.addEventListener("change", async () => {
  applyLocalEngineUi();
  await checkModelStatus();
  await saveSettings();
});
modelSelect.addEventListener("change", async () => { await checkModelStatus(); saveSettings(); });
languageSelect.addEventListener("change", async () => { await checkModelStatus(); saveSettings(); });

// Pro dictionary editor. Terms stay in local config and feed the existing
// whisper.cpp --prompt hook plus local Gemma spelling hints. Locked unless
// StoreKit Pro AND a Stiki session. Free dictation does not use this gate.
const dictInput = $<HTMLInputElement>("dict-input");
const dictAddBtn = $<HTMLButtonElement>("dict-add-btn");
const dictCancelBtn = $<HTMLButtonElement>("dict-cancel-btn");
const dictList = $("dict-list");
const dictEmpty = $("dict-empty");
const dictError = $("dict-error");
let editingTerm: string | null = null;

function showDictError(message: string) {
  if (!dictError) return;
  dictError.textContent = message;
  dictError.hidden = !message;
}

function applyDictionary(words: string[]) {
  if (!currentSettings) return;
  currentSettings.dictionary = words;
  renderDictionary();
}

function renderDictionary() {
  if (!dictList || !currentSettings) return;
  const words = currentSettings.dictionary || [];
  dictList.innerHTML = "";
  if (words.length === 0) {
    dictEmpty.classList.remove("hidden");
    return;
  }
  dictEmpty.classList.add("hidden");
  for (const word of words) {
    const chip = document.createElement("span");
    chip.className = "dict-chip";
    if (editingTerm && editingTerm.toLowerCase() === word.toLowerCase()) {
      chip.classList.add("editing");
    }
    const text = document.createElement("span");
    text.textContent = word;
    const edit = document.createElement("button");
    edit.type = "button";
    edit.setAttribute("aria-label", `Edit ${word}`);
    edit.textContent = "Edit";
    edit.addEventListener("click", () => beginEditTerm(word));
    const remove = document.createElement("button");
    remove.type = "button";
    remove.setAttribute("aria-label", `Remove ${word}`);
    remove.textContent = "×";
    remove.addEventListener("click", () => {
      void removeDictionaryEntry(word);
    });
    chip.appendChild(text);
    chip.appendChild(edit);
    chip.appendChild(remove);
    dictList.appendChild(chip);
  }
}

function beginEditTerm(word: string) {
  if (!dictionarySurfaceReady()) {
    if (!currentEntitlement.entitled) openPlans();
    return;
  }
  editingTerm = word;
  dictInput.value = word;
  dictAddBtn.textContent = "Save";
  dictCancelBtn.classList.remove("hidden");
  showDictError("");
  renderDictionary();
  dictInput.focus();
  dictInput.select();
}

function cancelEditTerm() {
  editingTerm = null;
  dictInput.value = "";
  dictAddBtn.textContent = "Add";
  dictCancelBtn.classList.add("hidden");
  showDictError("");
  renderDictionary();
}

async function addOrSaveDictionaryEntry() {
  if (!dictionarySurfaceReady()) {
    if (!currentEntitlement.entitled) openPlans();
    return;
  }
  const raw = dictInput.value.trim();
  if (!raw) return;
  showDictError("");
  try {
    if (editingTerm) {
      const next = await invoke<string[]>("dictionary_update", { from: editingTerm, to: raw });
      cancelEditTerm();
      applyDictionary(next);
    } else {
      const next = await invoke<string[]>("dictionary_add", { term: raw });
      dictInput.value = "";
      applyDictionary(next);
    }
  } catch (e) {
    showDictError(String(e));
    console.error("dictionary mutate:", e);
  }
}

async function removeDictionaryEntry(word: string) {
  if (!dictionarySurfaceReady()) {
    if (!currentEntitlement.entitled) openPlans();
    return;
  }
  showDictError("");
  try {
    const next = await invoke<string[]>("dictionary_remove", { term: word });
    if (editingTerm && editingTerm.toLowerCase() === word.toLowerCase()) {
      cancelEditTerm();
    }
    applyDictionary(next);
  } catch (e) {
    showDictError(String(e));
    console.error("dictionary_remove:", e);
  }
}

$("dictionary-open").addEventListener("click", openDictionary);
$("dictionary-activate").addEventListener("click", openPlans);
$("dictionary-stiki").addEventListener("click", () => {
  void signInWithStiki();
});
$("dict-pane-stiki").addEventListener("click", () => {
  void signInWithStiki();
});
$("snippets-open").addEventListener("click", openSnippets);
$("snippets-activate").addEventListener("click", openPlans);
$("snippets-stiki").addEventListener("click", () => {
  void signInWithStiki();
});
$("snippet-pane-stiki").addEventListener("click", () => {
  void signInWithStiki();
});
$("style-open").addEventListener("click", openStyle);
$("style-activate").addEventListener("click", openPlans);
$("style-stiki").addEventListener("click", () => {
  void signInWithStiki();
});
$("style-pane-stiki").addEventListener("click", () => {
  void signInWithStiki();
});
$("transforms-open").addEventListener("click", openTransforms);
$("transforms-activate").addEventListener("click", openPlans);
$("transforms-stiki").addEventListener("click", () => {
  void signInWithStiki();
});
$("xf-pane-stiki").addEventListener("click", () => {
  void signInWithStiki();
});
$("scratchpad-open").addEventListener("click", openScratchpad);
$("scratchpad-activate").addEventListener("click", openPlans);
$("scratchpad-stiki").addEventListener("click", () => {
  void signInWithStiki();
});
$("scratch-pane-stiki").addEventListener("click", () => {
  void signInWithStiki();
});
$("insights-open").addEventListener("click", openInsights);
$("insights-activate").addEventListener("click", openPlans);
$("insights-stiki").addEventListener("click", () => {
  void signInWithStiki();
});
$("insights-pane-stiki").addEventListener("click", () => {
  void signInWithStiki();
});

dictAddBtn.addEventListener("click", () => {
  void addOrSaveDictionaryEntry();
});
dictCancelBtn.addEventListener("click", cancelEditTerm);
dictInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    void addOrSaveDictionaryEntry();
  }
  if (e.key === "Escape") {
    e.preventDefault();
    cancelEditTerm();
  }
});

downloadBtn.addEventListener("click", async () => {
  downloadBtn.disabled = true;
  downloadProgress.classList.remove("hidden");
  progressFill.style.width = "0%";
  try {
    await invoke("download_local_engine", {
      engine: localEngineSelect.value || currentSettings.localEngine || "parakeet",
      language: languageSelect?.value || currentSettings.whisperLanguage || "en",
    });
    downloadBtn.textContent = "Downloaded";
  } catch (e) {
    downloadBtn.textContent = "Retry";
    downloadBtn.disabled = false;
    console.error("Download failed:", e);
  }
  downloadProgress.classList.add("hidden");
});

cleanupModeSelect.addEventListener("change", async () => {
  if (cleanupModeSelect.value === "llm" && !llmRuntimeAvailable) {
    cleanupModeSelect.value = "rules";
  }
  applyCleanupModeUi();
  await saveSettings();
  if (cleanupModeSelect.value === "llm" && llmRuntimeAvailable) {
    invoke("ensure_llm_started").catch((e) => console.error("LLM warm start:", e));
  }
});

async function persistPolishMode(next: string) {
  if (isLivePolish(next) && !proSurfacesUnlocked()) {
    currentSettings.polishMode = "off";
    applyPolishUi();
    askProUnlock();
    return;
  }
  if (isLivePolish(next) && !llmRuntimeAvailable) {
    currentSettings.polishMode = "off";
    applyPolishUi();
    return;
  }
  currentSettings.polishMode = next;
  applyPolishUi();
  try {
    await invoke("polish_set", { mode: next });
    currentSettings.polishMode = next;
    currentSettings.cleanupMode = isLivePolish(next) ? "llm" : currentSettings.cleanupMode;
  } catch (e) {
    console.error("polish_set:", e);
    currentSettings.polishMode = "off";
    applyPolishUi();
    if (isLivePolish(next)) askProUnlock(String(e));
    return;
  }
  if (isLivePolish(next) && llmRuntimeAvailable) {
    invoke("ensure_llm_started").catch((err) => console.error("LLM warm start:", err));
  }
}

polishToggle.addEventListener("click", () => {
  if (!proSurfacesUnlocked()) {
    setSwitch(polishToggle, false);
    askProUnlock();
    return;
  }
  const turningOn = polishToggle.getAttribute("aria-checked") !== "true";
  const next = turningOn
    ? (isLivePolish(currentSettings.polishMode) ? currentSettings.polishMode : "casual")
    : "off";
  void persistPolishMode(next);
});

polishModeSelect.addEventListener("change", () => {
  void persistPolishMode(polishModeSelect.value);
});

llmModelSelect.addEventListener("change", async () => {
  await checkLlmModelStatus();
  await saveSettings();
});

llmDownloadBtn.addEventListener("click", async () => {
  llmDownloadBtn.disabled = true;
  llmDownloadProgress.classList.remove("hidden");
  llmProgressFill.style.width = "0%";
  try {
    await invoke("download_llm_model", { model: llmModelSelect.value });
    llmDownloadBtn.textContent = "Downloaded";
  } catch (e) {
    llmDownloadBtn.textContent = "Retry";
    llmDownloadBtn.disabled = false;
    console.error("LLM download failed:", e);
  }
  llmDownloadProgress.classList.add("hidden");
});

keySave.addEventListener("click", () => saveGroqKey());
groqKey.addEventListener("keydown", (e) => {
  if (e.key === "Enter") saveGroqKey();
});
modeToggle.addEventListener("click", () => { setRecordingMode("toggle"); saveSettings(); });
modePtt.addEventListener("click", () => { setRecordingMode("push-to-talk"); saveSettings(); });

streamingToggle.addEventListener("click", () => {
  if (!STREAMING_AVAILABLE) return;
  const next = streamingToggle.getAttribute("aria-checked") !== "true";
  streamingToggle.setAttribute("aria-checked", String(next));
  currentSettings.streaming = next;
  saveSettings();
});

const autostartToggle = $<HTMLButtonElement>("autostart-toggle");
const dockToggle = $<HTMLButtonElement>("dock-toggle");
const soundsToggle = $<HTMLButtonElement>("sounds-toggle");
const pressEnterToggle = $<HTMLButtonElement>("press-enter-toggle");
const companionToggle = $<HTMLButtonElement>("companion-toggle");
const companionSettings = $("companion-settings");
const companionFrequencyRow = $("companion-frequency-row");
const companionVisitRow = $("companion-visit-row");
const companionTestRow = $("companion-test-row");
const companionSizeSelect = $<HTMLSelectElement>("companion-size-select");
const companionFrequencySelect = $<HTMLSelectElement>("companion-frequency-select");
const companionVisitSelect = $<HTMLSelectElement>("companion-visit-select");
const companionTestBtn = $<HTMLButtonElement>("companion-test-btn");
const clipboardHistoryToggle = $<HTMLButtonElement>("clipboard-history-toggle");
const clipboardHistoryClear = $<HTMLButtonElement>("clipboard-history-clear");
const clipboardHistoryOpen = $<HTMLButtonElement>("clipboard-history-open");

function applyCompanionUi() {
  const on = companionToggle.getAttribute("aria-checked") === "true";
  for (const row of [companionSettings, companionFrequencyRow, companionVisitRow, companionTestRow]) {
    row.classList.toggle("hidden", !on);
  }
}

function setSwitch(btn: HTMLButtonElement, on: boolean) {
  btn.setAttribute("aria-checked", String(on));
}

autostartToggle.addEventListener("click", async () => {
  const next = autostartToggle.getAttribute("aria-checked") !== "true";
  setSwitch(autostartToggle, next);
  currentSettings.launchAtLogin = next;
  try {
    await invoke("set_launch_at_login", { enabled: next });
    await saveSettings();
  } catch (e) {
    console.error("set_launch_at_login failed:", e);
    setSwitch(autostartToggle, !next);
    currentSettings.launchAtLogin = !next;
  }
});

dockToggle.addEventListener("click", async () => {
  const next = dockToggle.getAttribute("aria-checked") !== "true";
  setSwitch(dockToggle, next);
  currentSettings.showInDock = next;
  try {
    await invoke("set_show_in_dock", { show: next });
    await saveSettings();
  } catch (e) {
    console.error("set_show_in_dock failed:", e);
  }
});

soundsToggle.addEventListener("click", () => {
  const next = soundsToggle.getAttribute("aria-checked") !== "true";
  setSwitch(soundsToggle, next);
  currentSettings.dictationSounds = next;
  saveSettings();
});

pressEnterToggle.addEventListener("click", () => {
  const next = pressEnterToggle.getAttribute("aria-checked") !== "true";
  setSwitch(pressEnterToggle, next);
  currentSettings.pressEnterCommand = next;
  saveSettings();
});

companionToggle.addEventListener("click", () => {
  const next = companionToggle.getAttribute("aria-checked") !== "true";
  setSwitch(companionToggle, next);
  currentSettings.companionEnabled = next;
  applyCompanionUi();
  saveSettings();
});

for (const sel of [companionSizeSelect, companionFrequencySelect, companionVisitSelect]) {
  sel.addEventListener("change", () => {
    currentSettings.companionSize = companionSizeSelect.value;
    currentSettings.companionFrequency = companionFrequencySelect.value;
    currentSettings.companionVisit = companionVisitSelect.value;
    saveSettings();
  });
}

companionTestBtn.addEventListener("click", () => {
  invoke("companion_visit_now").catch((e) => console.error("companion test:", e));
});

clipboardHistoryToggle.addEventListener("click", async () => {
  const next = clipboardHistoryToggle.getAttribute("aria-checked") !== "true";
  setSwitch(clipboardHistoryToggle, next);
  currentSettings.clipboardHistoryEnabled = next;
  try {
    await invoke("clipboard_history_set_enabled", { enabled: next });
  } catch (e) {
    console.error("clipboard_history_set_enabled:", e);
    setSwitch(clipboardHistoryToggle, !next);
    currentSettings.clipboardHistoryEnabled = !next;
  }
});

clipboardHistoryClear.addEventListener("click", () => {
  invoke("clipboard_history_clear").catch((e) => console.error("clipboard_history_clear:", e));
});

clipboardHistoryOpen.addEventListener("click", () => {
  invoke("show_clipboard_history").catch((e) => console.error("show_clipboard_history:", e));
});

listen("clipboard-history-updated", async () => {
  try {
    const list = await invoke<{ enabled: boolean }>("clipboard_history_list");
    currentSettings.clipboardHistoryEnabled = list.enabled;
    setSwitch(clipboardHistoryToggle, list.enabled);
  } catch (e) {
    console.error("clipboard-history-updated:", e);
  }
}).catch((e) => console.error("listen clipboard-history-updated:", e));

function formatHotkey(accelerator: string): string {
  return accelerator.replace("CmdOrCtrl", "Cmd");
}

function eventToAccelerator(e: KeyboardEvent): string | null {
  if (["Control", "Meta", "Alt", "Shift"].includes(e.key)) return null;
  const parts: string[] = [];
  if (e.metaKey || e.ctrlKey) parts.push("CmdOrCtrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  const code = e.code;
  let key: string | null = null;
  if (code === "Space") key = "Space";
  else if (code === "Enter") key = "Enter";
  else if (code === "Tab") key = "Tab";
  else if (code === "Escape") key = "Escape";
  else if (code === "Backspace") key = "Backspace";
  else if (code === "Delete") key = "Delete";
  else if (code.startsWith("Arrow")) key = code.slice(5);
  else if (/^F\d{1,2}$/.test(code)) key = code;
  else if (code.startsWith("Key")) key = code.slice(3);
  else if (code.startsWith("Digit")) key = code.slice(5);
  else if (code === "Minus") key = "-";
  else if (code === "Equal") key = "=";
  else if (code === "BracketLeft") key = "[";
  else if (code === "BracketRight") key = "]";
  else if (code === "Backslash") key = "\\";
  else if (code === "Semicolon") key = ";";
  else if (code === "Quote") key = "'";
  else if (code === "Comma") key = ",";
  else if (code === "Period") key = ".";
  else if (code === "Slash") key = "/";
  else if (code === "Backquote") key = "`";
  else return null;
  const isFunctionKey = /^F\d{1,2}$/.test(key);
  if (parts.length === 0 && !isFunctionKey) return null;
  parts.push(key);
  return parts.join("+");
}

let capturingHotkey = false;
hotkeyText.addEventListener("click", () => {
  if (capturingHotkey) return;
  capturingHotkey = true;
  hotkeyText.classList.add("capturing");
  const previousText = hotkeyText.textContent ?? "";
  hotkeyText.textContent = "Press keys...";

  const cleanup = () => {
    capturingHotkey = false;
    hotkeyText.classList.remove("capturing");
    document.removeEventListener("keydown", onKey, true);
  };

  const onKey = async (e: KeyboardEvent) => {
    if (e.key === "Escape" && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      hotkeyText.textContent = previousText;
      cleanup();
      return;
    }
    const accelerator = eventToAccelerator(e);
    if (!accelerator) {
      e.preventDefault();
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    try {
      await invoke("update_hotkey", { hotkey: accelerator });
      currentSettings.hotkey = accelerator;
      const formatted = formatHotkey(accelerator);
      hotkeyText.textContent = formatted;
      homeHotkey.textContent = formatted;
    } catch (err) {
      console.error("update_hotkey failed:", err);
      hotkeyText.textContent = previousText;
      alert(`Couldn't bind that combination: ${err}`);
    } finally {
      cleanup();
    }
  };

  document.addEventListener("keydown", onKey, true);
});

let lastRecordingState = "Ready";
let lastTranscriptionWasLowConfidence = false;
listen<TranscriptionQuality>("transcription-quality", (event) => {
  lastTranscriptionWasLowConfidence = event.payload.lowConfidence;
});

listen<string>("recording-state", (event) => {
  const state = event.payload;
  statusDot.className = "status-dot";
  if (state === "Recording") {
    statusDot.classList.add("recording");
    statusText.textContent = "Recording";
  } else if (state === "Transcribing") {
    statusDot.classList.add("transcribing");
    statusText.textContent = "Transcribing";
  } else {
    statusText.textContent = lastTranscriptionWasLowConfidence
      ? "Ready · Check the last transcription"
      : "Ready";
    if (lastTranscriptionWasLowConfidence) {
      window.setTimeout(() => {
        if (statusText.textContent === "Ready · Check the last transcription") {
          statusText.textContent = "Ready";
        }
      }, 6000);
      lastTranscriptionWasLowConfidence = false;
    }
    // Transcribing → Ready means a paste just succeeded, so all required
    // permissions are granted. Stash the flag and hide the setup card.
    if (lastRecordingState === "Transcribing" && localStorage.getItem(SETUP_DONE_KEY) !== "1") {
      localStorage.setItem(SETUP_DONE_KEY, "1");
      hideSetupCardIfDone();
    }
  }
  lastRecordingState = state;
});

listen<string | { title?: string; message?: string }>("transcription-error", (event) => {
  const payload = event.payload;
  const title =
    payload && typeof payload === "object" && payload.title
      ? payload.title
      : "Transcription failed";
  const msg =
    typeof payload === "string"
      ? payload
      : payload?.message || "Unknown transcription error";
  statusDot.className = "status-dot";
  statusText.textContent = title;
  console.error("transcription-error:", title, msg);
  alert(`${title}: ${msg}`);
});

listen<DownloadProgress>("download-progress", (event) => {
  // The same progress event drives both the Whisper and LLM download bars,
  // since only one download runs at a time. Update whichever bar is currently
  // visible (its container is .hidden when not in flight).
  progressFill.style.width = `${event.payload.percent}%`;
  llmProgressFill.style.width = `${event.payload.percent}%`;
  // Mirror to first-run modal if it's the active context.
  const firstrunFill = document.getElementById("firstrun-fill");
  const firstrunPct = document.getElementById("firstrun-pct");
  if (firstrunFill) firstrunFill.style.width = `${event.payload.percent}%`;
  if (firstrunPct) firstrunPct.textContent = `${Math.round(event.payload.percent)}%`;
});

async function maybeRunFirstTimeSetup() {
  const settings = await invoke<Settings>("get_settings");
  // Returning users: any existing whisper ggml, Parakeet, or WhisperKit
  // cache skips first-run so we don't nag a 1.2 install with a new download.
  for (const v of WHISPER_VARIANTS) {
    if (await invoke<boolean>("check_model_downloaded", v)) return;
  }
  if (await invoke<boolean>("check_local_engine_ready", { engine: "parakeet", language: "en" })) return;
  if (await invoke<boolean>("check_local_engine_ready", { engine: "parakeet", language: "multi" })) return;
  if (await invoke<boolean>("check_local_engine_ready", { engine: "whisperkit", language: "en" })) return;

  const modal = document.getElementById("firstrun-modal")!;
  const body = document.getElementById("firstrun-body")!;
  const foot = document.getElementById("firstrun-foot")!;
  const done = document.getElementById("firstrun-done") as HTMLButtonElement;
  const retry = document.getElementById("firstrun-retry") as HTMLButtonElement;
  const fill = document.getElementById("firstrun-fill")!;
  const pct = document.getElementById("firstrun-pct")!;
  modal.classList.remove("hidden");

  const startDownload = async () => {
    body.textContent = "Downloading Parakeet so dictation works fully offline on this Mac. Recommended default. This is a one-time setup.";
    foot.classList.remove("hidden");
    done.classList.add("hidden");
    retry.classList.add("hidden");
    fill.style.width = "0%";
    pct.textContent = "0%";
    // Don't proactively trigger system permission prompts here. Unsigned test
    // builds can re-prompt due to changing signatures, which is disruptive.
    // Prompts will appear when the relevant feature is actually used.
    try {
      await invoke("download_local_engine", { engine: "parakeet", language: "en" });
      currentSettings = {
        ...settings,
        engine: "local",
        localEngine: "parakeet",
        whisperModel: "large-v3",
        whisperLanguage: "en",
      };
      await invoke("save_settings", { settings: currentSettings });
      body.textContent = "Parakeet is ready. Mabel works fully offline, on this Mac. Audio never leaves the device.";
      foot.innerHTML = 'Want WhisperKit or the older whisper.cpp path? Switch anytime in <b>Settings → Engine</b>.';
      fill.style.width = "100%";
      pct.textContent = "100%";
      done.classList.remove("hidden");
    } catch (e) {
      console.error("first-run download failed:", e);
      body.textContent = "Couldn't download the model. Check your internet connection and retry.";
      foot.classList.add("hidden");
      retry.classList.remove("hidden");
    }
  };

  done.addEventListener("click", () => {
    modal.classList.add("hidden");
    loadSettings();
  });
  retry.addEventListener("click", () => startDownload());

  startDownload();
}

listen("stats-updated", () => {
  loadStats();
});

interface IntroOffer {
  paymentMode: string;
  offerType: string;
  displayPrice: string;
  period: string;
  periodCount: number;
  display: string;
}

interface StoreProduct {
  id: string;
  displayName: string;
  description: string;
  displayPrice: string;
  price: string;
  kind: string;
  subscriptionPeriod: string;
  introOffer?: IntroOffer | null;
}

interface Entitlement {
  entitled: boolean;
  status: string;
  productId?: string | null;
  isTrial: boolean;
  willAutoRenew: boolean;
  expirationDate?: string | null;
  environment?: string | null;
}

interface TeamState {
  orgName: string;
  seats: { id: string; displayName: string; email: string; role: string }[];
  invites: { id: string; email: string; token: string; createdAt: string; status: string }[];
}

interface Snippet {
  id: string;
  trigger: string;
  expansion: string;
}

interface StylePrefs {
  mode: string;
}

interface TransformPrefs {
  lastAction: string;
  lastSource: string;
  lastResult: string;
}

interface ConnectorView {
  id: string;
  name: string;
  kind: string;
  connected: boolean;
  live: boolean;
  hint: string;
}

interface ConnectorsStatus {
  entitled: boolean;
  signedIn: boolean;
  stikiHint: string;
  stikiClientWired: boolean;
  catalog: ConnectorView[];
  scratchpadIsMcpSource: boolean;
  scratchpadIsMcpSink: boolean;
  defaultAlwaysOn: boolean;
}

interface StikiSession {
  live: boolean;
  signedIn?: boolean;
  subject?: string | null;
  scopes?: string[];
  clientWired?: boolean;
  hint?: string;
  expiresAt?: string | null;
}

let currentEntitlement: Entitlement = {
  entitled: false,
  status: "none",
  isTrial: false,
  willAutoRenew: false,
};

let currentStiki: StikiSession = { live: false };

function stikiLive(session: StikiSession = currentStiki): boolean {
  return !!(session.live || session.signedIn);
}

function dictionarySurfaceReady() {
  return proSurfacesUnlocked();
}

function snippetsSurfaceReady() {
  return proSurfacesUnlocked();
}

function styleSurfaceReady() {
  return proSurfacesUnlocked();
}

function transformsSurfaceReady() {
  return proSurfacesUnlocked();
}

function scratchpadSurfaceReady() {
  return proSurfacesUnlocked();
}

function insightsSurfaceReady() {
  return proSurfacesUnlocked();
}

function proUnlocked(): boolean {
  return proSurfacesUnlocked();
}

async function signInWithStiki() {
  try {
    const session = await invoke<StikiSession>("stiki_sign_in");
    currentStiki = { ...session, live: stikiLive(session) };
  } catch (e) {
    console.error("stiki_sign_in:", e);
    await refreshStikiSession();
  }
  applyEntitlement(currentEntitlement);
}

function applyDictionaryGate() {
  const ready = dictionarySurfaceReady();
  const entitled = !!currentEntitlement.entitled;
  const signedIn = stikiLive();

  document.querySelectorAll<HTMLElement>("[data-dict-gate='lock']").forEach((el) => {
    el.classList.toggle("hidden", ready);
  });
  document.querySelectorAll<HTMLElement>("[data-dict-gate='unlock']").forEach((el) => {
    el.classList.toggle("hidden", !ready);
  });

  $("dictionary-open").classList.toggle("hidden", !ready);
  $("dictionary-activate").classList.toggle("hidden", entitled);
  $("dictionary-stiki").classList.toggle("hidden", signedIn);
  $("dict-pane-activate").classList.toggle("hidden", entitled);
  $("dict-pane-stiki").classList.toggle("hidden", signedIn);

  const dictNav = document.querySelector<HTMLElement>('.nav-item[data-view="dictionary"]');
  if (dictNav) {
    dictNav.classList.toggle("locked", !ready);
    dictNav.toggleAttribute("data-pro", true);
    const lock = dictNav.querySelector<HTMLElement>(".lock-pill");
    if (lock) lock.style.display = ready ? "none" : "";
  }
}

function applySnippetsGate() {
  const ready = snippetsSurfaceReady();
  const entitled = !!currentEntitlement.entitled;
  const signedIn = stikiLive();

  document.querySelectorAll<HTMLElement>("[data-snippet-gate='lock']").forEach((el) => {
    el.classList.toggle("hidden", ready);
  });
  document.querySelectorAll<HTMLElement>("[data-snippet-gate='unlock']").forEach((el) => {
    el.classList.toggle("hidden", !ready);
  });

  $("snippets-open").classList.toggle("hidden", !ready);
  $("snippets-activate").classList.toggle("hidden", entitled);
  $("snippets-stiki").classList.toggle("hidden", signedIn);
  $("snippet-pane-activate").classList.toggle("hidden", entitled);
  $("snippet-pane-stiki").classList.toggle("hidden", signedIn);

  const snipNav = document.querySelector<HTMLElement>('.nav-item[data-view="snippets"]');
  if (snipNav) {
    snipNav.classList.toggle("locked", !ready);
    snipNav.toggleAttribute("data-pro", true);
    const lock = snipNav.querySelector<HTMLElement>(".lock-pill");
    if (lock) lock.style.display = ready ? "none" : "";
  }
}

function applyStyleGate() {
  const ready = styleSurfaceReady();
  const entitled = !!currentEntitlement.entitled;
  const signedIn = stikiLive();

  document.querySelectorAll<HTMLElement>("[data-style-gate='lock']").forEach((el) => {
    el.classList.toggle("hidden", ready);
  });
  document.querySelectorAll<HTMLElement>("[data-style-gate='unlock']").forEach((el) => {
    el.classList.toggle("hidden", !ready);
  });

  $("style-open").classList.toggle("hidden", !ready);
  $("style-activate").classList.toggle("hidden", entitled);
  $("style-stiki").classList.toggle("hidden", signedIn);
  $("style-pane-activate").classList.toggle("hidden", entitled);
  $("style-pane-stiki").classList.toggle("hidden", signedIn);

  const styleNav = document.querySelector<HTMLElement>('.nav-item[data-view="style"]');
  if (styleNav) {
    styleNav.classList.toggle("locked", !ready);
    styleNav.toggleAttribute("data-pro", true);
    const lock = styleNav.querySelector<HTMLElement>(".lock-pill");
    if (lock) lock.style.display = ready ? "none" : "";
  }
}

function applyTransformsGate() {
  const ready = transformsSurfaceReady();
  const entitled = !!currentEntitlement.entitled;
  const signedIn = stikiLive();

  document.querySelectorAll<HTMLElement>("[data-xf-gate='lock']").forEach((el) => {
    el.classList.toggle("hidden", ready);
  });
  document.querySelectorAll<HTMLElement>("[data-xf-gate='unlock']").forEach((el) => {
    el.classList.toggle("hidden", !ready);
  });

  $("transforms-open").classList.toggle("hidden", !ready);
  $("transforms-activate").classList.toggle("hidden", entitled);
  $("transforms-stiki").classList.toggle("hidden", signedIn);
  $("xf-pane-activate").classList.toggle("hidden", entitled);
  $("xf-pane-stiki").classList.toggle("hidden", signedIn);

  const xfNav = document.querySelector<HTMLElement>('.nav-item[data-view="transforms"]');
  if (xfNav) {
    xfNav.classList.toggle("locked", !ready);
    xfNav.toggleAttribute("data-pro", true);
    const lock = xfNav.querySelector<HTMLElement>(".lock-pill");
    if (lock) lock.style.display = ready ? "none" : "";
  }
}

function applyScratchpadGate() {
  const ready = scratchpadSurfaceReady();
  const entitled = !!currentEntitlement.entitled;
  const signedIn = stikiLive();

  document.querySelectorAll<HTMLElement>("[data-scratch-gate='lock']").forEach((el) => {
    el.classList.toggle("hidden", ready);
  });
  document.querySelectorAll<HTMLElement>("[data-scratch-gate='unlock']").forEach((el) => {
    el.classList.toggle("hidden", !ready);
  });

  $("scratchpad-open").classList.toggle("hidden", !ready);
  $("scratchpad-activate").classList.toggle("hidden", entitled);
  $("scratchpad-stiki").classList.toggle("hidden", signedIn);
  $("scratch-pane-activate").classList.toggle("hidden", entitled);
  $("scratch-pane-stiki").classList.toggle("hidden", signedIn);

  const padNav = document.querySelector<HTMLElement>('.nav-item[data-view="scratchpad"]');
  if (padNav) {
    padNav.classList.toggle("locked", !ready);
    padNav.toggleAttribute("data-pro", true);
    const lock = padNav.querySelector<HTMLElement>(".lock-pill");
    if (lock) lock.style.display = ready ? "none" : "";
  }
}

function applyInsightsGate() {
  const ready = insightsSurfaceReady();
  const entitled = !!currentEntitlement.entitled;
  const signedIn = stikiLive();

  document.querySelectorAll<HTMLElement>("[data-insights-gate='lock']").forEach((el) => {
    el.classList.toggle("hidden", ready);
  });
  document.querySelectorAll<HTMLElement>("[data-insights-gate='unlock']").forEach((el) => {
    el.classList.toggle("hidden", !ready);
  });

  $("insights-open").classList.toggle("hidden", !ready);
  $("insights-activate").classList.toggle("hidden", entitled);
  $("insights-stiki").classList.toggle("hidden", signedIn);
  $("insights-pane-activate").classList.toggle("hidden", entitled);
  $("insights-pane-stiki").classList.toggle("hidden", signedIn);

  const insNav = document.querySelector<HTMLElement>('.nav-item[data-view="insights"]');
  if (insNav) {
    insNav.classList.toggle("locked", !ready);
    insNav.toggleAttribute("data-pro", true);
    const lock = insNav.querySelector<HTMLElement>(".lock-pill");
    if (lock) lock.style.display = ready ? "none" : "";
  }
}

function askProUnlock(err?: string) {
  if (err && String(err).includes("Sign in with Stiki")) {
    openAccount();
    return;
  }
  if (!currentEntitlement.entitled) openPlans();
  else openAccount();
}

function applyProLocks() {
  const unlocked = proSurfacesUnlocked();
  document.querySelectorAll<HTMLElement>(".nav-item.locked, .nav-item[data-pro]").forEach((item) => {
    const view = item.dataset.view;
    if (!view || !["dictionary", "insights", "snippets", "style", "transforms", "scratchpad", "teams", "connectors"].includes(view)) return;
    item.classList.toggle("locked", !unlocked);
    item.toggleAttribute("data-pro", true);
    const lock = item.querySelector(".lock-pill");
    if (lock) (lock as HTMLElement).style.display = unlocked ? "none" : "";
  });
  document.querySelectorAll(".pro-lock").forEach((el) => {
    if ((el as HTMLElement).dataset.dictGate || (el as HTMLElement).dataset.snippetGate || (el as HTMLElement).dataset.styleGate || (el as HTMLElement).dataset.xfGate || (el as HTMLElement).dataset.scratchGate || (el as HTMLElement).dataset.insightsGate) return;
    el.classList.toggle("hidden", unlocked);
  });
  document.querySelectorAll(".pro-unlock").forEach((el) => {
    if ((el as HTMLElement).dataset.dictGate || (el as HTMLElement).dataset.snippetGate || (el as HTMLElement).dataset.styleGate || (el as HTMLElement).dataset.xfGate || (el as HTMLElement).dataset.scratchGate || (el as HTMLElement).dataset.insightsGate) return;
    el.classList.toggle("hidden", !unlocked);
  });
  applyDictionaryGate();
  applySnippetsGate();
  applyStyleGate();
  applyTransformsGate();
  applyScratchpadGate();
  applyInsightsGate();
  if (unlocked) loadStats();
  else resetStatsDisplay();
}

function planLabel(ent: Entitlement): string {
  if (!ent.entitled) return "Free";
  if (ent.isTrial || ent.status === "trial") return "Trial";
  return "Pro";
}

function proSurfacesUnlocked(ent: Entitlement = currentEntitlement, stiki: StikiSession = currentStiki): boolean {
  return !!ent.entitled && stikiLive(stiki);
}

async function refreshStikiSession() {
  try {
    const session = await invoke<StikiSession>("stiki_session");
    currentStiki = { ...session, live: stikiLive(session) };
    const hint = document.getElementById("stiki-session-hint");
    if (hint && session.hint) hint.textContent = session.hint;
    const signIn = document.getElementById("stiki-signin") as HTMLButtonElement | null;
    const signOut = document.getElementById("stiki-signout") as HTMLButtonElement | null;
    if (signIn) signIn.disabled = stikiLive(session);
    if (signOut) signOut.disabled = !stikiLive(session);
    applyProLocks();
    applyPolishUi();
    if (proSurfacesUnlocked()) {
      loadProSurfaces().catch((err) => console.error("loadProSurfaces:", err));
    }
  } catch (e) {
    console.error("stiki_session:", e);
    currentStiki = { live: false };
  }
}

function applyEntitlement(ent: Entitlement) {
  currentEntitlement = ent;
  const entitled = !!ent.entitled;
  const unlocked = proSurfacesUnlocked(ent);
  const pill = $("brand-pill");
  const label = unlocked ? planLabel(ent) : "Free";
  pill.textContent = label;
  pill.classList.toggle("pro", label === "Pro");
  pill.classList.toggle("trial", label === "Trial");

  const cta = $("cta-pro");
  const openProLabel = document.getElementById("open-pro-label");
  cta.textContent = entitled ? "Manage Pro" : "Activate Pro";
  if (openProLabel) openProLabel.textContent = entitled ? "Manage Pro" : "Activate Pro";

  applyProLocks();

  const accountHint = document.getElementById("account-plan-hint");
  const accountPill = document.getElementById("account-plan-pill");
  if (accountPill) {
    accountPill.textContent = label;
    accountPill.classList.toggle("pro", label === "Pro");
    accountPill.classList.toggle("trial", label === "Trial");
  }
  if (accountHint) {
    if (!entitled) {
      accountHint.textContent = "Personal (Free). Dictation works without an account. Pro features need Activate Pro and Sign in with Stiki.";
    } else if (!stikiLive()) {
      accountHint.textContent = "App Store Pro is on this Mac. Sign in with Stiki to unlock Pro surfaces. StoreKit alone is not enough.";
    } else if (ent.isTrial) {
      accountHint.textContent = `30-day trial is active${ent.expirationDate ? ` until ${ent.expirationDate}` : ""}. Sign in with Stiki to unlock Pro surfaces.`;
    } else {
      const product = ent.productId === "com.mabel.app.pro.yearly" ? "Yearly" : "Monthly";
      accountHint.textContent = `${product} Pro is active${ent.willAutoRenew ? " and renews automatically" : ""}. Sign in with Stiki to unlock Pro surfaces.`;
    }
  }

  const status = document.getElementById("plan-status");
  if (status) {
    if (!entitled) status.textContent = "Personal (Free). Subscribe below — prices come from the App Store.";
    else if (!currentStiki.live) status.textContent = "App Store is entitled. Sign on with Stiki to unlock Pro. StoreKit alone is not enough.";
    else if (ent.isTrial) status.textContent = `Trial active${ent.expirationDate ? ` · ends ${ent.expirationDate}` : ""}.`;
    else status.textContent = `Pro · ${ent.productId ?? "subscription"}${ent.willAutoRenew ? " · auto-renew on" : ""}`;
  }

  if (unlocked) {
    loadProSurfaces().catch((e) => console.error("loadProSurfaces:", e));
  }
  applyPolishUi();
  refreshConnectors().catch((e) => console.error("refreshConnectors:", e));
  refreshStikiSession().catch((e) => console.error("refreshStikiSession:", e));
  invoke("refresh_status_item").catch((e) => console.error("refresh_status_item:", e));
}

async function refreshEntitlement() {
  await refreshStikiSession();
  try {
    const ent = await invoke<Entitlement>("storekit_entitlement");
    applyEntitlement(ent);
  } catch (e) {
    console.error("storekit_entitlement:", e);
    applyEntitlement({
      entitled: false,
      status: "none",
      isTrial: false,
      willAutoRenew: false,
    });
  }
}

function renderProducts(products: StoreProduct[], ent: Entitlement) {
  const root = document.getElementById("plan-products");
  if (!root) return;
  root.innerHTML = "";
  if (!products.length) {
    root.innerHTML = `<p class="row-hint">NO_PRODUCTS: StoreKit catalog is empty. Launch the 1.4.0 Mabel.app from the Mabel-StoreKit Xcode scheme (StoreKit Configuration = src-tauri/Mabel.storekit). Do not open /Applications/Mabel.app or Mabel 2.app. See docs/app-store-iap.md.</p>`;
    return;
  }
  for (const product of products) {
    const card = document.createElement("div");
    card.className = "plan-card";
    if (ent.entitled && ent.productId === product.id) card.classList.add("current");
    const intro = product.introOffer?.display
      ? `<div class="plan-card-intro">${product.introOffer.display}</div>`
      : "";
    const period = product.subscriptionPeriod ? ` / ${product.subscriptionPeriod}` : "";
    card.innerHTML = `
      <div class="plan-card-name">${product.displayName || product.id}</div>
      <div class="plan-card-price">${product.displayPrice || "—"}${period}</div>
      ${intro}
      <p class="row-hint">${product.description || ""}</p>
      <button type="button" class="btn-primary" data-product="${product.id}">${
        ent.entitled && ent.productId === product.id ? "Current plan" : "Subscribe"
      }</button>
    `;
    const btn = card.querySelector("button") as HTMLButtonElement;
    btn.disabled = !!(ent.entitled && ent.productId === product.id);
    btn.addEventListener("click", () => buyProduct(product.id, btn));
    root.appendChild(card);
  }
}

async function refreshStorefront() {
  const errorEl = document.getElementById("plan-error");
  if (errorEl) errorEl.textContent = "";
  await refreshEntitlement();
  try {
    const products = await invoke<StoreProduct[]>("storekit_products");
    renderProducts(products, currentEntitlement);
  } catch (e) {
    renderProducts([], currentEntitlement);
    if (errorEl) errorEl.textContent = String(e);
  }
}

const STOREKIT_UI_TIMEOUT_MS = 125_000;

function withTimeout<T>(promise: Promise<T>, ms: number, message: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => reject(new Error(message)), ms);
    promise.then(
      (value) => {
        window.clearTimeout(timer);
        resolve(value);
      },
      (err) => {
        window.clearTimeout(timer);
        reject(err);
      },
    );
  });
}

async function buyProduct(productId: string, btn: HTMLButtonElement) {
  const errorEl = document.getElementById("plan-error");
  const statusEl = document.getElementById("plan-status");
  const original = btn.textContent;
  btn.disabled = true;
  btn.textContent = "Waiting for App Store…";
  if (errorEl) errorEl.textContent = "";
  if (statusEl) statusEl.textContent = "Waiting for App Store…";
  try {
    const ent = await withTimeout(
      invoke<Entitlement>("storekit_purchase", { productId }),
      STOREKIT_UI_TIMEOUT_MS,
      "Purchase timed out. Restore Purchases and Manage Subscriptions still work. Mabel did not grant Pro.",
    );
    applyEntitlement(ent);
    await refreshStorefront();
  } catch (e) {
    if (errorEl) errorEl.textContent = String(e);
    if (statusEl) statusEl.textContent = "Purchase did not complete.";
  } finally {
    btn.disabled = false;
    btn.textContent = original;
  }
}

document.getElementById("plan-restore")?.addEventListener("click", async () => {
  const errorEl = document.getElementById("plan-error");
  const statusEl = document.getElementById("plan-status");
  const restoreBtn = document.getElementById("plan-restore") as HTMLButtonElement | null;
  if (errorEl) errorEl.textContent = "";
  if (statusEl) statusEl.textContent = "Waiting for App Store…";
  if (restoreBtn) restoreBtn.disabled = true;
  try {
    const ent = await withTimeout(
      invoke<Entitlement>("storekit_restore"),
      STOREKIT_UI_TIMEOUT_MS,
      "Restore timed out. Manage Subscriptions still works. Mabel did not grant Pro.",
    );
    applyEntitlement(ent);
    await refreshStorefront();
  } catch (e) {
    if (errorEl) errorEl.textContent = String(e);
    if (statusEl) statusEl.textContent = "Restore did not complete.";
  } finally {
    if (restoreBtn) restoreBtn.disabled = false;
  }
});

document.getElementById("plan-manage")?.addEventListener("click", async () => {
  const errorEl = document.getElementById("plan-error");
  try {
    await invoke("storekit_manage_subscriptions");
  } catch (e) {
    if (errorEl) errorEl.textContent = String(e);
  }
});

document.getElementById("plan-redeem")?.addEventListener("click", async () => {
  const errorEl = document.getElementById("plan-error");
  const statusEl = document.getElementById("plan-status");
  const redeemBtn = document.getElementById("plan-redeem") as HTMLButtonElement | null;
  if (errorEl) errorEl.textContent = "";
  if (redeemBtn) redeemBtn.disabled = true;
  try {
    const supported = await invoke<boolean>("storekit_offer_codes_supported");
    if (!supported) {
      if (errorEl) errorEl.textContent = "Offer codes need a newer macOS";
      if (statusEl) statusEl.textContent = "Offer codes need a newer macOS";
      return;
    }
    if (statusEl) statusEl.textContent = "Waiting for App Store…";
    const ent = await withTimeout(
      invoke<Entitlement>("storekit_redeem_offer_code"),
      STOREKIT_UI_TIMEOUT_MS,
      "Offer code redemption timed out. You are still on Free.",
    );
    await refreshStikiSession();
    applyEntitlement(ent);
    await refreshStorefront();
    if (ent.entitled && !proSurfacesUnlocked(ent) && errorEl) {
      errorEl.textContent =
        "Offer applied. Sign on with Stiki to unlock Pro. StoreKit alone is not enough.";
    }
  } catch (e) {
    if (errorEl) errorEl.textContent = String(e);
    if (statusEl) statusEl.textContent = "Offer code was not applied.";
  } finally {
    if (redeemBtn) redeemBtn.disabled = false;
  }
});

listen<Entitlement>("pro-entitlement-changed", (event) => {
  applyEntitlement(event.payload);
});
listen<string>("polish-changed", (event) => {
  if (currentSettings) currentSettings.polishMode = event.payload || "off";
  applyPolishUi();
});
listen("open-plans", () => {
  openPlans();
});
listen("open-dictionary", () => {
  openDictionary();
});
listen("open-snippets", () => {
  openSnippets();
});
listen("open-style", () => {
  openStyle();
});
listen("open-transforms", () => {
  openTransforms();
});
listen("open-scratchpad", () => {
  openScratchpad();
});
listen("open-insights", () => {
  openInsights();
});
listen("open-account", () => {
  openAccount();
});
listen<string>("open-settings-pane", (event) => {
  openSettingsPane(event.payload || "engine");
});

function row(html: string, onRemove: () => void) {
  const el = document.createElement("div");
  el.className = "pro-row";
  el.innerHTML = html + `<button type="button" class="btn-secondary">Remove</button>`;
  el.querySelector("button")?.addEventListener("click", onRemove);
  return el;
}

async function loadDictionarySurface() {
  if (!dictionarySurfaceReady()) return;
  const terms = await invoke<string[]>("dictionary_get");
  applyDictionary(terms);
}

let editingSnippetId: string | null = null;

function showSnippetError(message: string) {
  const errorEl = document.getElementById("snippet-error");
  if (!errorEl) return;
  errorEl.textContent = message;
  errorEl.hidden = !message;
}

function cancelEditSnippet() {
  editingSnippetId = null;
  $<HTMLInputElement>("snippet-trigger").value = "";
  $<HTMLInputElement>("snippet-expansion").value = "";
  $("snippet-add-btn").textContent = "Add";
  $("snippet-cancel-btn").classList.add("hidden");
  showSnippetError("");
}

function beginEditSnippet(snippet: Snippet) {
  if (!snippetsSurfaceReady()) {
    if (!currentEntitlement.entitled) openPlans();
    return;
  }
  editingSnippetId = snippet.id;
  $<HTMLInputElement>("snippet-trigger").value = snippet.trigger;
  $<HTMLInputElement>("snippet-expansion").value = snippet.expansion;
  $("snippet-add-btn").textContent = "Save";
  $("snippet-cancel-btn").classList.remove("hidden");
  showSnippetError("");
  $<HTMLInputElement>("snippet-trigger").focus();
}

async function loadSnippetsSurface() {
  if (!snippetsSurfaceReady()) return;
  const snippets = await invoke<Snippet[]>("snippets_get");
  const snippetList = document.getElementById("snippet-list");
  const snippetEmpty = document.getElementById("snippet-empty");
  if (snippetList) {
    snippetList.innerHTML = "";
    snippets.forEach((s) => {
      const el = document.createElement("div");
      el.className = "pro-row";
      const text = document.createElement("div");
      const trigger = document.createElement("div");
      trigger.textContent = s.trigger;
      const meta = document.createElement("div");
      meta.className = "pro-row-meta";
      meta.textContent = s.expansion;
      text.appendChild(trigger);
      text.appendChild(meta);
      const edit = document.createElement("button");
      edit.type = "button";
      edit.className = "btn-secondary";
      edit.textContent = "Edit";
      edit.setAttribute("aria-label", `Edit ${s.trigger}`);
      edit.addEventListener("click", () => beginEditSnippet(s));
      const remove = document.createElement("button");
      remove.type = "button";
      remove.className = "btn-secondary";
      remove.textContent = "Remove";
      remove.setAttribute("aria-label", `Remove ${s.trigger}`);
      remove.addEventListener("click", async () => {
        if (!snippetsSurfaceReady()) {
          if (!currentEntitlement.entitled) openPlans();
          return;
        }
        showSnippetError("");
        try {
          await invoke("snippets_remove", { snippetId: s.id });
          if (editingSnippetId === s.id) cancelEditSnippet();
          await loadSnippetsSurface();
        } catch (e) {
          showSnippetError(String(e));
          console.error("snippets_remove:", e);
        }
      });
      const actions = document.createElement("div");
      actions.className = "row-actions";
      actions.appendChild(edit);
      actions.appendChild(remove);
      el.appendChild(text);
      el.appendChild(actions);
      snippetList.appendChild(el);
    });
  }
  snippetEmpty?.classList.toggle("hidden", snippets.length > 0);
}

function showStyleError(message: string) {
  const errorEl = document.getElementById("style-error");
  if (!errorEl) return;
  errorEl.textContent = message;
  errorEl.hidden = !message;
}

function renderStyleMode(mode: string) {
  const picked = mode === "formal" || mode === "casual" || mode === "very-casual" ? mode : "";
  document.querySelectorAll<HTMLButtonElement>("[data-style-mode]").forEach((btn) => {
    btn.classList.toggle("active", !!picked && btn.dataset.styleMode === picked);
  });
}

async function loadStyleSurface() {
  if (!styleSurfaceReady()) return;
  const style = await invoke<StylePrefs>("style_get");
  renderStyleMode(style.mode);
  showStyleError("");
}

async function pickStyleMode(mode: string) {
  if (!styleSurfaceReady()) {
    if (!currentEntitlement.entitled) openPlans();
    return;
  }
  showStyleError("");
  try {
    const next = await invoke<StylePrefs>("style_set", { mode });
    renderStyleMode(next.mode);
  } catch (e) {
    showStyleError(String(e));
    console.error("style_set:", e);
  }
}

async function clearStyleMode() {
  if (!styleSurfaceReady()) {
    if (!currentEntitlement.entitled) openPlans();
    return;
  }
  showStyleError("");
  try {
    const next = await invoke<StylePrefs>("style_clear");
    renderStyleMode(next.mode);
  } catch (e) {
    showStyleError(String(e));
    console.error("style_clear:", e);
  }
}

function showTransformsError(message: string) {
  const errorEl = document.getElementById("xf-error");
  if (!errorEl) return;
  errorEl.textContent = message;
  errorEl.hidden = !message;
}

function renderTransforms(prefs: TransformPrefs) {
  const source = $<HTMLTextAreaElement>("xf-source");
  const result = $<HTMLTextAreaElement>("xf-result");
  if (prefs.lastSource) source.value = prefs.lastSource;
  result.value = prefs.lastResult || "";
  const picked = ["email", "bullets", "shorter", "clearer"].includes(prefs.lastAction)
    ? prefs.lastAction
    : "";
  document.querySelectorAll<HTMLButtonElement>("[data-xf-action]").forEach((btn) => {
    btn.classList.toggle("active", !!picked && btn.dataset.xfAction === picked);
  });
}

async function loadTransformsSurface() {
  if (!transformsSurfaceReady()) return;
  const xf = await invoke<TransformPrefs>("transforms_get");
  renderTransforms(xf);
  showTransformsError("");
}

async function invokeTransform(action: string) {
  if (!transformsSurfaceReady()) {
    if (!currentEntitlement.entitled) openPlans();
    return;
  }
  const source = $<HTMLTextAreaElement>("xf-source").value;
  showTransformsError("");
  try {
    const next = await invoke<TransformPrefs>("transforms_apply", { action, source });
    renderTransforms(next);
  } catch (e) {
    showTransformsError(String(e));
    console.error("transforms_apply:", e);
  }
}

async function clearTransforms() {
  if (!transformsSurfaceReady()) {
    if (!currentEntitlement.entitled) openPlans();
    return;
  }
  showTransformsError("");
  try {
    const next = await invoke<TransformPrefs>("transforms_clear");
    $<HTMLTextAreaElement>("xf-source").value = "";
    renderTransforms(next);
  } catch (e) {
    showTransformsError(String(e));
    console.error("transforms_clear:", e);
  }
}

function showScratchpadError(message: string) {
  const errorEl = document.getElementById("scratchpad-error");
  if (!errorEl) return;
  errorEl.textContent = message;
  errorEl.hidden = !message;
}

async function loadScratchpadSurface() {
  if (!scratchpadSurfaceReady()) return;
  const text = await invoke<string>("scratchpad_get");
  $<HTMLTextAreaElement>("scratchpad-text").value = text;
  showScratchpadError("");
}

async function loadInsightsSurface() {
  if (!insightsSurfaceReady()) {
    resetStatsDisplay();
    return;
  }
  try {
    const s = await invoke<StatsSummary>("insights_get");
    renderStatsSummary(s);
    if (streakGrid) {
      const max = Math.max(1, ...s.last30);
      streakGrid.innerHTML = "";
      s.last30.forEach((count) => {
        const cell = document.createElement("span");
        cell.className = "streak-cell";
        if (count === 0) cell.classList.add("l0");
        else {
          const pct = count / max;
          if (pct > 0.75) cell.classList.add("l4");
          else if (pct > 0.5) cell.classList.add("l3");
          else if (pct > 0.25) cell.classList.add("l2");
          else cell.classList.add("l1");
        }
        cell.title = `${count} dictation${count === 1 ? "" : "s"}`;
        streakGrid.appendChild(cell);
      });
    }
  } catch (e) {
    console.error("insights_get failed:", e);
    resetStatsDisplay();
  }
}

async function saveScratchpad() {
  if (!scratchpadSurfaceReady()) {
    if (!currentEntitlement.entitled) openPlans();
    return;
  }
  showScratchpadError("");
  try {
    const next = await invoke<string>("scratchpad_save", {
      text: $<HTMLTextAreaElement>("scratchpad-text").value,
    });
    $<HTMLTextAreaElement>("scratchpad-text").value = next;
  } catch (e) {
    showScratchpadError(String(e));
    console.error("scratchpad_save:", e);
  }
}

async function clearScratchpad() {
  if (!scratchpadSurfaceReady()) {
    if (!currentEntitlement.entitled) openPlans();
    return;
  }
  showScratchpadError("");
  try {
    const next = await invoke<string>("scratchpad_clear");
    $<HTMLTextAreaElement>("scratchpad-text").value = next;
  } catch (e) {
    showScratchpadError(String(e));
    console.error("scratchpad_clear:", e);
  }
}

async function loadProSurfaces() {
  if (!proSurfacesUnlocked()) return;
  await loadDictionarySurface();
  await loadSnippetsSurface();
  await loadStyleSurface();
  await loadTransformsSurface();
  await loadScratchpadSurface();
  await loadInsightsSurface();

  try {
    const team = await invoke<TeamState>("teams_get");
    renderTeams(team);
  } catch (e) {
    console.error("teams_get:", e);
  }
  await refreshConnectors();
}

async function refreshConnectors() {
  const errorEl = document.getElementById("connectors-error");
  try {
    const status = await invoke<ConnectorsStatus>("connectors_status");
    renderConnectors(status);
    applyStikiFromConnectors(status);
    if (errorEl) {
      errorEl.textContent = "";
      errorEl.classList.add("hidden");
    }
  } catch (e) {
    console.error("connectors_status:", e);
    if (errorEl) {
      errorEl.textContent = String(e);
      errorEl.classList.remove("hidden");
    }
  }
}

function renderConnectors(status: ConnectorsStatus) {
  for (const item of status.catalog) {
    const card = document.getElementById(`connector-${item.id}`);
    const hint = document.getElementById(`connector-${item.id}-hint`);
    if (hint) hint.textContent = item.hint;
    if (card) {
      card.classList.toggle("connected", item.connected);
      card.classList.toggle("live", item.live);
    }
  }
}

async function setConnector(id: string, connect: boolean) {
  const errorEl = document.getElementById("connectors-error");
  if (!currentEntitlement.entitled && connect) {
    openPlans();
    return;
  }
  try {
    const status = await invoke<ConnectorsStatus>(
      connect ? "connectors_connect" : "connectors_disconnect",
      { id },
    );
    renderConnectors(status);
    applyStikiFromConnectors(status);
    if (errorEl) {
      errorEl.textContent = "";
      errorEl.classList.add("hidden");
    }
  } catch (e) {
    console.error(connect ? "connectors_connect:" : "connectors_disconnect:", e);
    if (errorEl) {
      errorEl.textContent = String(e);
      errorEl.classList.remove("hidden");
    }
    if (connect && !currentEntitlement.entitled) openPlans();
    else if (connect && String(e).includes("Sign in with Stiki")) openAccount();
  }
}

function applyStikiFromConnectors(status: ConnectorsStatus) {
  currentStiki = {
    live: !!status.signedIn,
    signedIn: !!status.signedIn,
    hint: status.stikiHint,
  };
  const hint = document.getElementById("stiki-session-hint");
  if (hint) hint.textContent = status.stikiHint;
  const signIn = document.getElementById("stiki-signin") as HTMLButtonElement | null;
  const signOut = document.getElementById("stiki-signout") as HTMLButtonElement | null;
  if (signIn) signIn.disabled = status.signedIn;
  if (signOut) signOut.disabled = !status.signedIn;
  applyProLocks();
  applyPolishUi();
  if (proUnlocked()) {
    loadProSurfaces().catch((e) => console.error("loadProSurfaces:", e));
  }
}

document.getElementById("stiki-signin")?.addEventListener("click", async () => {
  const hint = document.getElementById("stiki-session-hint");
  const signIn = document.getElementById("stiki-signin") as HTMLButtonElement | null;
  if (hint) {
    hint.textContent = "Opening Stiki (Apple, Google, or Microsoft at auth.chibitek.com)…";
  }
  if (signIn) signIn.disabled = true;
  try {
    const session = await invoke<StikiSession>("stiki_sign_in");
    if (hint) hint.textContent = session.hint;
    await refreshConnectors();
    await refreshStikiSession();
  } catch (e) {
    if (hint) hint.textContent = String(e);
    if (signIn) signIn.disabled = false;
    await refreshConnectors();
  }
});

document.getElementById("stiki-signout")?.addEventListener("click", async () => {
  try {
    const status = await invoke<ConnectorsStatus>("stiki_sign_out");
    renderConnectors(status);
    applyStikiFromConnectors(status);
  } catch (e) {
    console.error("stiki_sign_out:", e);
    const hint = document.getElementById("stiki-session-hint");
    if (hint) hint.textContent = String(e);
  }
});

document.querySelectorAll<HTMLButtonElement>(".connector-connect").forEach((btn) => {
  btn.addEventListener("click", () => {
    void setConnector(btn.dataset.id || "", true);
  });
});
document.querySelectorAll<HTMLButtonElement>(".connector-disconnect").forEach((btn) => {
  btn.addEventListener("click", () => {
    void setConnector(btn.dataset.id || "", false);
  });
});

function renderTeams(team: TeamState) {
  const org = $<HTMLInputElement>("team-org");
  org.value = team.orgName;
  const seats = document.getElementById("seat-list");
  const invites = document.getElementById("invite-list");
  if (seats) {
    seats.innerHTML = "";
    team.seats.forEach((s) => {
      seats.appendChild(
        row(
          `<div><div>${s.displayName} · ${s.role}</div><div class="pro-row-meta">${s.email}</div></div>`,
          async () => {
            try {
              renderTeams(await invoke<TeamState>("teams_remove_seat", { seatId: s.id }));
            } catch (e) {
              console.error("teams_remove_seat:", e);
              if (teamAclFailed(e)) openAccount();
            }
          }
        )
      );
    });
  }
  if (invites) {
    invites.innerHTML = "";
    team.invites.forEach((i) => {
      invites.appendChild(
        row(
          `<div><div>${i.email} · ${i.status}</div><div class="pro-row-meta">${i.token}</div></div>`,
          async () => {
            try {
              renderTeams(await invoke<TeamState>("teams_revoke_invite", { inviteId: i.id }));
            } catch (e) {
              console.error("teams_revoke_invite:", e);
              if (teamAclFailed(e)) openAccount();
            }
          }
        )
      );
    });
  }
}

async function addOrSaveSnippet() {
  if (!snippetsSurfaceReady()) {
    if (!currentEntitlement.entitled) openPlans();
    return;
  }
  showSnippetError("");
  const trigger = $<HTMLInputElement>("snippet-trigger").value;
  const expansion = $<HTMLInputElement>("snippet-expansion").value;
  try {
    if (editingSnippetId) {
      await invoke("snippets_update", {
        snippetId: editingSnippetId,
        trigger,
        expansion,
      });
      cancelEditSnippet();
    } else {
      await invoke("snippets_add", { trigger, expansion });
      $<HTMLInputElement>("snippet-trigger").value = "";
      $<HTMLInputElement>("snippet-expansion").value = "";
    }
    await loadSnippetsSurface();
  } catch (e) {
    showSnippetError(String(e));
    console.error("snippet mutate:", e);
  }
}

$("snippet-add-btn").addEventListener("click", () => {
  void addOrSaveSnippet();
});
$("snippet-cancel-btn").addEventListener("click", cancelEditSnippet);
$("snippet-trigger").addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    void addOrSaveSnippet();
  }
  if (e.key === "Escape") {
    e.preventDefault();
    cancelEditSnippet();
  }
});
$("snippet-expansion").addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    void addOrSaveSnippet();
  }
  if (e.key === "Escape") {
    e.preventDefault();
    cancelEditSnippet();
  }
});

document.querySelectorAll<HTMLButtonElement>("[data-style-mode]").forEach((btn) => {
  btn.addEventListener("click", () => {
    void pickStyleMode(btn.dataset.styleMode || "");
  });
});
$("style-clear-btn").addEventListener("click", () => {
  void clearStyleMode();
});

document.querySelectorAll<HTMLButtonElement>("[data-xf-action]").forEach((btn) => {
  btn.addEventListener("click", () => {
    void invokeTransform(btn.dataset.xfAction || "");
  });
});
$("xf-clear-btn").addEventListener("click", () => {
  void clearTransforms();
});

$("scratchpad-clear-btn").addEventListener("click", () => {
  void clearScratchpad();
});

let scratchpadTimer: number | undefined;
$("scratchpad-text").addEventListener("input", () => {
  window.clearTimeout(scratchpadTimer);
  scratchpadTimer = window.setTimeout(() => {
    void saveScratchpad();
  }, 400);
});

function teamAclFailed(err: unknown): boolean {
  return String(err).includes("Sign in with Stiki");
}

$("team-org-save").addEventListener("click", async () => {
  try {
    renderTeams(await invoke<TeamState>("teams_set_org", { orgName: $<HTMLInputElement>("team-org").value }));
  } catch (e) {
    console.error("teams_set_org:", e);
    if (teamAclFailed(e)) openAccount();
  }
});
$("seat-add").addEventListener("click", async () => {
  try {
    const team = await invoke<TeamState>("teams_add_seat", {
      displayName: $<HTMLInputElement>("seat-name").value,
      email: $<HTMLInputElement>("seat-email").value,
      role: $<HTMLSelectElement>("seat-role").value,
    });
    $<HTMLInputElement>("seat-name").value = "";
    $<HTMLInputElement>("seat-email").value = "";
    renderTeams(team);
  } catch (e) {
    console.error("teams_add_seat:", e);
    if (teamAclFailed(e)) openAccount();
  }
});
$("invite-add").addEventListener("click", async () => {
  try {
    const team = await invoke<TeamState>("teams_create_invite", {
      email: $<HTMLInputElement>("invite-email").value,
    });
    $<HTMLInputElement>("invite-email").value = "";
    renderTeams(team);
  } catch (e) {
    console.error("teams_create_invite:", e);
    if (teamAclFailed(e)) openAccount();
  }
});

function showStorageError(message: string) {
  console.error("storage-status:", message);
  alert(`Could not load previous Mabel data: ${message}`);
}

async function checkStorageStatus() {
  try {
    const status = await invoke<{
      migration: { status: string; message?: string | null };
      settingsError?: string | null;
      statsError?: string | null;
      historyError?: string | null;
    }>("get_storage_status");
    const msg =
      status.migration.status === "failed"
        ? status.migration.message
        : status.settingsError || status.statsError || status.historyError;
    if (msg) showStorageError(msg);
  } catch (e) {
    console.error("get_storage_status:", e);
  }
}

listen<string>("storage-status-error", (event) => {
  if (event.payload) showStorageError(event.payload);
});

loadSettings();
loadVersion();
loadStats();
checkStorageStatus();
refreshEntitlement();
maybeRunFirstTimeSetup();
setTimeout(() => {
  checkForUpdates(true);
}, 2500);

// Check Accessibility status on launch, but do not auto-prompt. Unsigned test
// builds can trigger repeated permission dialogs because binary signatures
// change across builds.
async function ensureAccessibility() {
  try {
    const trusted = await invoke<boolean>("check_accessibility");
    if (!trusted) {
      console.warn("Accessibility not granted yet. Prompt is deferred until needed.");
    }
  } catch (e) {
    console.error("accessibility check failed:", e);
  }
}
ensureAccessibility();

async function showWhatsNew(markSeen: boolean) {
  const entry = await invoke<{ version: string; body: string } | null>("get_whats_new");
  if (!entry) {
    updateStatus.textContent = "No release notes are bundled for this version.";
    return false;
  }
  const modal = document.getElementById("whatsnew-modal")!;
  const verEl = document.getElementById("whatsnew-version")!;
  const bodyEl = document.getElementById("whatsnew-body")!;
  const done = document.getElementById("whatsnew-done")!;
  verEl.textContent = "v" + entry.version;
  bodyEl.innerHTML = renderChangelog(entry.body);
  modal.classList.remove("hidden");

  const dismiss = async () => {
    modal.classList.add("hidden");
    if (markSeen) await invoke("mark_version_seen");
  };
  done.replaceWith(done.cloneNode(true));
  document.getElementById("whatsnew-done")!.addEventListener("click", dismiss, { once: true });
  return true;
}

showWhatsNewBtn.addEventListener("click", () => {
  showWhatsNew(false).catch((e) => {
    console.error("show whats-new failed:", e);
    updateStatus.textContent = `What's New failed: ${String(e)}`;
  });
});

// What's New popup. On first launch after an update (or first launch ever
// after the user has finished the initial Whisper download), if the bundled
// changelog has an entry for the running version, pop a modal showing what
// changed. Marks the version as seen on dismiss so we don't re-show it.
async function maybeShowWhatsNew() {
  try {
    const settings = await invoke<Settings>("get_settings");
    const ver = await invoke<VersionInfo>("get_version");
    if (settings.lastSeenVersion === ver.version) return;
    // Don't pop on the very first install before they've used the app at all.
    // The first-run download modal handles that surface; popping What's New on
    // top of it would be noisy. We detect "first launch ever" as no
    // lastSeenVersion AND no Whisper model on disk.
    if (!settings.lastSeenVersion) {
      let hasModel = false;
      for (const v of WHISPER_VARIANTS) {
        if (await invoke<boolean>("check_model_downloaded", v)) {
          hasModel = true;
          break;
        }
      }
      if (!hasModel) return;
    }
    const shown = await showWhatsNew(true);
    if (!shown) {
      // No changelog entry for this version — still mark it seen so we don't
      // try again next launch.
      await invoke("mark_version_seen");
    }
  } catch (e) {
    console.error("whats-new check failed:", e);
  }
}

// Tiny changelog renderer: handles ### subheaders and bullet lists, escapes
// everything else. Keeps the popup safe against accidental HTML in the
// changelog source.
function renderChangelog(md: string): string {
  const escape = (s: string) =>
    s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  const lines = md.split("\n");
  const html: string[] = [];
  let inList = false;
  for (const raw of lines) {
    const line = raw.trim();
    if (!line) {
      if (inList) { html.push("</ul>"); inList = false; }
      continue;
    }
    if (line.startsWith("### ")) {
      if (inList) { html.push("</ul>"); inList = false; }
      html.push(`<h3 class="whatsnew-h">${escape(line.slice(4))}</h3>`);
    } else if (line.startsWith("- ")) {
      if (!inList) { html.push("<ul class=\"whatsnew-list\">"); inList = true; }
      html.push(`<li>${escape(line.slice(2))}</li>`);
    } else {
      if (inList) { html.push("</ul>"); inList = false; }
      html.push(`<p>${escape(line)}</p>`);
    }
  }
  if (inList) html.push("</ul>");
  return html.join("");
}

maybeShowWhatsNew();

// Easter egg: seven clicks anywhere on the hero (portrait + title block) within
// the first ten seconds of opening the app toggle Mochi mode (hero portrait,
// brand name, and view title swap). The flag is persisted in localStorage so
// the chosen skin survives across launches; the same gesture toggles it back.
(function mochiModeEasterEgg() {
  const STORAGE_KEY = "mabel.mochiMode";
  const UNLOCKED_KEY = "mabel.mochiUnlocked";
  const portrait = document.getElementById("hero-portrait") as HTMLImageElement | null;
  const title = document.getElementById("meet-title");
  const brand = document.getElementById("brand-name");
  const skinRow = document.getElementById("companion-skin-row");
  const skinSelect = document.getElementById("companion-skin-select") as HTMLSelectElement | null;

  const setMode = (on: boolean) => {
    if (on) {
      localStorage.setItem(STORAGE_KEY, "1");
      localStorage.setItem(UNLOCKED_KEY, "1");
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
    apply(on);
  };

  const apply = (on: boolean) => {
    if (portrait) {
      portrait.src = on ? "/mochi.png" : "/mabel.png";
      portrait.alt = on ? "Mochi" : "Mabel";
      portrait.classList.toggle("mochi", on);
    }
    if (title) title.textContent = on ? "Meet Mochi" : "Meet Mabel";
    if (brand) brand.textContent = on ? "Mochi" : "Mabel";
    // Once unlocked, expose the Settings toggle so the user doesn't have to
    // re-do the easter-egg gesture to switch back. Currently being in mochi
    // mode counts as unlocked too — covers users who activated the egg on a
    // build that didn't yet write the unlocked flag.
    if (on) localStorage.setItem(UNLOCKED_KEY, "1");
    if (skinRow && localStorage.getItem(UNLOCKED_KEY) === "1") {
      skinRow.classList.remove("hidden");
    }
    if (skinSelect) skinSelect.value = on ? "mochi" : "mabel";
    // Tell the companion window to swap its sprite skin. Re-emit a few times
    // because the companion's listener may not have attached yet on first
    // boot — the emits are idempotent.
    const payload = { skin: on ? "mochi" : "mabel" };
    emit("mabel-companion-skin", payload).catch(() => {});
    setTimeout(() => emit("mabel-companion-skin", payload).catch(() => {}), 500);
    setTimeout(() => emit("mabel-companion-skin", payload).catch(() => {}), 2000);
  };

  apply(localStorage.getItem(STORAGE_KEY) === "1");

  if (skinSelect) {
    skinSelect.addEventListener("change", () => {
      setMode(skinSelect.value === "mochi");
    });
  }

  const hero = document.querySelector(".hero") as HTMLElement | null;
  if (!hero) return;
  const start = Date.now();
  let count = 0;
  const onClick = () => {
    if (Date.now() - start > 10000) {
      hero.removeEventListener("click", onClick);
      return;
    }
    count++;
    console.log("[mochi-egg] click", count);
    if (count >= 7) {
      hero.removeEventListener("click", onClick);
      setMode(localStorage.getItem(STORAGE_KEY) !== "1");
    }
  };
  hero.addEventListener("click", onClick);
  setTimeout(() => hero.removeEventListener("click", onClick), 10100);
})();
