"use client";

// Page-wide details: the compact bar after the hero, headlines rising line
// by line, and a fine grain over everything.
import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { SplitText } from "gsap/SplitText";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";

import styles from "./Craft.module.css";
import { DownloadLink } from "./Download";

gsap.registerPlugin(ScrollTrigger, SplitText, useGSAP);

export function StickyBar() {
  const [shown, setShown] = useState(false);
  useEffect(() => {
    const onScroll = () => setShown(scrollY > innerHeight * 0.85);
    onScroll();
    addEventListener("scroll", onScroll, { passive: true });
    return () => removeEventListener("scroll", onScroll);
  }, []);

  return (
    <AnimatePresence>
      {shown && (
        <motion.div
          className={styles.bar}
          initial={{ y: -80, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          exit={{ y: -80, opacity: 0 }}
          transition={{ duration: 0.35, ease: [0.23, 1, 0.32, 1] }}
        >
          <a href="#top" className={styles.brand} aria-label="Vocelle, back to top">
            <img src={`${process.env.NEXT_PUBLIC_BASE_PATH}/logo-mark.svg`} alt="" width={26} height={26} />
            Vocelle
          </a>
          <nav className={styles.links} aria-label="Sections">
            <a href="#tour">Tour</a>
            <a href="#privacy">Privacy</a>
            <a href="#cost">Cost</a>
            <a href="#specs">Specs</a>
          </nav>
          <DownloadLink className={styles.download}>Download</DownloadLink>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

/** Section headings rise into place, a line at a time, as they arrive. */
export function HeadingReveal() {
  useGSAP(() => {
    const mm = gsap.matchMedia();
    mm.add("(prefers-reduced-motion: no-preference)", () => {
      const splits: SplitText[] = [];
      document.fonts.ready.then(() => {
        gsap.utils.toArray<HTMLElement>("h2.heading").forEach((el) => {
          const split = SplitText.create(el, { type: "lines", mask: "lines", linesClass: "line" });
          splits.push(split);
          gsap.from(split.lines, {
            yPercent: 105,
            duration: 0.9,
            ease: "power3.out",
            stagger: 0.08,
            scrollTrigger: { trigger: el, start: "top 85%", once: true },
          });
        });
        ScrollTrigger.refresh();
      });
      return () => splits.forEach((s) => s.revert());
    });
  });
  return null;
}

export function Grain() {
  return <div className={styles.grain} aria-hidden="true" />;
}
