import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface AppInfo {
  name: string;
  version: string;
}

interface ModelStatus {
  id: string;
  name: string;
  description: string;
  english_only: boolean;
  size_bytes: number;
  downloaded: boolean;
  downloading: boolean;
  selected: boolean;
  loaded: boolean;
  recommended: boolean;
}

interface ModelEvent {
  id: string;
  error: string | null;
}

interface Progress {
  id: string;
  done: number;
  total: number;
}

const list = document.getElementById("models")!;
const engineStatus = document.getElementById("engine-status")!;
const errorBox = document.getElementById("models-error")!;
const progress = new Map<string, number>();
const loading = new Set<string>();

function formatSize(bytes: number): string {
  return bytes >= 1e9 ? `${(bytes / 1e9).toFixed(1)} GB` : `${Math.round(bytes / 1e6)} MB`;
}

function showError(message: string | null): void {
  errorBox.hidden = !message;
  errorBox.textContent = message ?? "";
}

async function showAppInfo(): Promise<void> {
  const el = document.getElementById("version");
  if (!el) return;
  try {
    const info = await invoke<AppInfo>("app_info");
    el.textContent = `${info.name} ${info.version}`;
  } catch (err) {
    el.textContent = `unavailable (${String(err)})`;
  }
}

function modelRow(m: ModelStatus): HTMLLIElement {
  const li = document.createElement("li");
  li.className = "model";

  const info = document.createElement("div");
  const title = document.createElement("div");
  title.className = "model-name";
  title.textContent = m.name;
  if (m.recommended) {
    const tag = document.createElement("span");
    tag.className = "tag";
    tag.textContent = "Recommended for this PC";
    title.append(" ", tag);
  }
  const desc = document.createElement("div");
  desc.className = "muted small";
  desc.textContent = `${m.description} ${formatSize(m.size_bytes)}.`;
  info.append(title, desc);

  const action = document.createElement("div");
  action.className = "model-action";
  const pct = progress.get(m.id);
  if (m.downloading || pct !== undefined) {
    const bar = document.createElement("progress");
    bar.max = 100;
    bar.value = pct ?? 0;
    bar.dataset.id = m.id;
    action.append(bar);
  } else if (!m.downloaded) {
    const btn = document.createElement("button");
    btn.textContent = "Download";
    btn.onclick = () => void download(m.id);
    action.append(btn);
  } else if (m.selected) {
    const state = document.createElement("span");
    state.className = "muted small";
    state.textContent = m.loaded ? "In use" : loading.has(m.id) ? "Loading…" : "Selected";
    action.append(state);
  } else {
    const btn = document.createElement("button");
    btn.textContent = "Use";
    btn.onclick = () => void select(m.id);
    action.append(btn);
  }

  li.append(info, action);
  return li;
}

async function refresh(): Promise<void> {
  try {
    const models = await invoke<ModelStatus[]>("list_models");
    list.replaceChildren(...models.map(modelRow));
    const active = models.find((m) => m.selected);
    engineStatus.textContent = !active
      ? "None yet. Download one below."
      : active.loaded
        ? `${active.name}, ready`
        : `${active.name}, loading…`;
  } catch (err) {
    showError(String(err));
  }
}

async function download(id: string): Promise<void> {
  showError(null);
  progress.set(id, 0);
  await refresh();
  await invoke("download_model", { id }).catch((err) => {
    progress.delete(id);
    showError(String(err));
  });
}

async function select(id: string): Promise<void> {
  showError(null);
  loading.add(id);
  await invoke("select_model", { id }).catch((err) => showError(String(err)));
  await refresh();
}

void listen<Progress>("model-progress", ({ payload }) => {
  const pct = Math.floor((payload.done / payload.total) * 100);
  progress.set(payload.id, pct);
  // Update the bar in place; re-render only if it isn't on screen yet.
  const bar = list.querySelector<HTMLProgressElement>(`progress[data-id="${payload.id}"]`);
  if (bar) bar.value = pct;
  else void refresh();
});

void listen<ModelEvent>("model-downloaded", ({ payload }) => {
  progress.delete(payload.id);
  showError(payload.error);
  void refresh();
});

void listen<ModelEvent>("model-loaded", ({ payload }) => {
  loading.delete(payload.id);
  if (payload.error) showError(payload.error);
  void refresh();
});

void showAppInfo();
void refresh();
