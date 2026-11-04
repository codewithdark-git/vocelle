"use client";

// The held key. Press the keycap (or the real Right Ctrl): the headline
// stretches with a simulated voice and words arrive as uncertain tokens.
// Let go and they settle into clean, typed text, as Vocelle does.
import { animate } from "motion";
import dynamic from "next/dynamic";
import { motion, useReducedMotion } from "motion/react";
import { useCallback, useEffect, useRef, useState } from "react";

import { DownloadLink, ReleaseMeta } from "./Download";
import styles from "./Hero.module.css";

// The 3D keycap loads after the page; the CSS keycap shows until then, and
// stays for reduced motion or browsers without WebGL.
const KeyCap3D = dynamic(() => import("./KeyCap3D"), { ssr: false });

type Token = { word: string; filler?: boolean };

// What someone might say, fillers and all. Cleanup drops the fillers.
const UTTERANCES: Token[][] = [
  [
    { word: "so" , filler: true }, { word: "can" }, { word: "we" }, { word: "move" }, { word: "the" },
    { word: "review" }, { word: "um", filler: true }, { word: "to" }, { word: "Thursday" },
    { word: "at" }, { word: "three?" },
  ],
  [
    { word: "thanks" }, { word: "for" }, { word: "the" }, { word: "notes," }, { word: "uh", filler: true },
    { word: "I'll" }, { word: "send" }, { word: "the" }, { word: "new" }, { word: "draft" },
    { word: "tonight." },
  ],
  [
    { word: "remind" }, { word: "me" }, { word: "to" }, { word: "call" }, { word: "the" },
    { word: "dentist" }, { word: "like", filler: true }, { word: "tomorrow" }, { word: "at" }, { word: "ten." },
  ],
];

const WORD_MS = 230;
const MIN_HOLD_MS = 250;

function settle(tokens: Token[]): string {
  const words = tokens.filter((t) => !t.filler).map((t) => t.word);
  const text = words.join(" ");
  return text.charAt(0).toUpperCase() + text.slice(1);
}

export function Hero() {
  const reduce = useReducedMotion();
  const headline = useRef<HTMLHeadingElement>(null);
  const field = useRef<HTMLElement>(null);
  const [held, setHeld] = useState(false);
  const [heard, setHeard] = useState<Token[]>([]);
  const [typed, setTyped] = useState<string[]>([]);
  const [threeD, setThreeD] = useState(false);
  const [hint, setHint] = useState("Press and hold the key. Your keyboard's Right Ctrl works too.");

  const holding = useRef(false);
  const since = useRef(0);
  const utterance = useRef(0);
  const timer = useRef<number>(0);
  const frame = useRef<number>(0);

  // The voice: a smooth, uneven envelope that drives letter width and heat.
  const voice = useCallback((t: number) => {
    const level = holding.current
      ? 0.5 + 0.5 * Math.sin(t / 140) * Math.sin(t / 370 + 1.3) * Math.sin(t / 900 + 0.4)
      : 0;
    const el = headline.current;
    if (el) el.style.setProperty("--w", String(100 + level * 32));
    field.current?.style.setProperty("--heat", String(holding.current ? 0.55 + level * 0.45 : 0));
    if (holding.current) frame.current = requestAnimationFrame(voice);
  }, []);

  const press = useCallback(() => {
    if (holding.current) return;
    holding.current = true;
    since.current = performance.now();
    setHeld(true);
    setHeard([]);
    setHint("Listening. Let go when you're done.");
    const words = UTTERANCES[utterance.current % UTTERANCES.length];
    let i = 0;
    window.clearInterval(timer.current);
    timer.current = window.setInterval(() => {
      if (i >= words.length) return window.clearInterval(timer.current);
      const next = words[i++];
      setHeard((h) => [...h, next]);
    }, WORD_MS);
    if (!reduce) frame.current = requestAnimationFrame(voice);
  }, [reduce, voice]);

  const release = useCallback(() => {
    if (!holding.current) return;
    holding.current = false;
    setHeld(false);
    window.clearInterval(timer.current);
    cancelAnimationFrame(frame.current);
    voice(0);
    if (performance.now() - since.current < MIN_HOLD_MS) {
      setHint("Hold it a little longer, then let go.");
      setHeard([]);
      return;
    }
    const words = UTTERANCES[utterance.current++ % UTTERANCES.length];
    // Whatever was said so far settles; if released early, the rest is "heard" too.
    setHeard(words);
    window.setTimeout(() => {
      setTyped((t) => [...t.slice(-2), settle(words)]);
      setHeard([]);
      setHint("Typed. Hold it again for another.");
    }, reduce ? 0 : 420);
  }, [reduce, voice]);

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.code === "ControlRight" && !e.repeat) press();
    };
    const up = (e: KeyboardEvent) => {
      if (e.code === "ControlRight") release();
    };
    addEventListener("keydown", down);
    addEventListener("keyup", up);
    addEventListener("blur", release);
    return () => {
      removeEventListener("keydown", down);
      removeEventListener("keyup", up);
      removeEventListener("blur", release);
    };
  }, [press, release]);

  useEffect(() => {
    if (reduce) return;
    const probe = document.createElement("canvas");
    if (!(probe.getContext("webgl2") || probe.getContext("webgl"))) return;
    // The legend is drawn in Anybody, so wait for the font.
    document.fonts.ready.then(() => setThreeD(true));
  }, [reduce]);

  // The one orchestrated page-load moment: the headline opens up from narrow.
  useEffect(() => {
    const el = headline.current;
    if (!el || reduce) return;
    const controls = animate(60, 100, {
      duration: 1.1,
      ease: [0.23, 1, 0.32, 1],
      onUpdate: (v) => el.style.setProperty("--w", String(v)),
    });
    return () => controls.stop();
  }, [reduce]);

  return (
    <section className={styles.hero} ref={field}>
      <header className={`container ${styles.nav}`}>
        <a className={styles.brand} href="#top" aria-label="Vocelle, back to top">
          <img src={`${process.env.NEXT_PUBLIC_BASE_PATH}/logo-mark.svg`} alt="" width={30} height={30} />
          Vocelle
        </a>
        <nav className={styles.links} aria-label="Sections">
          <a href="#anywhere">Where it works</a>
          <a href="#app">The app</a>
          <a href="#privacy">Privacy</a>
          <a href="#specs">Specs</a>
        </nav>
        <DownloadLink className={styles.navButton}>Download</DownloadLink>
      </header>

      <div className={`container ${styles.body}`} id="top">
        <h1 ref={headline} className={`display ${styles.headline}`} data-held={held}>
          Hold the key{" "}
          <br />
          and talk.
        </h1>

        <div className={styles.lower}>
          <div className={styles.copy}>
            <p className="lede">
              Vocelle types what you say into whatever app you&rsquo;re using, on Windows and Linux. The speech
              recognition runs on your own processor, so your voice never leaves the computer.
            </p>
            <div className={styles.actions}>
              <DownloadLink className="button">Download for Windows</DownloadLink>
              <a className={styles.secondary} href="https://github.com/codewithdark-git/vocelle/releases/latest">
                Linux packages
              </a>
            </div>
            <p className={styles.meta}>
              Free, with no account. <ReleaseMeta />.
            </p>
          </div>

          <div className={styles.demo}>
            <div className={styles.sheet} aria-live="polite">
              {typed.map((line, i) => (
                <motion.p
                  key={`${i}-${line}`}
                  className={styles.typed}
                  initial={reduce ? false : { opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.35, ease: [0.23, 1, 0.32, 1] }}
                >
                  {line}
                </motion.p>
              ))}
              <p className={styles.heard} aria-hidden="true">
                {heard.map((t, i) => (
                  <motion.span
                    key={i}
                    className={t.filler ? styles.filler : undefined}
                    initial={reduce ? false : { opacity: 0, filter: "blur(4px)" }}
                    animate={{ opacity: 1, filter: "blur(0px)" }}
                    transition={{ duration: 0.25 }}
                  >
                    {t.word}{" "}
                  </motion.span>
                ))}
                <span className={styles.caret} />
              </p>
              {typed.length === 0 && heard.length === 0 && (
                <p className={styles.placeholder}>Your words will be typed here.</p>
              )}
            </div>

            <button
              type="button"
              className={styles.key}
              data-down={held}
              data-3d={threeD || undefined}
              aria-label="Talk key: press and hold"
              aria-describedby="hero-hint"
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
              onKeyUp={(e) => {
                if (e.key === " " || e.key === "Enter") release();
              }}
            >
              {threeD ? (
                <KeyCap3D down={held} />
              ) : (
                <span className={styles.keyTop}>
                  <span className={styles.keyLabel}>Ctrl</span>
                  <span className={styles.keySub}>Right</span>
                </span>
              )}
            </button>
            <p id="hero-hint" className={styles.hint}>
              {hint}
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
