"use client";

// Two plain-spoken sections: what it costs, and what it doesn't do yet.
import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { useRef } from "react";

import styles from "./Facts.module.css";

gsap.registerPlugin(ScrollTrigger, useGSAP);

export function Cost() {
  const root = useRef<HTMLElement>(null);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add("(prefers-reduced-motion: no-preference)", () => {
        gsap.from(".cost-bar", {
          scaleX: 0,
          duration: 1.2,
          ease: "power3.out",
          stagger: 0.15,
          scrollTrigger: { trigger: ".cost-bars", start: "top 80%" },
        });
      });
    },
    { scope: root },
  );

  return (
    <section className={styles.cost} id="cost" ref={root}>
      <div className={`container ${styles.costGrid}`}>
        <div>
          <p className={`display tnum ${styles.zero}`}>$0</p>
          <h2 className={`heading ${styles.costTitle}`}>a year, for as much dictation as you like.</h2>
        </div>
        <div className={styles.costSide}>
          <p className="lede">
            No trial, no word limit, no account. Local recognition costs us nothing per word, so there&rsquo;s
            nothing to meter.
          </p>
          <div className={`cost-bars ${styles.bars}`}>
            <div className={styles.barRow}>
              <span>Vocelle</span>
              <span className={`${styles.barTrack}`}>
                <span className={`cost-bar ${styles.bar} ${styles.ours}`} />
              </span>
              <b className="tnum">$0</b>
            </div>
            <div className={styles.barRow}>
              <span>A typical cloud dictation subscription</span>
              <span className={styles.barTrack}>
                <span className={`cost-bar ${styles.bar}`} />
              </span>
              <b className="tnum">$144–180</b>
            </div>
          </div>
          <p className={styles.small}>
            Yearly cost. Cloud plans run around $12–15 a month at the time of writing. A paid Vocelle plan with
            optional AI rewriting is planned; nothing that&rsquo;s free now will move behind it.
          </p>
        </div>
      </div>
    </section>
  );
}

const LIMITS = [
  ["There’s no Mac version yet.", "Vocelle runs on Windows 10 and 11, with a Linux preview."],
  ["On Linux it needs an X11 session.", "Wayland doesn’t let apps see a global talk key. Choose “Xorg” at the login screen."],
  ["The Windows installer isn’t code-signed.", "So Windows warns you once. The source is public and releases are built by GitHub Actions."],
  ["It doesn’t rewrite what you say.", "It types your words, tidied. AI rewriting (“make this formal”) is planned as an option."],
  ["It needs a processor with AVX2.", "That’s Intel from 2013 on, or any AMD Ryzen. Older processors are planned."],
];

export function Limits() {
  return (
    <section className={styles.limits}>
      <div className={`container ${styles.limitsGrid}`}>
        <h2 className="heading">What it doesn&rsquo;t do yet.</h2>
        <ul className={styles.limitList}>
          {LIMITS.map(([title, text]) => (
            <li key={title}>
              <b>{title}</b> {text}
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
