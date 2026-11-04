"use client";

// Privacy shown, not claimed: the voice's whole route is drawn as you scroll,
// microphone to processor to cursor, while the network cable stays unplugged.
import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { useRef, useState } from "react";

import styles from "./AirGap.module.css";

gsap.registerPlugin(ScrollTrigger, useGSAP);

export function AirGap() {
  const section = useRef<HTMLElement>(null);
  const [wifi, setWifi] = useState(true);

  useGSAP(
    () => {
      const mm = gsap.matchMedia();
      mm.add("(min-width: 901px) and (prefers-reduced-motion: no-preference)", () => {
        const tl = gsap.timeline({
          scrollTrigger: { trigger: section.current, start: "top top", end: "+=110%", pin: true, scrub: 0.6 },
        });
        tl.fromTo(".route", { strokeDashoffset: 1 }, { strokeDashoffset: 0, ease: "none", duration: 1 })
          .fromTo(".stop", { opacity: 0.25 }, { opacity: 1, stagger: 0.33, duration: 0.3 }, 0)
          .fromTo(".plug", { x: 0 }, { x: -26, duration: 0.6, ease: "power2.out" }, 0.2)
          .fromTo(".gap", { opacity: 0 }, { opacity: 1, duration: 0.3 }, 0.5);
      });
    },
    { scope: section },
  );

  return (
    <section className={styles.airgap} id="privacy" ref={section}>
      <div className={`container ${styles.grid}`}>
        <div className={styles.copy}>
          <h2 className="heading">Microphone to cursor. Nowhere&nbsp;else.</h2>
          <p className="lede">
            Recognition runs on your own processor with open-source Whisper models. Audio is held in memory for
            the length of a sentence and never written to disk. There&rsquo;s no account, no analytics and no
            telemetry; the only thing Vocelle ever downloads is the model you choose.
          </p>

          <div className={styles.toggle}>
            <button
              type="button"
              role="switch"
              aria-checked={wifi}
              className={styles.switch}
              onClick={() => setWifi((w) => !w)}
            >
              <span className={styles.knob} />
            </button>
            <div>
              <p className={styles.toggleLabel}>Wi-Fi {wifi ? "on" : "off"}</p>
              <p className={styles.toggleText}>
                {wifi
                  ? "Try turning it off."
                  : "Dictation works exactly the same. It never needed the connection."}
              </p>
            </div>
          </div>
        </div>

        <svg className={styles.art} viewBox="0 0 560 420" role="img" aria-label="Your voice travels from the microphone to the processor to the cursor; the network cable is unplugged.">
          {/* The route */}
          <path className={`route ${styles.route}`} pathLength={1} d="M70 120 C 170 120, 190 210, 280 210 S 400 300, 490 300" />

          {/* Microphone */}
          <g className={`stop ${styles.stop}`}>
            <rect x="52" y="78" width="36" height="64" rx="18" />
            <path d="M40 120a30 30 0 0 0 60 0M70 150v20M54 170h32" className={styles.line} />
            <text x="70" y="208" className={styles.label}>Microphone</text>
          </g>

          {/* Processor */}
          <g className={`stop ${styles.stop}`}>
            <rect x="236" y="166" width="88" height="88" rx="10" />
            <rect x="254" y="184" width="52" height="52" rx="4" className={styles.die} />
            <path d="M226 186h10M226 210h10M226 234h10M324 186h10M324 210h10M324 234h10M256 156v10M280 156v10M304 156v10M256 254v10M280 254v10M304 254v10" className={styles.line} />
            <text x="280" y="294" className={styles.label}>Your processor</text>
          </g>

          {/* Cursor in a text line */}
          <g className={`stop ${styles.stop}`}>
            <rect x="440" y="270" width="104" height="60" rx="8" />
            <path d="M456 300h44" className={styles.textLine} />
            <rect x="506" y="286" width="3" height="28" className={styles.cursor} />
            <text x="492" y="356" className={styles.label}>Your cursor</text>
          </g>

          {/* The network, unplugged */}
          <g className={wifi ? styles.netOn : styles.netOff}>
            <path d="M330 40 C 360 40, 380 60, 400 60" className={styles.cable} />
            <g className="plug">
              <rect x="300" y="28" width="30" height="24" rx="4" className={styles.plug} />
              <path d="M288 34v-4M288 50v-4" className={styles.line} />
            </g>
            <text x="478" y="66" className={styles.label}>Internet</text>
            <rect x="400" y="46" width="26" height="28" rx="4" className={styles.socket} />
            <text x="352" y="104" className={`gap ${styles.gapLabel}`}>never connected</text>
          </g>
        </svg>
      </div>
    </section>
  );
}
