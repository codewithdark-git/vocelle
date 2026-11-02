// Home page: greeting, totals and recent dictations grouped by day.
import { api, el, showError, type HistoryEntry, type Stats } from "./api";

/** Typing speed used to estimate time saved; a common average for typists. */
const TYPING_WPM = 40;

const $ = (id: string) => document.getElementById(id)!;

function greeting(): string {
  const hour = new Date().getHours();
  return hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
}

function minutesSaved(stats: Stats): number {
  return Math.max(0, stats.words / TYPING_WPM - stats.seconds / 60);
}

function formatDuration(minutes: number): string {
  return minutes < 60 ? `${Math.round(minutes)} min` : `${(minutes / 60).toFixed(1)} h`;
}

function dayLabel(at: number): string {
  const date = new Date(at);
  const today = new Date();
  const yesterday = new Date(today.getTime() - 86_400_000);
  if (date.toDateString() === today.toDateString()) return "Today";
  if (date.toDateString() === yesterday.toDateString()) return "Yesterday";
  return date.toLocaleDateString([], { weekday: "long", month: "short", day: "numeric" });
}

function row(entry: HistoryEntry): HTMLDivElement {
  const copy = el("button", { type: "button", className: "ghost" }, "Copy");
  copy.onclick = async () => {
    await api.copyText(entry.text).catch((err) => showError(String(err)));
    copy.textContent = "Copied";
    setTimeout(() => (copy.textContent = "Copy"), 1200);
  };
  const remove = el("button", { type: "button", className: "subtle", title: "Delete" }, "Delete");
  remove.onclick = async () => {
    await api.deleteHistoryEntry(entry.at);
    await refreshHome();
  };
  const time = new Date(entry.at).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  return el(
    "div",
    { className: "history-entry" },
    el(
      "div",
      { className: "history-text" },
      el("p", {}, entry.text),
      el("span", { className: "history-meta" }, `${time} · ${entry.seconds.toFixed(1)} s`),
    ),
    el("div", { className: "history-actions" }, copy, remove),
  );
}

let historyEnabled = true;

export async function refreshHome(enabled = historyEnabled): Promise<void> {
  historyEnabled = enabled;
  $("greeting").textContent = greeting();

  const stats = await api.getStats().catch((): Stats => ({ dictations: 0, words: 0, seconds: 0 }));
  $("stat-words").textContent = stats.words.toLocaleString();
  $("stat-saved").textContent = formatDuration(minutesSaved(stats));
  $("stat-count").textContent = stats.dictations.toLocaleString();

  const entries = enabled ? await api.listHistory().catch(() => [] as HistoryEntry[]) : [];
  const items: HTMLElement[] = [];
  let lastDay = "";
  for (const entry of entries) {
    const day = dayLabel(entry.at);
    if (day !== lastDay) {
      items.push(el("div", { className: "day" }, day));
      lastDay = day;
    }
    items.push(row(entry));
  }
  $("history").replaceChildren(...items);
  $("history-empty").hidden = !enabled || entries.length > 0;
  $("history-off").hidden = enabled;
  ($("clear-history") as HTMLButtonElement).hidden = entries.length === 0;
}

export function initHome(): void {
  $("clear-history").onclick = async () => {
    if (!confirm("Delete all saved dictations? This can't be undone.")) return;
    await api.clearHistory();
    await refreshHome();
  };
  // New dictations happen in other apps; show them when the user comes back.
  window.addEventListener("focus", () => void refreshHome());
}
