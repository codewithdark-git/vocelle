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

- Rust stable (MSVC toolchain) and Visual Studio Build Tools with the Windows SDK and CMake
- LLVM, for generating whisper.cpp bindings (`winget install LLVM.LLVM`; the build looks in `C:\Program Files\LLVM\bin`, override with `LIBCLANG_PATH`)
- Node.js 22+ and npm
- WebView2 runtime (included with Windows 11)

```bash
npm install
npm run tauri dev
```

On first run, open Settings from the tray and download a speech model (the one tagged "Recommended for this PC" is a good start). Then hold **Right Ctrl**, speak, and release. Models are stored in `%LOCALAPPDATA%\com.codewithdark.vocelle\models`.

For GPU speech recognition, build with `--features vulkan` (needs the Vulkan SDK).

Check transcription speed and accuracy against WAV files (16 kHz mono):

```bash
cd src-tauri
VOCELLE_TEST_MODEL=path/to/ggml-base.en.bin VOCELLE_TEST_WAV="a.wav;b.wav" \
  cargo test transcribe_wav_files -- --ignored --nocapture
```

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
