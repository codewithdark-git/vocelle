// Questions and the footer.
import styles from "./Closing.module.css";

const QUESTIONS = [
  ["Why is it free?", "Recognition runs on your computer, so a dictation costs us nothing to serve. There's no server bill to pass on. A paid plan will add optional extras, like AI rewriting, for people who want them."],
  ["Does it work on Linux?", "Yes, as a preview: a .deb for Debian and Ubuntu and an AppImage for other distributions. The talk key needs an X11 session; Wayland doesn't let apps see global keys."],
  ["Does anything I say leave my computer?", "No. The microphone, the speech model and the text all stay on your machine. You can check: unplug the network and keep dictating."],
  ["Will it understand my accent?", "Whisper was trained on a wide range of accents. If a name keeps coming out wrong, add it to your dictionary, or set a replacement from what Vocelle hears to what you meant."],
  ["Why does Windows warn me about the installer?", "The installer isn't code-signed yet, which costs a certificate we haven't bought. The source is public, and every release is built by GitHub Actions from that source."],
  ["Is there a Mac version?", "Not yet. Windows 10 and 11 are supported, and Linux is in preview."],
];

export function Closing() {
  return (
    <>
      <section className={styles.faq}>
        <div className={`container ${styles.grid}`}>
          <h2 className="heading">Questions people ask first.</h2>
          <div className={styles.list}>
            {QUESTIONS.map(([q, a]) => (
              <details key={q} className={styles.item}>
                <summary>{q}</summary>
                <p>{a}</p>
              </details>
            ))}
          </div>
        </div>
      </section>

      <footer className={styles.footer}>
        <div className={`container ${styles.footerInner}`}>
          <p className={styles.sign}>Vocelle. Made for people who think faster than they type.</p>
          <nav aria-label="Project">
            <a href="https://github.com/codewithdark-git/vocelle">Source</a>
            <a href="https://github.com/codewithdark-git/vocelle/blob/main/CHANGELOG.md">Changelog</a>
            <a href="https://github.com/codewithdark-git/vocelle/releases">Releases</a>
            <a href="https://github.com/codewithdark-git/vocelle/blob/main/SECURITY.md">Security</a>
          </nav>
        </div>
      </footer>
    </>
  );
}
