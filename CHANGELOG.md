# Changelog

All notable changes to Vocelle are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/codewithdark-git/vocelle/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/codewithdark-git/vocelle/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/codewithdark-git/vocelle/releases/tag/v0.1.0
