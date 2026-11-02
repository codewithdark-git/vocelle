import { listen } from "@tauri-apps/api/event";

type Phase = "listening" | "transcribing" | "done" | "empty" | "error";

interface Status {
  phase: Phase;
  message: string | null;
}

// Screen readers still get a name for each phase, even though nothing is written on screen.
const NAMES: Record<Phase, string> = {
  listening: "Listening",
  transcribing: "Transcribing",
  done: "Done",
  empty: "No speech heard",
  error: "Something went wrong",
};

// Middle bars move most, so the meter reads as a small waveform.
const BAR_WEIGHTS = [0.35, 0.6, 0.85, 1, 0.85, 0.6, 0.35];
const MIN_HEIGHT = 4;
const MAX_HEIGHT = 22;

const pill = document.getElementById("pill")!;
const bars = Array.from(document.querySelectorAll<HTMLSpanElement>(".bars span"));

// The backend sends the level every 50 ms. Easing towards it, with a little
// per-bar wobble, keeps the meter alive between updates and during pauses.
let target = 0;
let level = 0;
let frame = 0;

function animate(time: number): void {
  level += (target - level) * 0.35;
  bars.forEach((bar, i) => {
    const wobble = 0.12 * (1 + Math.sin(time / 160 + i * 1.3)) * (0.3 + level);
    const value = Math.min(1, level * BAR_WEIGHTS[i] + wobble * BAR_WEIGHTS[i]);
    bar.style.height = `${MIN_HEIGHT + value * (MAX_HEIGHT - MIN_HEIGHT)}px`;
  });
  frame = requestAnimationFrame(animate);
}

function setPhase(phase: Phase, message: string | null): void {
  pill.dataset.phase = phase;
  pill.setAttribute("aria-label", message ?? NAMES[phase]);
  cancelAnimationFrame(frame);
  if (phase === "listening") {
    target = level = 0;
    frame = requestAnimationFrame(animate);
  }
  // Restart the one-shot animations (check stroke, shake) on every change.
  pill.classList.remove("enter");
  void pill.offsetWidth;
  pill.classList.add("enter");
}

void listen<Status>("overlay-status", ({ payload }) => setPhase(payload.phase, payload.message));
// Speech levels are small numbers; a square root makes quiet voices visible.
void listen<number>("overlay-level", ({ payload }) => (target = Math.min(1, Math.sqrt(payload) * 1.4)));

setPhase("listening", null);
