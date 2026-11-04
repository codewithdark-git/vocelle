# Vocelle website: art direction

Plain HTML, CSS and JavaScript in this folder, published as-is to GitHub
Pages by `.github/workflows/pages.yml`. No build step, no framework, no
third-party requests: fonts are self-hosted, and the only network call is to
the GitHub API for the latest release (with static links as a fallback).

## The idea

Vocelle is one gesture: hold a key, talk, let go. The hero shows exactly that,
live. Visitors hold Right Ctrl (or press and hold the key under the screen)
and the scene warms up: an ember light comes on behind the display while the
indicator listens, then the clean text lands in the app. That warmth is the
one bold thing on the page; everything else is calm.

## Colour

| Token | Hex | Role |
|---|---|---|
| night | `#140702` | Top of the hero, the app showcase |
| umber | `#2A1206` | Dark sections; body text on paper |
| paper | `#FFF5EC` | Reading sections |
| peach | `#FFE6D4` | Headlines on dark |
| ember | `#FF5A1F` | Listening, the key, primary buttons, measured numbers. Never body text. |
| apricot | `#FFB88A` | Links and accents on dark |

Dark and paper sections alternate. No decorative gradients beyond the
listening light, and no glass.

## Type

- **Anybody** (variable width): headlines and UI. The headline is "spoken"
  onto the page: it starts condensed and widens to full voice on load.
- **Newsreader**: reading text, at least 18 px, under 70 characters a line.

## Motion

- One page-load moment (the headline widening, the display rising).
- The hold-to-talk demo, which plays by itself until someone tries it.
- One pinned, scroll-driven story: hold, talk, let go.
- The screenshot frame tilts up into place once.
- Nothing else animates on entry. `prefers-reduced-motion` shows every
  scene's end state.

## Words

Plain, specific, measured. Every number on the page has a source in the repo
(see `CHANGELOG.md` and `docs/DEVELOPMENT.md`). Don't announce features or
plans that haven't shipped.
