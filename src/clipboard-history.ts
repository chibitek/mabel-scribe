import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface HistoryItemView {
  id: string;
  preview: string;
  textLen: number;
  createdAt: number;
}

interface HistoryList {
  enabled: boolean;
  entitled: boolean;
  cap: number;
  count: number;
  items: HistoryItemView[];
  localSlotsOnly: boolean;
}

const enabledToggle = document.getElementById("enabled-toggle") as HTMLButtonElement;
const capHint = document.getElementById("cap-hint") as HTMLElement;
const offState = document.getElementById("off-state") as HTMLElement;
const onState = document.getElementById("on-state") as HTMLElement;
const listEl = document.getElementById("list") as HTMLElement;
const emptyState = document.getElementById("empty-state") as HTMLElement;
const clearBtn = document.getElementById("clear-btn") as HTMLButtonElement;

function setSwitch(on: boolean) {
  enabledToggle.setAttribute("aria-checked", String(on));
}

function formatWhen(epochSecs: number): string {
  if (!epochSecs) return "";
  const d = new Date(epochSecs * 1000);
  return d.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

function render(list: HistoryList) {
  setSwitch(list.enabled);
  if (list.entitled) {
    capHint.textContent = `Pro · up to ${list.cap.toLocaleString()} local slots on this Mac.`;
  } else {
    capHint.textContent = `Free keeps the last ${list.cap} local slots. Pro raises the local cap.`;
  }
  offState.classList.toggle("hidden", list.enabled);
  onState.classList.toggle("hidden", !list.enabled);
  clearBtn.disabled = !list.enabled || list.count === 0;

  listEl.innerHTML = "";
  if (!list.enabled) return;
  emptyState.classList.toggle("hidden", list.items.length > 0);
  for (const item of list.items) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "item";
    btn.innerHTML = `<div class="item-preview"></div><div class="item-meta"></div>`;
    (btn.querySelector(".item-preview") as HTMLElement).textContent = item.preview;
    (btn.querySelector(".item-meta") as HTMLElement).textContent =
      `${formatWhen(item.createdAt)} · click to paste`;
    btn.addEventListener("click", () => {
      invoke("clipboard_history_paste", { id: item.id }).catch((e) => {
        console.error("clipboard_history_paste:", e);
      });
    });
    listEl.appendChild(btn);
  }
}

async function refresh() {
  const list = await invoke<HistoryList>("clipboard_history_list");
  render(list);
}

enabledToggle.addEventListener("click", async () => {
  const next = enabledToggle.getAttribute("aria-checked") !== "true";
  setSwitch(next);
  try {
    const list = await invoke<HistoryList>("clipboard_history_set_enabled", { enabled: next });
    render(list);
  } catch (e) {
    console.error("clipboard_history_set_enabled:", e);
    await refresh();
  }
});

clearBtn.addEventListener("click", async () => {
  try {
    const list = await invoke<HistoryList>("clipboard_history_clear");
    render(list);
  } catch (e) {
    console.error("clipboard_history_clear:", e);
  }
});

refresh().catch((e) => console.error("clipboard_history_list:", e));
listen("clipboard-history-updated", () => {
  refresh().catch((e) => console.error("refresh:", e));
}).catch((e) => console.error("listen:", e));
