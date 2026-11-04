"use client";

// Typing vs talking, the same message. Typing appears letter by letter at
// 40 wpm; with Vocelle you talk at 150 wpm and the text lands when you let
// go. Played four times faster than real time, and the page says so.
import { useReducedMotion } from "motion/react";
import { useEffect, useRef, useState } from "react";

import styles from "./Race.module.css";

const MESSAGE =
  "Hi Sam, the launch is moving to Thursday. Can you update the release notes and let the support team know before the end of the day?";
const WORDS = MESSAGE.split(" ").length; // 26
const TYPE_S = (WORDS / 40) * 60; // 39 s at 40 wpm
const TALK_S = (WORDS / 150) * 60; // 10.4 s at 150 wpm
const INSERT_S = 1; // from letting go to text, on a typical laptop
const SPEEDUP = 4;

export function Race() {
  const reduce = useReducedMotion();
  const root = useRef<HTMLElement>(null);
  const [t, setT] = useState(0); // real seconds elapsed
  const [run, setRun] = useState(0);

  useEffect(() => {
    if (reduce) {
      setT(TYPE_S);
      return;
    }
    const el = root.current;
    if (!el) return;
    let frame = 0;
    let start = 0;
    const step = (now: number) => {
      start ||= now;
      const elapsed = ((now - start) / 1000) * SPEEDUP;
      setT(Math.min(TYPE_S, elapsed));
      if (elapsed < TYPE_S) frame = requestAnimationFrame(step);
    };
    const io = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          io.disconnect();
          setT(0);
          frame = requestAnimationFrame(step);
        }
      },
      { threshold: 0.5 },
    );
    io.observe(el);
    return () => {
      io.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [reduce, run]);

  const typed = MESSAGE.slice(0, Math.round((Math.min(t, TYPE_S) / TYPE_S) * MESSAGE.length));
  const talking = t < TALK_S;
  const landed = t >= TALK_S + INSERT_S;
  const voiceDone = Math.round(TALK_S + INSERT_S);

  return (
    <section className={styles.race} ref={root}>
      <div className={`container ${styles.head}`}>
        <h2 className="heading">Same message. Typed, then said.</h2>
        <p className={`lede ${styles.lede}`}>
          Most people type about 40 words a minute and speak about 150. Here&rsquo;s a 26-word message both ways,
          played four times faster than real time.
        </p>
      </div>

      <div className={`container ${styles.lanes}`}>
        <figure className={styles.lane}>
          <figcaption className={styles.laneHead}>
            <span>Typing, 40 words a minute</span>
            <span className="tnum">{Math.floor(t)}&nbsp;s</span>
          </figcaption>
          <p className={styles.text}>
            {typed}
            {t < TYPE_S && <span className={styles.caret} />}
          </p>
        </figure>

        <figure className={`${styles.lane} ${styles.voice}`}>
          <figcaption className={styles.laneHead}>
            <span>Talking to Vocelle, 150 words a minute</span>
            <span className="tnum">{landed ? `done in ${voiceDone}` : Math.floor(t)}&nbsp;s</span>
          </figcaption>
          {landed ? (
            <p className={styles.text}>{MESSAGE}</p>
          ) : (
            <div className={styles.listening} data-talking={talking}>
              <span className={styles.pill}>
                {Array.from({ length: 7 }, (_, i) => (
                  <i key={i} style={{ animationDelay: `${i * -0.12}s` }} />
                ))}
              </span>
              <span>{talking ? "Holding the key and talking…" : "Let go. Typing…"}</span>
            </div>
          )}
        </figure>
      </div>

      <div className="container">
        <button type="button" className={styles.again} onClick={() => setRun((r) => r + 1)}>
          Race again
        </button>
      </div>
    </section>
  );
}
