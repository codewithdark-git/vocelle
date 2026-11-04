"use client";

// Speech becoming text: sentences flow out of a waveform along a curve.
// Scrolling speeds the flow up a little; reduced motion holds it still.
import { useReducedMotion } from "motion/react";
import { useEffect, useRef } from "react";

import styles from "./WordRibbon.module.css";

const LINES =
  "Can we move the review to Thursday at three?   Remind me to call the dentist tomorrow at ten.   Thanks for the notes, I’ll send the new draft tonight.   Ship it once the Linux build is green.   I’m running ten minutes late, start without me.   ";

export function WordRibbon() {
  const reduce = useReducedMotion();
  const path = useRef<SVGTextPathElement>(null);
  const text = useRef<SVGTextElement>(null);

  useEffect(() => {
    if (reduce) return;
    let offset = 0;
    let boost = 0;
    let lastY = scrollY;
    let frame = 0;
    let last = performance.now();
    const half = () => (text.current?.getComputedTextLength() ?? 4000) / 2;

    const tick = (now: number) => {
      const dt = Math.min(64, now - last);
      last = now;
      const dy = Math.abs(scrollY - lastY);
      lastY = scrollY;
      boost = Math.max(boost * 0.92, Math.min(6, dy * 0.08));
      offset -= (0.06 + boost * 0.06) * dt;
      // The text is two copies long; wrapping at half keeps it seamless.
      if (-offset > half()) offset += half();
      path.current?.setAttribute("startOffset", String(offset));
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [reduce]);

  return (
    <div className={styles.ribbon} aria-hidden="true">
      <svg viewBox="0 0 1600 240" preserveAspectRatio="xMidYMid slice" className={styles.svg}>
        <defs>
          <path id="ribbon-path" d="M 120 150 C 380 40, 620 220, 900 120 S 1380 40, 1700 130" />
        </defs>
        <g className={styles.wave}>
          {[18, 34, 52, 34, 18, 28, 12].map((h, i) => (
            <rect key={i} x={12 + i * 14} y={150 - h / 2} width="7" height={h} rx="3.5" style={{ animationDelay: `${i * -0.13}s` }} />
          ))}
        </g>
        <text ref={text} className={styles.text}>
          <textPath ref={path} href="#ribbon-path" startOffset="0">
            {LINES}
            {LINES}
          </textPath>
        </text>
      </svg>
    </div>
  );
}
