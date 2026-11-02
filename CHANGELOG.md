# Changelog

All notable changes to Vocelle are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-11-03

Faster dictation, a new look in the Vocelle brand colours, and a guided
first run.

### Added

- **Welcome setup** on first launch: check your microphone, download the
  model recommended for your computer, pick your talk key and try a
  dictation. Skip it any time, or run it again from General → Setup guide.
- **Home page**, shown first: words dictated, time saved compared with
  typing, and your recent dictations grouped by day, with Copy and Delete.
  It replaces the History tab.
- **Transcribing ahead**: while you hold the talk key, finished phrases are
  transcribed in the background, so long dictations appear about as quickly
  as short ones.

### Changed

- **Much faster transcription.** On a 2-core laptop, 3 seconds of speech now
  takes about 0.7 s instead of 2.1 s with Base (English), and about 0.6 s with
  Tiny (English). The first dictation after start-up is no longer slower than
  the rest.
- **Speed-first model advice**: Tiny (English) is now recommended on 2- and
  4-core computers, Base (English) on 6–11 cores and Small (English) above
  that. Your current model isn't changed.
- **New floating indicator** without text: bars that move with your voice
  while listening, a wave of dots while transcribing, then a check mark (or a
  short dash when nothing was heard, or a red mark on errors). It's smaller
  and sits just above the taskbar.
- **Redesigned app** in the Vocelle brand colours from the website (ember
  orange, apricot and umber), with a sidebar, clearer settings rows, switches
  and a before/after cleanup preview. Light and dark mode follow Windows.
- **Styled dropdowns** replace the long system lists. The talk key picker
  shows every key at once as a compact grid (recommended keys first), and
  all pickers work with the keyboard.
- New logo and app icon in the brand colours.
- The talk key responds right away even while the previous dictation is
  still being typed, and dictations are always typed in the order spoken.
- Installers are built with tauri-action 1.0 and the latest GitHub Actions.
- The README has screenshots, an explanation of how dictation works, an
  updating guide and a FAQ. New [developer guide](https://github.com/codewithdark-git/vocelle/blob/main/docs/DEVELOPMENT.md)
  and [security policy](https://github.com/codewithdark-git/vocelle/blob/main/SECURITY.md).

### Removed

- **Toggle mode** (press to start, press again to stop). It was unreliable;
  hold-to-talk is now the only mode. If you used toggle mode, Vocelle switches
  to hold-to-talk with the same key.

### Fixed

- Whisper could repeat a short phrase several times on some recordings.
- Pressing the talk key while a long dictation was still being transcribed
  could be ignored.

### Known limitations

- The installer isn't code-signed yet, so Windows SmartScreen shows a warning.
- Needs a CPU with AVX2 (Intel 2013+ or AMD 2015+).
- Linux: the talk key needs an X11 session (Wayland doesn't allow global key
  listening), and dictation doesn't appear in terminals, which paste with
  Ctrl+Shift+V.

## [0.2.0] - 2026-11-02

Cleaner text, your own talk key, and a first Linux preview.

### Added

- **Text cleanup** with three levels: *Off*, *Light* (removes "um", "uh" and
  similar, fixes spacing) and *Full* (also removes stutters like "the the",
  capitalizes sentences and fixes a lone "i"). Try each level on sample text
  in Settings.
- **Personal dictionary**: names and terms you add are passed to the speech
  model so it spells them your way.
- **Replacements**: whole-word rules such as "vocel" → "Vocelle", applied to
  every dictation.
- **Choose your talk key**: Right Ctrl, Left Ctrl, Right Shift, Caps Lock,
  Scroll Lock, Pause, Insert or F1–F12 (plus Menu and F13–F24 on Windows).
  Click *Press a key…* to set it. Changes apply immediately.
- **Toggle mode**: press once to start and again to stop, for hands-free
  dictation. Recordings stop on their own after 5 minutes.
- **History** of your last 200 dictations (text only, stored on your PC), with
  Copy and Delete. It can be turned off.
- **Microphone picker** and **language picker** (21 languages, or automatic
  detection, for multilingual models).
- **Start at sign-in** option.
- A space is added after each dictation so back-to-back dictations don't run
  together (can be turned off).
- **Linux preview** (`.deb` and AppImage) for X11 desktops.

### Changed

- Settings is reorganized into General, Text, Models and History tabs, and
  saves automatically.
- Your whole clipboard is now preserved: images, files and formatted text
  survive a dictation, not just plain text.
- Dictated text is kept out of Windows clipboard history (Win+V) and cloud
  clipboard sync.
- Non-modifier talk keys (Caps Lock, F-keys) no longer reach other apps while
  they're set as the talk key, so Caps Lock doesn't also toggle caps.

### Fixed

- The Settings window could show "state not managed" if it opened before the
  app finished starting.

### Known limitations

- The installer isn't code-signed yet, so Windows SmartScreen shows a warning.
- Needs a CPU with AVX2 (Intel 2013+ or AMD 2015+).
- Linux: the talk key needs an X11 session (Wayland doesn't allow global key
  listening), and Caps Lock as the talk key also toggles caps.

## [0.1.0] - 2026-10-31

First public preview: hold a key, speak, and your words are typed into any app,
transcribed entirely on your PC.

### Added

- Hold **Right Ctrl** to dictate into any app. Pressing another key while
  holding it (for example Right Ctrl+C) cancels, so normal shortcuts still work.
- Offline speech recognition with open-source Whisper models (whisper.cpp),
  with Silero voice detection so silence isn't typed.
- Model manager in Settings: six models from Tiny to Large Turbo, a
  recommendation based on your CPU, resumable downloads, and SHA-256 checks.
- Floating status pill (Listening, Transcribing, Done) that never steals focus
  from the app you're typing in.
- Text is pasted at the cursor, and your previous clipboard text is restored.
- A clear message when the target app runs as administrator, which Windows
  blocks for security.
- Tray icon with Settings and Quit; closing Settings keeps Vocelle running.
  Starting Vocelle again reopens Settings instead of launching a second copy.

### Known limitations

- The installer isn't code-signed yet, so Windows SmartScreen shows a warning.
- Needs a CPU with AVX2 (Intel 2013+ or AMD 2015+).
- Only text is restored to the clipboard; images or files copied before
  dictating are cleared.
- The talk key can't be changed yet.

[Unreleased]: https://github.com/codewithdark-git/vocelle/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/codewithdark-git/vocelle/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/codewithdark-git/vocelle/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/codewithdark-git/vocelle/releases/tag/v0.1.0
