// Models page: download, pick and track speech models.
import { listen } from "@tauri-apps/api/event";

import { api, el, showError, type ModelStatus } from "./api";

interface ModelEvent {
  id: string;
  error: string | null;
}

interface Progress {
  id: string;
  done: number;
  total: number;
}

const list = () => document.getElementById("models")!;
const progress = new Map<string, number>();
const loading = new Set<string>();
let onChange: (models: ModelStatus[]) => void = () => {};

function formatSize(bytes: number): string {
  return bytes >= 1e9 ? `${(bytes / 1e9).toFixed(1)} GB` : `${Math.round(bytes / 1e6)} MB`;
}

function modelRow(m: ModelStatus): HTMLLIElement {
  const title = el("div", { className: "model-name" }, m.name);
  if (m.recommended) {
    title.append(" ", el("span", { className: "tag" }, "Recommended for this PC"));
  }
  const info = el(
    "div",
    {},
    title,
    el("div", { className: "muted small" }, `${m.description} ${formatSize(m.size_bytes)}.`),
  );

  const action = el("div", { className: "model-action" });
  const pct = progress.get(m.id);
  if (m.downloading || pct !== undefined) {
    const bar = el("progress", { max: 100, value: pct ?? 0 });
    bar.dataset.id = m.id;
    action.append(bar);
  } else if (!m.downloaded) {
    action.append(el("button", { onclick: () => void download(m.id) }, "Download"));
  } else if (m.selected) {
    const state = m.loaded ? "In use" : loading.has(m.id) ? "Loading…" : "Selected";
    action.append(el("span", { className: "muted small" }, state));
  } else {
    action.append(el("button", { onclick: () => void select(m.id) }, "Use"));
  }
  return el("li", { className: "model" }, info, action);
}

export async function refreshModels(): Promise<ModelStatus[]> {
  try {
    const models = await api.listModels();
    list().replaceChildren(...models.map(modelRow));
    onChange(models);
    return models;
  } catch (err) {
    showError(String(err));
    return [];
  }
}

async function download(id: string): Promise<void> {
  showError(null);
  progress.set(id, 0);
  await refreshModels();
  await api.downloadModel(id).catch((err) => {
    progress.delete(id);
    showError(String(err));
  });
}

async function select(id: string): Promise<void> {
  showError(null);
  loading.add(id);
  await api.selectModel(id).catch((err) => showError(String(err)));
  await refreshModels();
}

/** Wires up model events. `changed` runs after every refresh (for status lines). */
export function initModels(changed: (models: ModelStatus[]) => void): void {
  onChange = changed;

  void listen<Progress>("model-progress", ({ payload }) => {
    const pct = Math.floor((payload.done / payload.total) * 100);
    progress.set(payload.id, pct);
    // Update the bar in place; re-render only if it isn't on screen yet.
    const bar = list().querySelector<HTMLProgressElement>(`progress[data-id="${payload.id}"]`);
    if (bar) bar.value = pct;
    else void refreshModels();
  });

  void listen<ModelEvent>("model-downloaded", ({ payload }) => {
    progress.delete(payload.id);
    showError(payload.error);
    void refreshModels();
  });

  void listen<ModelEvent>("model-loaded", ({ payload }) => {
    loading.delete(payload.id);
    if (payload.error) showError(payload.error);
    void refreshModels();
  });

  void refreshModels();
}
