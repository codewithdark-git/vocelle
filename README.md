# Vocelle

Private, offline voice dictation for Windows. Hold a key, speak, and the text appears in whatever app you're typing in, transcribed on your own PC with open-source Whisper models.

> Status: early development (v0.1 skeleton). Not usable for dictation yet.

## Goals

- **Offline by default**: speech recognition runs locally; no account or internet needed.
- **Private**: no screenshots, no audio leaves the machine unless you turn on a cloud provider with your own API key.
- **Light**: a small Tauri + Rust tray app that stays out of the way.

See [docs/PLAN.md](docs/PLAN.md) for the product plan and milestones.

## Development

Requirements (Windows 10 1903+ / 11):

- Rust stable (MSVC toolchain) and Visual Studio Build Tools with the Windows SDK
- Node.js 22+ and npm
- WebView2 runtime (included with Windows 11)

```bash
npm install
npm run tauri dev
```

Hold **Right Ctrl** to dictate. Until the speech engine lands, the app pastes a placeholder like `[Vocelle test: heard 1.6 s of audio]`.

For automated tests, a debug build started with `VOCELLE_TEST_INJECTED_KEYS=1` accepts a talk key sent with `SendInput` (release builds always ignore injected keys).

Build an installer:

```bash
npm run tauri build
```

## Project layout

```
src/            settings UI (TypeScript + Vite)
src-tauri/      Rust core: tray, windows, and (soon) hotkey, audio, speech engine
docs/           plan and competitor behavior research
```
