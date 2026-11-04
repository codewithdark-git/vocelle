"use client";

// The latest release from GitHub, fetched once and shared by every download
// button. The fallback values are a working release, so links never break.
import { useEffect, useState } from "react";

const REPO = "codewithdark-git/vocelle";

export type Asset = { url: string; size: string };
export type Release = {
  version: string;
  windows: Asset;
  deb: Asset;
  appImage: Asset;
};

const FALLBACK_VERSION = "0.3.2";
const file = (name: string) =>
  `https://github.com/${REPO}/releases/download/v${FALLBACK_VERSION}/${name}`;

export const FALLBACK: Release = {
  version: FALLBACK_VERSION,
  windows: { url: file(`Vocelle_${FALLBACK_VERSION}_x64-setup.exe`), size: "2.1 MB" },
  deb: { url: file(`Vocelle_${FALLBACK_VERSION}_amd64.deb`), size: "3.5 MB" },
  appImage: { url: file(`Vocelle_${FALLBACK_VERSION}_amd64.AppImage`), size: "78 MB" },
};

let pending: Promise<Release> | null = null;

function megabytes(bytes: number): string {
  const mb = bytes / 1024 / 1024;
  return `${mb < 10 ? mb.toFixed(1) : Math.round(mb)} MB`;
}

function load(): Promise<Release> {
  pending ??= fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
    headers: { Accept: "application/vnd.github+json" },
  })
    .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
    .then((release: { tag_name: string; assets: { name: string; size: number; browser_download_url: string }[] }) => {
      const pick = (re: RegExp, fallback: Asset): Asset => {
        const a = release.assets.find((x) => re.test(x.name));
        return a ? { url: a.browser_download_url, size: megabytes(a.size) } : fallback;
      };
      return {
        version: release.tag_name.replace(/^v/, ""),
        windows: pick(/x64-setup\.exe$/i, FALLBACK.windows),
        deb: pick(/amd64\.deb$/i, FALLBACK.deb),
        appImage: pick(/\.AppImage$/i, FALLBACK.appImage),
      };
    })
    .catch(() => FALLBACK);
  return pending;
}

export function useRelease(): Release {
  const [release, setRelease] = useState(FALLBACK);
  useEffect(() => {
    let live = true;
    load().then((r) => live && setRelease(r));
    return () => {
      live = false;
    };
  }, []);
  return release;
}
