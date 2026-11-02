// Main window: Home, General and Text pages, auto-save, and the welcome setup.
import { api, el, showError, type Choices, type ModelStatus, type Replacement, type Settings } from "./api";
import { initHome, refreshHome } from "./home";
import { initModels } from "./models";
import { enhanceSelect, fillTalkKeys } from "./select";
import { initWelcome, openWelcome } from "./welcome";

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;

let settings: Settings;
let choices: Choices;
let models: ModelStatus[] = [];
let saveTimer: number | undefined;

// ---------- Tabs ----------

function showTab(name: string): void {
  document.querySelectorAll<HTMLButtonElement>("[role=tab]").forEach((tab) => {
    tab.setAttribute("aria-selected", String(tab.dataset.tab === name));
  });
  document.querySelectorAll<HTMLElement>(".page").forEach((page) => {
    page.hidden = page.id !== `page-${name}`;
  });
  if (name === "home") void refreshHome(settings?.history ?? true);
}

// ---------- Saving ----------

let toastTimer: number | undefined;

function toast(message: string): void {
  const box = $("save-state");
  box.textContent = message;
  box.classList.add("visible");
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => box.classList.remove("visible"), 1400);
}

/** Saves shortly after the last change, so typing in a field doesn't save on every key. */
function scheduleSave(delay = 400): void {
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(save, delay);
}

async function save(): Promise<void> {
  try {
    // The backend saves a cleaned copy (trimmed words, no empty rules). The page keeps
    // its own copy so half-typed rows don't vanish while the user is editing them.
    await api.saveSettings(settings);
    showError(null);
    toast("Saved");
    renderStatus();
  } catch (err) {
    showError(String(err));
  }
}

// ---------- General ----------

function keyLabel(code: string): string {
  return choices.talk_keys.find((k) => k.code === code)?.label ?? code;
}

function renderStatus(): void {
  const active = models.find((m) => m.selected);
  const engine = $("engine");
  engine.dataset.state = !active ? "missing" : active.loaded ? "ready" : "loading";
  $("engine-status").textContent = !active
    ? "No model yet"
    : active.loaded
      ? `${active.name} ready`
      : `Loading ${active.name}…`;
  $("hero-key").textContent = keyLabel(settings.talk_key);

  const englishOnly = active?.english_only ?? false;
  $<HTMLSelectElement>("language").disabled = englishOnly;
  $("language-note").textContent = englishOnly
    ? "English-only models always transcribe English."
    : "The language you'll speak, or detect it automatically.";

  const talkKey = choices.talk_keys.find((k) => k.code === settings.talk_key);
  $("key-note").textContent =
    talkKey && !talkKey.modifier
      ? "Hold it while you speak. It's reserved for Vocelle while set here."
      : "Hold it while you speak. Shortcuts that use it still work.";
}

function fillSelect(select: HTMLSelectElement, options: [string, string][], value: string): void {
  select.replaceChildren(...options.map(([v, label]) => el("option", { value: v, textContent: label })));
  select.value = value;
}

function initGeneral(): void {
  const keySelect = $<HTMLSelectElement>("talk-key");
  fillTalkKeys(keySelect, choices.talk_keys, settings.talk_key);
  enhanceSelect(keySelect);
  keySelect.onchange = () => {
    settings.talk_key = keySelect.value;
    scheduleSave(0);
  };

  const capture = $<HTMLButtonElement>("capture-key");
  capture.onclick = () => {
    capture.textContent = "Press it now…";
    capture.classList.add("capturing");
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      window.removeEventListener("keydown", onKey, true);
      capture.textContent = "Press a key…";
      capture.classList.remove("capturing");
      if (choices.talk_keys.some((k) => k.code === e.code)) {
        settings.talk_key = e.code;
        keySelect.value = e.code;
        scheduleSave(0);
      } else {
        showError(`${e.code || e.key} can't be the talk key. Try Right Ctrl, Caps Lock or an F-key.`);
      }
    };
    window.addEventListener("keydown", onKey, true);
  };

  const mic = $<HTMLSelectElement>("microphone");
  const mics: [string, string][] = [["", "System default"], ...choices.microphones.map((m): [string, string] => [m, m])];
  // Keep a saved microphone visible even if it's unplugged right now.
  if (settings.microphone && !choices.microphones.includes(settings.microphone)) {
    mics.push([settings.microphone, `${settings.microphone} (not connected)`]);
  }
  fillSelect(mic, mics, settings.microphone ?? "");
  enhanceSelect(mic);
  mic.onchange = () => {
    settings.microphone = mic.value || null;
    scheduleSave(0);
  };

  const language = $<HTMLSelectElement>("language");
  fillSelect(
    language,
    choices.languages.map((l) => [l.code, l.name]),
    settings.language,
  );
  enhanceSelect(language);
  language.onchange = () => {
    settings.language = language.value;
    scheduleSave(0);
  };

  const autostart = $<HTMLInputElement>("autostart");
  void api.getAutostart().then((on) => (autostart.checked = on));
  autostart.onchange = async () => {
    await api.setAutostart(autostart.checked).catch((err) => {
      autostart.checked = !autostart.checked;
      showError(String(err));
    });
  };

  const historyToggle = $<HTMLInputElement>("history-enabled");
  historyToggle.checked = settings.history;
  historyToggle.onchange = () => {
    settings.history = historyToggle.checked;
    scheduleSave(0);
    void refreshHome(settings.history);
  };

  $("rerun-welcome").onclick = () => openWelcome();
}

// ---------- Text ----------

const CLEANUP_HELP: Record<Settings["cleanup"], string> = {
  off: "Types exactly what the model heard.",
  light: "Removes “um”, “uh” and similar, and fixes spacing.",
  full: "Light, plus stutters (“the the”), sentence capitals and “i” → “I”.",
};

async function renderPreview(): Promise<void> {
  $("cleanup-help").textContent = CLEANUP_HELP[settings.cleanup];
  const input = $<HTMLInputElement>("preview-input").value;
  $("preview-output").textContent = await api
    .previewCleanup(input, settings.cleanup, settings.replacements)
    .catch(() => input);
}

function replacementRow(rule: Replacement, index: number): HTMLDivElement {
  const from = el("input", { type: "text", value: rule.from, placeholder: "When I say…" });
  const to = el("input", { type: "text", value: rule.to, placeholder: "Type…" });
  const update = () => {
    settings.replacements[index] = { from: from.value, to: to.value };
    void renderPreview();
    scheduleSave();
  };
  from.oninput = update;
  to.oninput = update;
  const remove = el("button", { type: "button", className: "subtle", title: "Remove" }, "Remove");
  remove.onclick = () => {
    settings.replacements.splice(index, 1);
    renderReplacements();
    void renderPreview();
    scheduleSave(0);
  };
  return el("div", { className: "replacement" }, from, el("span", { className: "arrow" }, "→"), to, remove);
}

function renderReplacements(): void {
  $("replacements").replaceChildren(...settings.replacements.map(replacementRow));
}

function initText(): void {
  document.querySelectorAll<HTMLInputElement>("input[name=cleanup]").forEach((radio) => {
    radio.checked = radio.value === settings.cleanup;
    radio.onchange = () => {
      settings.cleanup = radio.value as Settings["cleanup"];
      void renderPreview();
      scheduleSave(0);
    };
  });
  $<HTMLInputElement>("preview-input").oninput = () => void renderPreview();

  const trailing = $<HTMLInputElement>("trailing-space");
  trailing.checked = settings.trailing_space;
  trailing.onchange = () => {
    settings.trailing_space = trailing.checked;
    scheduleSave(0);
  };

  const dictionary = $<HTMLTextAreaElement>("dictionary");
  dictionary.value = settings.dictionary.join("\n");
  dictionary.oninput = () => {
    settings.dictionary = dictionary.value.split("\n");
    scheduleSave(800);
  };
  // The backend trims entries; show the cleaned list once editing is done.
  dictionary.onblur = () => (dictionary.value = settings.dictionary.filter((w) => w.trim()).join("\n"));

  $("add-replacement").onclick = () => {
    settings.replacements.push({ from: "", to: "" });
    renderReplacements();
    const inputs = $("replacements").querySelectorAll("input");
    (inputs[inputs.length - 2] as HTMLInputElement | undefined)?.focus();
  };
  renderReplacements();
  void renderPreview();
}

// ---------- Start ----------

async function start(): Promise<void> {
  document.querySelectorAll<HTMLButtonElement>("[role=tab]").forEach((tab) => {
    tab.onclick = () => showTab(tab.dataset.tab!);
  });
  try {
    const [info, loaded, available] = await Promise.all([api.appInfo(), api.getSettings(), api.choices()]);
    $("version").textContent = `v${info.version}`;
    settings = loaded;
    choices = available;
  } catch (err) {
    showError(String(err));
    return;
  }
  initGeneral();
  initText();
  initHome();
  initModels((list) => {
    models = list;
    renderStatus();
  });
  renderStatus();
  void refreshHome(settings.history);
  initWelcome({
    settings: () => settings,
    choices: () => choices,
    save: async (change) => {
      Object.assign(settings, change);
      await save();
      // Keep the General page in step with choices made in the welcome steps.
      $<HTMLSelectElement>("talk-key").value = settings.talk_key;
      $<HTMLSelectElement>("microphone").value = settings.microphone ?? "";
    },
  });
}

void start();
