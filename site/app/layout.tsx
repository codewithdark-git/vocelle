import type { Metadata, Viewport } from "next";
import { Anybody, Newsreader } from "next/font/google";

import "./globals.css";

// Fonts are downloaded at build time and served from this site, so visitors'
// browsers never contact Google.
const anybody = Anybody({
  subsets: ["latin"],
  axes: ["wdth"],
  variable: "--font-anybody",
  display: "swap",
});

const newsreader = Newsreader({
  subsets: ["latin"],
  axes: ["opsz"],
  style: ["normal", "italic"],
  variable: "--font-newsreader",
  display: "swap",
});

const base = process.env.NEXT_PUBLIC_BASE_PATH ?? "";
const site = "https://codewithdark-git.github.io/vocelle/";

export const metadata: Metadata = {
  metadataBase: new URL(site),
  title: "Vocelle: hold a key, talk, and it's typed",
  description:
    "Vocelle types what you say into any app on Windows and Linux. Speech recognition runs on your own computer, so your voice never leaves it. Free, no account.",
  icons: { icon: `${base}/logo-mark.svg` },
  openGraph: {
    title: "Vocelle: hold a key, talk, and it's typed",
    description: "Private voice typing for Windows and Linux. Your voice never leaves your computer.",
    url: site,
    images: [`${base}/og.jpg`],
    type: "website",
  },
  twitter: { card: "summary_large_image" },
};

export const viewport: Viewport = {
  themeColor: "#FF5A1F",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${anybody.variable} ${newsreader.variable}`}>
      <body>{children}</body>
    </html>
  );
}
