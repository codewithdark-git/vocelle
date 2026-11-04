"use client";

// The first screen: what Vocelle is, in one line, next to Vocelle working.
import { animate } from "motion";
import { useReducedMotion } from "motion/react";
import { useEffect, useRef } from "react";

import { DemoStage } from "./DemoStage";
import { DownloadLink, ReleaseMeta } from "./Download";
import styles from "./Hero.module.css";

export function Hero() {
  const reduce = useReducedMotion();
  const headline = useRef<HTMLHeadingElement>(null);

  // The one orchestrated page-load moment: the headline opens up from narrow.
  useEffect(() => {
    const el = headline.current;
    if (!el || reduce) return;
    const controls = animate(55, 100, {
      duration: 1.2,
      ease: [0.23, 1, 0.32, 1],
      onUpdate: (v) => el.style.setProperty("--w", String(v)),
    });
    return () => controls.stop();
  }, [reduce]);

  return (
    <section className={styles.hero} id="top">
      <header className={`container ${styles.nav}`}>
        <a className={styles.brand} href="#top" aria-label="Vocelle, back to top">
          <img src={`${process.env.NEXT_PUBLIC_BASE_PATH}/logo-mark.svg`} alt="" width={30} height={30} />
          Vocelle
        </a>
        <nav className={styles.links} aria-label="Sections">
          <a href="#tour">Tour</a>
          <a href="#privacy">Privacy</a>
          <a href="#cost">Cost</a>
          <a href="#specs">Specs</a>
        </nav>
        <DownloadLink className={styles.navButton}>Download</DownloadLink>
      </header>

      <div className={`container ${styles.grid}`}>
        <div className={styles.copy}>
          <h1 ref={headline} className={`display ${styles.headline}`}>
            Hold the key{" "}
            <br />
            and talk.
          </h1>
          <p className={`lede ${styles.lede}`}>
            Voice typing that runs on your own computer. Free, works offline, and nothing you say leaves the
            machine.
          </p>
          <div className={styles.actions}>
            <DownloadLink className="button">Download for Windows</DownloadLink>
            <a className={styles.secondary} href="https://github.com/codewithdark-git/vocelle/releases/latest">
              Linux packages
            </a>
          </div>
          <p className={styles.meta}>
            <ReleaseMeta />. No account.
          </p>
        </div>

        <div className={styles.stage}>
          <DemoStage />
          <p className={styles.stageNote}>
            A real dictation, start to finish. Hold Right Ctrl on your keyboard to try it.
          </p>
        </div>
      </div>
    </section>
  );
}
