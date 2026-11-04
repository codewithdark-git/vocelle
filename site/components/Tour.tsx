"use client";

// A product film driven by scrolling: the real app screens in a frame, with
// a camera that moves to the part each step talks about.
import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { useRef, useState } from "react";

import styles from "./Tour.module.css";

gsap.registerPlugin(ScrollTrigger, useGSAP);

const base = process.env.NEXT_PUBLIC_BASE_PATH ?? "";

const STEPS = [
  {
    src: "welcome.png",
    alt: "The welcome setup, first step",
    title: "Set up in two minutes",
    text: "The first time you open Vocelle, it checks your microphone and downloads the speech model that suits your computer.",
    origin: "50% 42%",
    scale: 1.28,
  },
  {
    src: "home.png",
    alt: "Home: totals and recent dictations by day",
    title: "See what you’ve said",
    text: "Home shows your dictations by day and how much typing they saved. Copy any of them back with one click.",
    origin: "62% 40%",
    scale: 1.55,
  },
  {
    src: "text.png",
    alt: "The Text page: cleanup levels and a live preview",
    title: "Choose how tidy",
    text: "Cleanup removes “um”, stutters and stray capitals. The preview shows exactly what will be typed.",
    origin: "62% 48%",
    scale: 1.5,
  },
  {
    src: "models.png",
    alt: "The Models page listing six speech models",
    title: "Pick a speed",
    text: "Six models, from 78 MB to 574 MB. Smaller ones type faster; bigger ones make fewer mistakes. All run offline.",
    origin: "62% 34%",
    scale: 1.4,
  },
];

export function Tour() {
  const section = useRef<HTMLElement>(null);
  const [active, setActive] = useState(0);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add("(min-width: 901px) and (prefers-reduced-motion: no-preference)", () => {
        const shots = gsap.utils.toArray<HTMLElement>(".tour-shot");
        gsap.set(shots, { opacity: 0, scale: 1 });
        gsap.set(shots[0], { opacity: 1 });

        const tl = gsap.timeline({
          defaults: { ease: "power2.inOut" },
          scrollTrigger: {
            trigger: section.current,
            start: "top top",
            end: `+=${STEPS.length * 85}%`,
            pin: true,
            scrub: 0.7,
            onUpdate: (self) => setActive(Math.min(STEPS.length - 1, Math.floor(self.progress * STEPS.length))),
          },
        });
        STEPS.forEach((step, i) => {
          const shot = shots[i];
          // Push in on the part this step is about, then hold.
          tl.fromTo(shot, { scale: 1, transformOrigin: step.origin }, { scale: step.scale, duration: 1 });
          tl.to({}, { duration: 0.35 });
          if (i < STEPS.length - 1) {
            // Pull back and cross to the next screen.
            tl.to(shot, { scale: 1, opacity: 0, duration: 0.5 });
            tl.to(shots[i + 1], { opacity: 1, duration: 0.5 }, "<");
          }
        });
        tl.to(".tour-progress", { scaleX: 1, ease: "none", duration: tl.duration() }, 0);
      });
    },
    { scope: section },
  );

  return (
    <section className={styles.tour} id="tour" ref={section}>
      <div className={`container ${styles.grid}`}>
        <div className={styles.copy}>
          <h2 className="heading">The whole app, in four screens.</h2>
          <ol className={styles.steps}>
            {STEPS.map((s, i) => (
              <li key={s.src} data-active={i === active}>
                <h3>{s.title}</h3>
                <p>{s.text}</p>
              </li>
            ))}
          </ol>
          <div className={styles.track} aria-hidden="true">
            <span className="tour-progress" />
          </div>
        </div>

        <div className={styles.frame}>
          {STEPS.map((s) => (
            <figure key={s.src} className={styles.shotWrap}>
              <img
                className={`tour-shot ${styles.shot}`}
                src={`${base}/product/${s.src}`}
                alt={s.alt}
                width={1520}
                height={1280}
                loading="lazy"
              />
              <figcaption className={styles.mobileCaption}>
                <b>{s.title}.</b> {s.text}
              </figcaption>
            </figure>
          ))}
        </div>
      </div>
    </section>
  );
}
