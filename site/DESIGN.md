# Vocelle website: art direction

Static HTML, CSS and JavaScript, published as-is to GitHub Pages by
`.github/workflows/pages.yml`. Motion uses GSAP 3.13 (ScrollTrigger,
SplitText, Flip, MotionPath, DrawSVG) and Lenis, self-hosted in `vendor/`
with the fonts, so the page makes no third-party requests except the GitHub
API call for the latest release (static links are the fallback).

## The idea

Every scene explains the product: hold a key, talk, let go, it's typed.
Motion is never decoration; each animation shows something Vocelle does.

| Section | What moves, and what it explains |
|---|---|
| Hero | The headline is spoken onto the page (each word widens, Anybody's `wdth` axis). A laptop's lid opens as you scroll. Hold Right Ctrl (or the button) and the laptop's Right Ctrl glows, the indicator listens, the voice lines swell, and clean text lands in the app. Autoplays until someone tries it. |
| How it works | Pinned, scroll-played: the key goes down, spoken words rise as chips, the fillers fall away, the rest fly into the message field and become the sentence. |
| Speed | Pinned: the same passage typed (about 40 wpm) and spoken (about 150 wpm). |
| Anywhere | Pinned: app windows travel along a curve; the centre one receives the dictation. Depth by light (brightness), not transparency. |
| Privacy | Pinned: the voice reaches this computer, the line to the cloud is cut, airplane mode switches on. |
| Cleanup | Off / Light / Full: removed words collapse out of the sentence (Flip). |
| App | The real screens stack up as you scroll. |
| Numbers | Count up once. Every number has a source in the repo. |
| Download | Sparks stream along a voice line behind the call to action. |
| Footer | The wordmark widens as you arrive. |

Global: Lenis smooth scrolling; sections slide over each other as sheets;
headings rise a line at a time from a mask; buttons lean towards the pointer;
the nav hides while scrolling down.

## Colour

| Token | Hex | Role |
|---|---|---|
| night | `#0F0501` | Hero, Anywhere, App, Download |
| umber | `#2A1206` | How it works, Privacy; ink on paper |
| paper | `#FFF4EA` | Speed, Cleanup, Numbers, FAQ |
| peach | `#FFE6D4` | Headlines on dark |
| ember | `#FF5A1F` | Listening, keys, primary buttons, numbers. Never body text. |
| apricot | `#FFB88A` | Links and highlights on dark |

A fine film grain sits over everything. Ember light (radial glows) only
where something is listening.

## Type

- **Anybody** (variable width and weight): headlines and UI; width is the
  voice.
- **Newsreader**: reading text, at least 18 px.

## Rules

- Respect `prefers-reduced-motion`: no smooth scroll, no pins; every scene
  shows its end state.
- Under 700 px, pinned scenes become simple stacked layouts.
- Don't announce features or plans that haven't shipped.
- When changing a pinned scene, check it with real scroll positions (the
  capture script approach in the session notes), not just a static
  screenshot.
