// The spec sheet: dense, factual, every number measured or checkable.
import styles from "./Specs.module.css";

const SPEED = [
  ["A 3-second sentence", "Base (English)", "≈ 1 s"],
  ["8 seconds of talking", "Base (English)", "1.7 s"],
  ["25 seconds of talking", "Base (English)", "2.3 s"],
  ["25 seconds of talking", "Tiny (English)", "0.8 s"],
  ["Start-up, model ready", "Base (English)", "1.9 s"],
];

const MODELS = [
  ["Tiny (English)", "78 MB", "English"],
  ["Base (English)", "148 MB", "English"],
  ["Base", "148 MB", "99 languages"],
  ["Small (English)", "190 MB", "English"],
  ["Small", "190 MB", "99 languages"],
  ["Large Turbo", "574 MB", "99 languages"],
];

export function Specs() {
  return (
    <section className={styles.specs} id="specs">
      <div className={`container ${styles.grid}`}>
        <div className={styles.intro}>
          <h2 className="heading">Measured on an old laptop, on battery.</h2>
          <p className={styles.note}>
            An Intel Core i5-7300U from 2017: two cores, running at 0.8&nbsp;GHz because it was unplugged. Times run
            from letting go of the key to the text appearing, with recorded speech and background noise.<sup>1</sup>
          </p>
        </div>

        <div className={styles.tables}>
          <table className={styles.table}>
            <caption>Time from letting go to text</caption>
            <tbody>
              {SPEED.map(([what, model, time]) => (
                <tr key={what + model}>
                  <th scope="row">{what}</th>
                  <td>{model}</td>
                  <td className={`tnum ${styles.num}`}>{time}</td>
                </tr>
              ))}
            </tbody>
          </table>

          <table className={styles.table}>
            <caption>Speech models</caption>
            <tbody>
              {MODELS.map(([name, size, lang]) => (
                <tr key={name}>
                  <th scope="row">{name}</th>
                  <td>{lang}</td>
                  <td className={`tnum ${styles.num}`}>{size}</td>
                </tr>
              ))}
            </tbody>
          </table>

          <dl className={styles.list}>
            <div>
              <dt>Talk keys</dt>
              <dd>
                Right Ctrl (default), Left Ctrl, Right Shift, Caps Lock, Scroll Lock, Pause, Insert, F1–F12. Menu and
                F13–F24 on Windows.
              </dd>
            </div>
            <div>
              <dt>Languages</dt>
              <dd>21 to pick from in Settings, or automatic detection with a multilingual model.</dd>
            </div>
            <div>
              <dt>Kept on disk</dt>
              <dd>Your settings, the text of your last 200 dictations (optional), and running totals. Never audio.</dd>
            </div>
            <div>
              <dt>Network</dt>
              <dd>Only the model downloads you start, from Hugging Face, checked with SHA-256.</dd>
            </div>
            <div>
              <dt>Needs</dt>
              <dd>
                Windows 10 (1903+) or 11, or Linux with X11. A processor with AVX2 (Intel 2013+, AMD Ryzen). 4 GB of
                memory; about 200 MB of disk.
              </dd>
            </div>
          </dl>

          <p className={styles.footnote}>
            <sup>1</sup> Long dictations are transcribed in parts while you&rsquo;re still talking, so only the last
            sentence is left when you let go. Plugged in, the same laptop is about three times faster.
          </p>
        </div>
      </div>
    </section>
  );
}
