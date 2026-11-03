// Typed wrappers for the backend commands.
import { invoke } from "@tauri-apps/api/core";

export type CleanupLevel = "off" | "light" | "full";

export interface Replacement {
  from: string;
  to: string;
}

export interface Settings {
  model: string | null;
  talk_key: string;
  microphone: string | null;
  language: string;
  cleanup: CleanupLevel;
  trailing_space: boolean;
  dictionary: string[];
  replacements: Replacement[];
  history: boolean;
  onboarded: boolean;
}

export interface Stats {
  dictations: number;
  words: number;
  seconds: number;
  recent_speed: number[];
}

export interface TalkKey {
  code: string;
  label: string;
  modifier: boolean;
}

export interface Choices {
  talk_keys: TalkKey[];
  languages: { code: string; name: string }[];
  microphones: string[];
}

export interface AppInfo {
  name: string;
  version: string;
  platform: string;
}

export interface ModelStatus {
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

export interface HistoryEntry {
  at: number;
  text: string;
  seconds: number;
  model: string;
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  choices: () => invoke<Choices>("settings_choices"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  previewCleanup: (text: string, level: CleanupLevel, replacements: Replacement[]) =>
    invoke<string>("preview_cleanup", { text, level, replacements }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
  listModels: () => invoke<ModelStatus[]>("list_models"),
  downloadModel: (id: string) => invoke<void>("download_model", { id }),
  selectModel: (id: string) => invoke<void>("select_model", { id }),
  listHistory: () => invoke<HistoryEntry[]>("list_history"),
  deleteHistoryEntry: (at: number) => invoke<void>("delete_history_entry", { at }),
  clearHistory: () => invoke<void>("clear_history"),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  getStats: () => invoke<Stats>("get_stats"),
  testMicrophone: (microphone: string | null) => invoke<number>("test_microphone", { microphone }),
};

const errorBox = () => document.getElementById("error")!;

export function showError(message: string | null): void {
  const box = errorBox();
  box.hidden = !message;
  box.textContent = message ?? "";
}

export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: Partial<HTMLElementTagNameMap[K]> = {},
  ...children: (Node | string)[]
): HTMLElementTagNameMap[K] {
  const node = Object.assign(document.createElement(tag), props);
  node.append(...children);
  return node;
}
