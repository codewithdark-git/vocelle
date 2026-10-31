import { listen } from "@tauri-apps/api/event";

type Phase = "listening" | "transcribing" | "done" | "error";

interface Status {
  phase: Phase;
  message: string | null;
}

const LABELS: Record<Phase, string> = {
  listening: "Listening…",
  transcribing: "Transcribing…",
  done: "Done",
  error: "Something went wrong",
};

// Bar heights relative to the input level, so the meter looks like a small waveform.
const BAR_WEIGHTS = [0.5, 0.8, 1, 0.8, 0.5];

const pill = document.getElementById("pill")!;
const label = document.getElementById("label")!;
const bars = Array.from(document.querySelectorAll<HTMLSpanElement>(".meter span"));

void listen<Status>("overlay-status", ({ payload }) => {
  pill.dataset.phase = payload.phase;
  label.textContent = payload.message ?? LABELS[payload.phase];
  if (payload.phase !== "listening") setLevel(0);
});

void listen<number>("overlay-level", ({ payload }) => setLevel(payload));

function setLevel(level: number): void {
  bars.forEach((bar, i) => {
    const h = 4 + Math.round(level * BAR_WEIGHTS[i] * 16);
    bar.style.height = `${h}px`;
  });
}
