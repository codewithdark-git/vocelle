"use client";

// The product, working: a desktop with an app window, a taskbar and
// Vocelle's real indicator pill. A loop plays four everyday dictations; hold
// Right Ctrl (or the key on stage) to drive one yourself.
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { useCallback, useEffect, useRef, useState } from "react";

import styles from "./DemoStage.module.css";

type Scene = {
  app: "mail" | "chat" | "code" | "notes";
  title: string;
  context: string;
  said: string[]; // what was heard, fillers and all
  typed: string; // what Vocelle types
};

const SCENES: Scene[] = [
  {
    app: "mail",
    title: "Re: Thursday launch",
    context: "To: Sam Okafor",
    said: ["um", "so", "the", "launch", "is", "moving", "to", "thursday", "uh", "can", "you", "tell", "the", "team"],
    typed: "So the launch is moving to Thursday. Can you tell the team?",
  },
  {
    app: "chat",
    title: "# design",
    context: "Priya: new icons are up for review",
    said: ["oh", "these", "look", "great", "um", "ship", "them", "today"],
    typed: "Oh, these look great. Ship them today.",
  },
  {
    app: "code",
    title: "download.rs",
    context: "fn verify(path: &Path) -> Result<()> {",
    said: ["todo", "retry", "the", "the", "download", "once", "if", "the", "checksum", "fails"],
    typed: "// TODO: retry the download once if the checksum fails",
  },
  {
    app: "notes",
    title: "Tuesday",
    context: "Groceries, calls, ideas",
    said: ["remind", "me", "to", "call", "the", "dentist", "uh", "tomorrow", "at", "ten"],
    typed: "Remind me to call the dentist tomorrow at 10.",
  },
];

const FILLERS = new Set(["um", "uh", "oh"]);

type Phase = "idle" | "listening" | "thinking" | "done";

export function DemoStage() {
  const reduce = useReducedMotion();
  const root = useRef<HTMLDivElement>(null);
  const [scene, setScene] = useState(0);
  const [phase, setPhase] = useState<Phase>("idle");
  const [heard, setHeard] = useState(0);
  const [typed, setTyped] = useState(false);
  const [visible, setVisible] = useState(false);
  const [manual, setManual] = useState(false);
  const timers = useRef<number[]>([]);
  const holding = useRef(false);

  const s = SCENES[scene];

  const clear = () => {
    timers.current.forEach((t) => clearTimeout(t));
    timers.current = [];
  };
  const after = (ms: number, fn: () => void) => timers.current.push(window.setTimeout(fn, ms));

  // Moves on from `from` only; a second call for the same scene does nothing.
  const advance = useCallback((from: number) => {
    setPhase("idle");
    setHeard(0);
    setTyped(false);
    setScene((i) => (i === from ? (from + 1) % SCENES.length : i));
  }, []);

  // Letting go: the pill thinks, then the clean text lands, then a check.
  const finish = useCallback(
    (from: number, next: boolean) => {
      setPhase("thinking");
      after(650, () => {
        setTyped(true);
        setPhase("done");
        if (next) after(2600, () => advance(from));
      });
    },
    [advance],
  );

  // The automatic loop, while the stage is on screen and nobody is driving.
  useEffect(() => {
    if (reduce) {
      setHeard(s.said.length);
      setTyped(true);
      setPhase("done");
      return;
    }
    if (!visible || manual) return;
    clear();
    after(900, () => {
      setPhase("listening");
      s.said.forEach((_, i) => after(260 * (i + 1), () => setHeard(i + 1)));
      after(260 * s.said.length + 500, () => finish(scene, true));
    });
    return clear;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scene, visible, manual, reduce]);

  useEffect(() => {
    const io = new IntersectionObserver(([e]) => setVisible(e.isIntersecting), { threshold: 0.35 });
    if (root.current) io.observe(root.current);
    return () => io.disconnect();
  }, []);

  // Driving it yourself.
  const press = useCallback(() => {
    if (holding.current) return;
    holding.current = true;
    clear();
    setManual(true);
    setTyped(false);
    setHeard(0);
    setPhase("listening");
    const words = SCENES[scene].said;
    words.forEach((_, i) => after(230 * (i + 1), () => setHeard(i + 1)));
  }, [scene]);

  const release = useCallback(() => {
    if (!holding.current) return;
    holding.current = false;
    clear();
    setHeard(SCENES[scene].said.length);
    finish(scene, false);
    // Hand back to the loop after a pause.
    after(3600, () => {
      setManual(false);
      advance(scene);
    });
  }, [scene, finish, advance]);

  useEffect(() => {
    const down = (e: KeyboardEvent) => e.code === "ControlRight" && !e.repeat && press();
    const up = (e: KeyboardEvent) => e.code === "ControlRight" && release();
    addEventListener("keydown", down);
    addEventListener("keyup", up);
    return () => {
      removeEventListener("keydown", down);
      removeEventListener("keyup", up);
    };
  }, [press, release]);

  const keyDown = phase === "listening";

  return (
    <div className={styles.stage} ref={root} data-app={s.app}>
      <div className={styles.wallpaper} aria-hidden="true" />

      <AnimatePresence mode="wait">
        <motion.div
          key={scene}
          className={`${styles.window} ${styles[s.app]}`}
          initial={reduce ? false : { opacity: 0, y: 24, scale: 0.97 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: -16, scale: 0.98 }}
          transition={{ duration: 0.55, ease: [0.23, 1, 0.32, 1] }}
        >
          <div className={styles.titlebar}>
            <span className={styles.lights} aria-hidden="true" />
            <span className={styles.app}>{appName(s.app)}</span>
            <span className={styles.title}>{s.title}</span>
          </div>
          <div className={styles.body}>
            <p className={styles.context}>{s.context}</p>
            <div className={styles.field}>
              {typed ? (
                <motion.span
                  className={styles.typed}
                  initial={reduce ? false : { backgroundSize: "100% 100%" }}
                  animate={{ backgroundSize: "0% 100%" }}
                  transition={{ duration: 0.9, ease: [0.23, 1, 0.32, 1], delay: 0.15 }}
                >
                  {s.typed}
                </motion.span>
              ) : (
                <span className={styles.placeholder}>{placeholder(s.app)}</span>
              )}
              <span className={styles.caret} aria-hidden="true" />
            </div>
          </div>
        </motion.div>
      </AnimatePresence>

      {/* Live caption: what the microphone is hearing, fillers included. */}
      <div className={styles.caption} data-show={phase === "listening" || phase === "thinking"} aria-hidden="true">
        {s.said.slice(0, heard).map((w, i) => (
          <span key={i} className={FILLERS.has(w) ? styles.filler : undefined}>
            {w}{" "}
          </span>
        ))}
      </div>

      <div className={styles.taskbar} aria-hidden="true">
        <span />
        <span />
        <span />
        <span />
      </div>

      {/* Vocelle's indicator, as in the app. */}
      <div className={styles.pill} data-phase={phase} aria-hidden="true">
        <span className={styles.bars}>
          {Array.from({ length: 7 }, (_, i) => (
            <i key={i} style={{ animationDelay: `${(i * -0.13).toFixed(2)}s` }} />
          ))}
        </span>
        <span className={styles.dots}>
          <i />
          <i />
          <i />
        </span>
        <svg className={styles.check} viewBox="0 0 24 24">
          <path d="M5 12.5l4.5 4.5L19 7.5" />
        </svg>
      </div>

      <button
        type="button"
        className={styles.key}
        data-down={keyDown}
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId);
          press();
        }}
        onPointerUp={release}
        onPointerCancel={release}
        onKeyDown={(e) => {
          if ((e.key === " " || e.key === "Enter") && !e.repeat) {
            e.preventDefault();
            press();
          }
        }}
        onKeyUp={(e) => (e.key === " " || e.key === "Enter") && release()}
      >
        Right Ctrl
        <span className={styles.keyHint}>{keyDown ? "Listening" : "Hold to try"}</span>
      </button>

      <p className="visually-hidden" aria-live="polite">
        {typed ? `Typed into ${appName(s.app)}: ${s.typed}` : ""}
      </p>
    </div>
  );
}

function appName(app: Scene["app"]) {
  return { mail: "Mail", chat: "Team chat", code: "Code editor", notes: "Notes" }[app];
}

function placeholder(app: Scene["app"]) {
  return { mail: "Write a reply…", chat: "Message #design", code: "", notes: "Start typing…" }[app];
}
