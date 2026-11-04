"use client";

// One caret, four apps. Pinned on wide screens: scrolling slides the strip
// sideways and types each line as its surface reaches the middle.
import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { useRef } from "react";

import styles from "./Anywhere.module.css";

gsap.registerPlugin(ScrollTrigger, useGSAP);

const SURFACES = [
  {
    kind: "terminal",
    title: "Terminal",
    before: '$ git commit -m "',
    line: "Restore the clipboard after pasting on Linux",
    after: '"',
  },
  {
    kind: "mail",
    title: "Re: Contract draft",
    before: "",
    line: "Hi Priya, the contract looks good. I've left two comments on page four, the rest is ready to sign.",
    after: "",
  },
  {
    kind: "code",
    title: "download.rs",
    before: "// ",
    line: "Retry once if the checksum doesn't match, then tell the user what went wrong.",
    after: "",
  },
  {
    kind: "chat",
    title: "# launch",
    before: "",
    line: "It's out. Thanks for all the testing this week, the Linux build was the hard part.",
    after: "",
  },
] as const;

export function Anywhere() {
  const section = useRef<HTMLElement>(null);
  const track = useRef<HTMLDivElement>(null);
  const lines = useRef<(HTMLSpanElement | null)[]>([]);
  const surfaces = useRef<(HTMLElement | null)[]>([]);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add("(min-width: 901px) and (prefers-reduced-motion: no-preference)", () => {
        const el = track.current!;
        const distance = () => el.scrollWidth - el.clientWidth;
        // Lines start empty and type as the strip moves.
        lines.current.forEach((l) => l && (l.textContent = ""));
        ScrollTrigger.create({
          trigger: section.current,
          start: "top top",
          end: () => `+=${distance() + window.innerHeight * 0.8}`,
          pin: true,
          scrub: 0.6,
          invalidateOnRefresh: true,
          onUpdate: (self) => {
            const p = self.progress;
            gsap.set(el, { x: -distance() * Math.min(1, p * 1.08) });
            // One caret: it sits in whichever surface is being typed into.
            const active = Math.min(SURFACES.length - 1, Math.floor(p * SURFACES.length));
            surfaces.current.forEach((f, i) => f?.toggleAttribute("data-active", i === active));
            SURFACES.forEach((s, i) => {
              // Each surface types during its own slice of the scroll.
              const start = i / SURFACES.length;
              const local = Math.min(1, Math.max(0, (p - start) * SURFACES.length * 1.3));
              const span = lines.current[i];
              if (span) span.textContent = s.line.slice(0, Math.round(local * s.line.length));
            });
          },
        });
        return () => lines.current.forEach((l, i) => l && (l.textContent = SURFACES[i].line));
      });
    },
    { scope: section },
  );

  return (
    <section className={styles.anywhere} id="anywhere" ref={section}>
      <div className={`container ${styles.head}`}>
        <h2 className="heading">It types wherever your cursor&nbsp;is.</h2>
        <p className={`lede ${styles.lede}`}>
          Vocelle pastes into the window you&rsquo;re in, then puts back whatever you had copied. If an app has a
          text cursor and accepts paste, it works there.
        </p>
      </div>

      <div className={styles.track} ref={track}>
        {SURFACES.map((s, i) => (
          <figure
            key={s.kind}
            ref={(n) => void (surfaces.current[i] = n)}
            className={`${styles.surface} ${styles[s.kind]}`}
            data-active={i === 0 ? "" : undefined}
          >
            <figcaption className={styles.bar}>
              <span className={styles.dots} aria-hidden="true" />
              {s.title}
            </figcaption>
            <div className={styles.content}>
              {s.kind === "mail" && <p className={styles.to}>To: Priya Raman</p>}
              {s.kind === "chat" && <p className={styles.author}>You</p>}
              <p className={styles.text}>
                {s.before}
                <span ref={(n) => void (lines.current[i] = n)}>{s.line}</span>
                <span className={styles.caret} aria-hidden="true" />
                {s.after}
              </p>
            </div>
          </figure>
        ))}
      </div>
    </section>
  );
}
