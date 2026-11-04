# Vocelle website: art direction

Read this before changing the site. It exists so the site keeps a point of
view instead of drifting back to a template.

## The idea: the held key

Vocelle is one gesture: hold a key, talk, let go. The site is built around
that object. The hero is a physical keycap you can press (or press your real
Right Ctrl). While it's held the page *listens*: the headline's letters
stretch with the voice (Anybody's width axis) and the field warms. That is
the one bold moment. Everything after it is calm, specific and factual.

## Colour

| Token | Hex | Role |
|---|---|---|
| ember | `#FF5A1F` | The hero field: the colour of "listening". Display type only on light grounds (fails as body text). |
| umber | `#2A1206` | Night: the product stage, download, footer. Body text on light grounds. |
| peach | `#FFE6D4` | Paper: reading sections. |
| apricot | `#FFB88A` | Text and accents on umber. |
| rust | `#B8380A` | Secondary text and rules on peach. |
| white | `#FFFFFF` | Only inside product surfaces (windows, fields). |

Colour temperature is the motion system: things warm towards ember while
someone speaks and cool to umber when idle. No gradients as decoration, no
glows, no glass.

## Type

- **Anybody** (variable `wdth` 50–150, `wght`): display and UI. Big sizes,
  tight tracking (−0.04em at display sizes). Width is expressive: condensed
  for dense data, wide for the voice.
- **Newsreader** (variable `opsz`): all reading text, set at proper optical
  sizes, 1.55 line height, under 70 characters a line.
- Tabular numbers for every measurement. Curly quotes, real ellipses (…),
  non-breaking spaces between numbers and units.

## Layout

Left-aligned on an asymmetric 12-column grid. Density changes on purpose: a
dense spec sheet next to a nearly empty statement. No centred hero, no
uniform section padding.

## Motion

- One orchestrated page load (keycap settles, headline widens).
- Pressing the key: immediate (≤120 ms) and physical.
- Two pinned, scroll-scrubbed scenes (the caret through apps, the air-gap).
- Only `transform`, `opacity` and font variations. Ease: `cubic-bezier(0.23, 1, 0.32, 1)`.
- `prefers-reduced-motion`: show every scene's end state; no smooth scrolling.

## Banned (these make a page look generated)

- Centred hero with a badge pill and two buttons.
- Grids of identical rounded cards with icons; emoji or unicode as icons.
- One italic or coloured accent word inside a headline.
- All-caps tracked labels above headings; "A · B · C" meta strings; "→" on buttons.
- Fade-and-slide-up on every section.
- Stat rows of invented numbers. Every number on the site is measured or a fact.
- Generic copy: seamless, effortless, supercharge, unlock, "everything you need".
- Logo walls of apps we don't have permission to show.

## Voice

Plain, specific, a little dry. Claims a reader can check ("Runs on your
processor. Nothing leaves the computer."), not adjectives.
