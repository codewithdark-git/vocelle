// History page: recent dictations with copy and delete.
import { api, el, showError, type HistoryEntry } from "./api";

const list = () => document.getElementById("history")!;

function when(at: number): string {
  const date = new Date(at);
  const today = new Date().toDateString() === date.toDateString();
  return today
    ? date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })
    : date.toLocaleString([], { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" });
}

function row(entry: HistoryEntry): HTMLLIElement {
  const copy = el("button", { type: "button" }, "Copy");
  copy.onclick = async () => {
    await api.copyText(entry.text).catch((err) => showError(String(err)));
    copy.textContent = "Copied";
    setTimeout(() => (copy.textContent = "Copy"), 1200);
  };
  const remove = el("button", { type: "button", className: "subtle", title: "Delete" }, "Delete");
  remove.onclick = async () => {
    await api.deleteHistoryEntry(entry.at);
    await refreshHistory();
  };
  return el(
    "li",
    { className: "history-entry" },
    el(
      "div",
      { className: "history-text" },
      el("p", {}, entry.text),
      el("span", { className: "muted small" }, `${when(entry.at)} · ${entry.seconds.toFixed(1)} s`),
    ),
    el("div", { className: "history-actions" }, copy, remove),
  );
}

export async function refreshHistory(enabled = true): Promise<void> {
  const entries = await api.listHistory().catch(() => [] as HistoryEntry[]);
  list().replaceChildren(...entries.map(row));
  document.getElementById("history-empty")!.hidden = entries.length > 0 || !enabled;
  document.getElementById("history-off")!.hidden = enabled;
  (document.getElementById("clear-history") as HTMLButtonElement).disabled = entries.length === 0;
}

export function initHistory(): void {
  document.getElementById("clear-history")!.onclick = async () => {
    if (!confirm("Delete all saved dictations? This can't be undone.")) return;
    await api.clearHistory();
    await refreshHistory();
  };
}
