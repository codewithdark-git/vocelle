import { invoke } from "@tauri-apps/api/core";

interface AppInfo {
  name: string;
  version: string;
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

void showAppInfo();
