# Changelog

All notable changes to Vocelle are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/codewithdark-git/vocelle/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/codewithdark-git/vocelle/releases/tag/v0.1.0
