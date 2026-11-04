import type { NextConfig } from "next";

// Vercel serves the site at the root. The GitHub Pages copy lives under
// /vocelle, so its workflow sets PAGES_BASE_PATH=/vocelle.
const basePath = process.env.PAGES_BASE_PATH ?? "";

const config: NextConfig = {
  output: "export",
  basePath,
  trailingSlash: true,
  images: { unoptimized: true },
  env: { NEXT_PUBLIC_BASE_PATH: basePath },
  // The repository root has its own lockfile (the desktop app); this site is separate.
  turbopack: { root: import.meta.dirname },
};

export default config;
