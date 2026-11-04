"use client";

import { useState } from "react";

import styles from "./Download.module.css";
import { useRelease } from "./release";

/** A link that always points at the latest Windows installer. */
export function DownloadLink({ className, children }: { className?: string; children: React.ReactNode }) {
  const release = useRelease();
  return (
    <a className={className} href={release.windows.url}>
      {children}
    </a>
  );
}

/** Version and installer size for running text. */
export function ReleaseMeta() {
  const release = useRelease();
  return (
    <>
      Version {release.version}, {release.windows.size}
    </>
  );
}

type Os = "windows" | "linux";

export function Download() {
  const release = useRelease();
  const [os, setOs] = useState<Os>("windows");

  return (
    <section className={styles.download} id="download">
      <div className={`container ${styles.grid}`}>
        <h2 className={`heading ${styles.title}`}>Two minutes from now, you&rsquo;re talking.</h2>

        <div className={styles.panel}>
          <div className={styles.tabs} role="tablist" aria-label="Operating system">
            {(["windows", "linux"] as Os[]).map((value) => (
              <button
                key={value}
                type="button"
                role="tab"
                aria-selected={os === value}
                className={styles.tab}
                onClick={() => setOs(value)}
              >
                {value === "windows" ? "Windows" : "Linux"}
              </button>
            ))}
          </div>

          {os === "windows" ? (
            <div role="tabpanel" className={styles.body}>
              <p className={styles.file}>
                Vocelle_{release.version}_x64-setup.exe <span className="tnum">{release.windows.size}</span>
              </p>
              <a className={`button ${styles.cta}`} href={release.windows.url}>
                Download for Windows
              </a>
              <ol className={styles.steps}>
                <li>
                  Run the installer. It isn&rsquo;t code-signed yet, so Windows may say{" "}
                  <q>Windows protected your PC</q>. Choose <b>More info</b>, then <b>Run anyway</b>.
                </li>
                <li>The welcome setup checks your microphone and downloads the speech model that suits your computer.</li>
                <li>Hold Right Ctrl in any app and talk.</li>
              </ol>
              <p className={styles.small}>Windows 10 (1903 or later) or 11, 64-bit.</p>
            </div>
          ) : (
            <div role="tabpanel" className={styles.body}>
              <p className={styles.file}>
                Vocelle_{release.version}_amd64.deb <span className="tnum">{release.deb.size}</span>
              </p>
              <a className={`button ${styles.cta}`} href={release.deb.url}>
                Download .deb
              </a>
              <p className={styles.alt}>
                Or the{" "}
                <a href={release.appImage.url}>
                  AppImage <span className="tnum">({release.appImage.size})</span>
                </a>{" "}
                for any distribution.
              </p>
              <ol className={styles.steps}>
                <li>
                  Install with <code>sudo apt install ./Vocelle_{release.version}_amd64.deb</code>, or make the
                  AppImage executable and run it.
                </li>
                <li>Log in with an X11 (Xorg) session: Wayland doesn&rsquo;t let apps see a global talk key.</li>
              </ol>
              <p className={styles.small}>Linux support is a preview.</p>
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
