// The real app, staged: the Home screen with footnotes placed on the actual
// interface, then three other screens as captioned figures.
import styles from "./Product.module.css";

const base = process.env.NEXT_PUBLIC_BASE_PATH ?? "";

// Marker positions are percentages of the Home screenshot.
const NOTES = [
  { x: 23.5, y: 18, text: "Your talk key, front and centre. Change it to Caps Lock, an F-key or anything that suits your hands." },
  { x: 23.5, y: 37.5, text: "Words dictated and the time that saved you, compared with typing at 40 words a minute." },
  { x: 23.5, y: 66.5, text: "Every dictation, grouped by day. Copy one back, or turn history off; audio is never kept." },
  { x: 21.5, y: 92, text: "The speech model, and whether it's ready. Six to choose from, from 78 MB to 574 MB." },
];

const SCREENS = [
  { src: "welcome.png", alt: "The welcome setup's first step", caption: "A two-minute setup checks your microphone and picks a model for your computer." },
  { src: "text.png", alt: "The Text page with cleanup levels and a live preview", caption: "Cleanup removes “um”, stutters and stray capitals. You choose how much." },
  { src: "models.png", alt: "The Models page listing six speech models", caption: "Models download once, are checked with SHA-256, and run offline after that." },
];

export function Product() {
  return (
    <section className={styles.product} id="app">
      <div className={`container ${styles.head}`}>
        <h2 className="heading">This is the whole app.</h2>
        <p className={`lede ${styles.lede}`}>
          It lives in the tray and stays out of the way. When you want it, there&rsquo;s one window, and it opens on
          what you&rsquo;ve said.
        </p>
      </div>

      <div className={`container ${styles.stage}`}>
        <figure className={styles.window}>
          <img
            src={`${base}/product/home.png`}
            alt="Vocelle's Home screen: the talk key, totals for words dictated and time saved, and recent dictations by day"
            width={1520}
            height={1280}
          />
          {NOTES.map((n, i) => (
            <span key={i} className={styles.marker} style={{ left: `${n.x}%`, top: `${n.y}%` }} aria-hidden="true">
              {i + 1}
            </span>
          ))}
        </figure>
        <ol className={styles.notes}>
          {NOTES.map((n, i) => (
            <li key={i}>
              <span className={styles.num} aria-hidden="true">
                {i + 1}
              </span>
              {n.text}
            </li>
          ))}
        </ol>
      </div>

      <div className={`container ${styles.more}`}>
        {SCREENS.map((s) => (
          <figure key={s.src} className={styles.figure}>
            <img src={`${base}/product/${s.src}`} alt={s.alt} width={1520} height={1280} loading="lazy" />
            <figcaption>{s.caption}</figcaption>
          </figure>
        ))}
      </div>
    </section>
  );
}
