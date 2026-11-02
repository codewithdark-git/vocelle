// Welcome setup: a short guided first run (microphone, model, talk key, a
// practice dictation). Shown until finished or skipped, and from General.
import { listen } from "@tauri-apps/api/event";

import { api, el, type Choices, type ModelStatus, type Settings } from "./api";
import { enhanceSelect, fillTalkKeys } from "./select";

const STEPS = ["hello", "mic", "model", "key", "try"] as const;
type Step = (typeof STEPS)[number];

interface Context {
  settings: () => Settings;
  choices: () => Choices;
  /** Saves a settings change and refreshes the rest of the window. */
  save: (change: Partial<Settings>) => Promise<void>;
}

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;

let ctx: Context;
let current = 0;
let downloading: string | null = null;

function keyLabel(code: string): string {
  return ctx.choices().talk_keys.find((k) => k.code === code)?.label ?? code;
}

function show(index: number): void {
  current = Math.max(0, Math.min(STEPS.length - 1, index));
  const step: Step = STEPS[current];
  document.querySelectorAll<HTMLElement>(".welcome .step").forEach((section) => {
    section.hidden = section.dataset.step !== step;
  });
  $("steps").replaceChildren(
    ...STEPS.map((_, i) => el("li", { className: i <= current ? "done" : "" })),
  );
  $<HTMLButtonElement>("welcome-back").hidden = current === 0;
  $("welcome-next").textContent =
    step === "hello" ? "Get started" : step === "try" ? "Finish" : "Continue";
  if (step === "model") void renderModel();
  if (step === "key" || step === "try") renderKey();
  if (step === "try") $<HTMLTextAreaElement>("try-box").focus();
}

// ---------- Microphone ----------

function initMic(): void {
  const select = $<HTMLSelectElement>("welcome-mic");
  const fill = () => {
    const mics = ctx.choices().microphones;
    select.replaceChildren(
      el("option", { value: "", textContent: "System default" }),
      ...mics.map((m) => el("option", { value: m, textContent: m })),
    );
    select.value = ctx.settings().microphone ?? "";
  };
  fill();
  enhanceSelect(select);
  select.onchange = () => void ctx.save({ microphone: select.value || null });

  const button = $<HTMLButtonElement>("mic-test");
  const meter = $("mic-meter");
  const result = $("mic-result");
  button.onclick = async () => {
    button.disabled = true;
    result.className = "note";
    result.textContent = "Listening… say a few words.";
    meter.style.width = "0";
    // Animate while the backend listens, then show the real peak.
    const started = Date.now();
    const timer = window.setInterval(() => {
      meter.style.width = `${20 + 30 * Math.abs(Math.sin((Date.now() - started) / 180))}%`;
    }, 60);
    try {
      const peak = await api.testMicrophone(select.value || null);
      meter.style.width = `${Math.round(Math.min(1, peak) * 100)}%`;
      if (peak > 0.08) {
        result.className = "note good";
        result.textContent = "Sounds good. We can hear you.";
      } else {
        result.className = "note bad";
        result.textContent = "That was very quiet. Check the microphone, or pick another one.";
      }
    } catch (err) {
      meter.style.width = "0";
      result.className = "note bad";
      result.textContent = String(err);
    } finally {
      window.clearInterval(timer);
      button.disabled = false;
    }
  };
}

// ---------- Model ----------

function formatSize(bytes: number): string {
  return `${Math.round(bytes / 1e6)} MB`;
}

async function renderModel(): Promise<void> {
  const models = await api.listModels().catch(() => [] as ModelStatus[]);
  const box = $("welcome-model");
  const active = models.find((m) => m.selected && m.downloaded);
  const pick = models.find((m) => m.recommended) ?? models[0];
  if (!pick) return;

  const cards: HTMLElement[] = [];
  if (active) {
    cards.push(modelCard(active, active.loaded ? "✓ Ready" : "Loading…"));
  }
  if (!active || (pick.id !== active.id && !pick.downloaded)) {
    cards.push(modelCard(pick, null));
  }
  box.replaceChildren(...cards);
}

function modelCard(m: ModelStatus, state: string | null): HTMLElement {
  const title = el("div", { className: "model-name" }, m.name);
  if (m.recommended) title.append(el("span", { className: "tag" }, "Recommended for this PC"));
  title.append(el("span", { className: "tag neutral" }, formatSize(m.size_bytes)));
  const action = el("div", { className: "model-action" });
  if (state) {
    action.append(el("span", { className: "model-state" }, state));
  } else if (downloading === m.id || m.downloading) {
    const bar = el("progress", { max: 100, value: 0 });
    bar.id = "welcome-progress";
    action.append(bar);
  } else {
    const button = el("button", { className: "primary", type: "button" }, "Download");
    button.onclick = () => void download(m.id);
    action.append(button);
  }
  return el(
    "div",
    { className: state ? "model in-use" : "model" },
    el("div", {}, title, el("div", { className: "model-meta" }, m.description)),
    action,
  );
}

async function download(id: string): Promise<void> {
  downloading = id;
  await renderModel();
  await api.downloadModel(id).catch(() => (downloading = null));
}

function initModelEvents(): void {
  void listen<{ id: string; done: number; total: number }>("model-progress", ({ payload }) => {
    const bar = document.getElementById("welcome-progress") as HTMLProgressElement | null;
    if (bar && payload.id === downloading) bar.value = Math.floor((payload.done / payload.total) * 100);
  });
  void listen<{ id: string; error: string | null }>("model-downloaded", async ({ payload }) => {
    if (payload.id !== downloading) return;
    downloading = null;
    // Use the model the user just chose here, even if another was active.
    if (!payload.error) await api.selectModel(payload.id).catch(() => {});
    if (STEPS[current] === "model") await renderModel();
  });
  void listen("model-loaded", () => {
    if (STEPS[current] === "model") void renderModel();
  });
}

// ---------- Talk key ----------

function renderKey(): void {
  const label = keyLabel(ctx.settings().talk_key);
  $("welcome-key").textContent = label;
  $("try-key").textContent = label;
  $<HTMLSelectElement>("welcome-key-select").value = ctx.settings().talk_key;
}

function initKey(): void {
  const select = $<HTMLSelectElement>("welcome-key-select");
  fillTalkKeys(select, ctx.choices().talk_keys, ctx.settings().talk_key);
  enhanceSelect(select);
  select.onchange = async () => {
    await ctx.save({ talk_key: select.value });
    renderKey();
  };
  const capture = $<HTMLButtonElement>("welcome-capture");
  capture.onclick = () => {
    capture.textContent = "Press it now…";
    capture.classList.add("capturing");
    const onKey = async (e: KeyboardEvent) => {
      e.preventDefault();
      window.removeEventListener("keydown", onKey, true);
      capture.textContent = "Press a key…";
      capture.classList.remove("capturing");
      if (ctx.choices().talk_keys.some((k) => k.code === e.code)) {
        await ctx.save({ talk_key: e.code });
        renderKey();
      }
    };
    window.addEventListener("keydown", onKey, true);
  };
}

// ---------- Try it ----------

function initTry(): void {
  const box = $<HTMLTextAreaElement>("try-box");
  box.oninput = () => {
    const result = $("try-result");
    if (box.value.trim()) {
      result.className = "note good";
      result.textContent = "That's it. Vocelle works the same way in any app.";
    }
  };
}

// ---------- Flow ----------

async function close(): Promise<void> {
  $("welcome").hidden = true;
  if (!ctx.settings().onboarded) await ctx.save({ onboarded: true });
}

export function openWelcome(): void {
  $("welcome").hidden = false;
  show(0);
}

export function initWelcome(context: Context): void {
  ctx = context;
  initMic();
  initKey();
  initTry();
  initModelEvents();
  $("welcome-next").onclick = () => {
    if (current === STEPS.length - 1) void close();
    else show(current + 1);
  };
  $("welcome-back").onclick = () => show(current - 1);
  $("welcome-skip").onclick = () => void close();
  if (!ctx.settings().onboarded) openWelcome();
}
