// Settings window: tabs, General and Text pages, and auto-save.
import { api, el, showError, type Choices, type ModelStatus, type Replacement, type Settings } from "./api";
import { initHistory, refreshHistory } from "./history";
import { initModels } from "./models";

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
  if (name === "history") void refreshHistory(settings?.history ?? true);
}

// ---------- Saving ----------

const saveState = () => $("save-state");

/** Saves shortly after the last change, so typing in a field doesn't save on every key. */
function scheduleSave(delay = 400): void {
  window.clearTimeout(saveTimer);
  saveState().textContent = "Saving…";
  saveTimer = window.setTimeout(save, delay);
}

async function save(): Promise<void> {
  try {
    // The backend saves a cleaned copy (trimmed words, no empty rules). The page keeps
    // its own copy so half-typed rows don't vanish while the user is editing them.
    await api.saveSettings(settings);
    showError(null);
    saveState().textContent = "Saved";
    window.setTimeout(() => (saveState().textContent = ""), 1500);
    renderStatus();
  } catch (err) {
    saveState().textContent = "";
    showError(String(err));
  }
}

// ---------- General ----------

function keyLabel(code: string): string {
  return choices.talk_keys.find((k) => k.code === code)?.label ?? code;
}

function renderStatus(): void {
  const active = models.find((m) => m.selected);
  $("engine-status").textContent = !active
    ? "None yet. Download one in Models."
    : active.loaded
      ? `${active.name}, ready`
      : `${active.name}, loading…`;
  const key = keyLabel(settings.talk_key);
  $("howto").textContent =
    settings.talk_mode === "hold" ? `Hold ${key}, speak, release` : `Press ${key}, speak, press again`;

  const englishOnly = active?.english_only ?? false;
  $<HTMLSelectElement>("language").disabled = englishOnly;
  $("language-note").hidden = !englishOnly;

  const talkKey = choices.talk_keys.find((k) => k.code === settings.talk_key);
  const note = $("key-note");
  note.hidden = !talkKey || talkKey.modifier;
  note.textContent = "This key is reserved for Vocelle while it's set as the talk key.";
}

function fillSelect(select: HTMLSelectElement, options: [string, string][], value: string): void {
  select.replaceChildren(...options.map(([v, label]) => el("option", { value: v, textContent: label })));
  select.value = value;
}

function initGeneral(): void {
  const keySelect = $<HTMLSelectElement>("talk-key");
  fillSelect(
    keySelect,
    choices.talk_keys.map((k) => [k.code, k.label]),
    settings.talk_key,
  );
  keySelect.onchange = () => {
    settings.talk_key = keySelect.value;
    scheduleSave(0);
  };

  const capture = $<HTMLButtonElement>("capture-key");
  capture.onclick = () => {
    capture.textContent = "Press the key now…";
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      window.removeEventListener("keydown", onKey, true);
      capture.textContent = "Press a key…";
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

  document.querySelectorAll<HTMLInputElement>("input[name=talk-mode]").forEach((radio) => {
    radio.checked = radio.value === settings.talk_mode;
    radio.onchange = () => {
      settings.talk_mode = radio.value as Settings["talk_mode"];
      scheduleSave(0);
    };
  });

  const mic = $<HTMLSelectElement>("microphone");
  const mics: [string, string][] = [["", "System default"], ...choices.microphones.map((m): [string, string] => [m, m])];
  // Keep a saved microphone visible even if it's unplugged right now.
  if (settings.microphone && !choices.microphones.includes(settings.microphone)) {
    mics.push([settings.microphone, `${settings.microphone} (not connected)`]);
  }
  fillSelect(mic, mics, settings.microphone ?? "");
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
  };
}

// ---------- Text ----------

async function renderPreview(): Promise<void> {
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
  return el("div", { className: "replacement" }, from, el("span", { className: "muted" }, "→"), to, remove);
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
  initHistory();
  initModels((list) => {
    models = list;
    renderStatus();
  });
  renderStatus();
}

void start();
