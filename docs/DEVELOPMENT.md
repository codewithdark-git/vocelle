# Developing Vocelle

Vocelle is a [Tauri 2](https://tauri.app/) app: a Rust backend that records,
transcribes and types, and a small TypeScript + Vite frontend for the Settings
window and the floating indicator.

- [Requirements](#requirements)
- [Run, check and build](#run-check-and-build)
- [How the app fits together](#how-the-app-fits-together)
- [Project layout](#project-layout)
- [Testing](#testing)
- [Releasing](#releasing)

## Requirements

### Windows

- Rust stable (MSVC toolchain)
- Visual Studio 2022 Build Tools with *Desktop development with C++* (includes
  CMake and the Windows SDK)
- LLVM, used to generate the whisper.cpp bindings: `winget install LLVM.LLVM`.
  The build looks in `C:\Program Files\LLVM\bin`; set `LIBCLANG_PATH` to use
  another location.
- Node.js 22+ and npm

### Linux (Debian/Ubuntu package names)

```bash
sudo apt install build-essential cmake libclang-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev \
  libasound2-dev libxtst-dev libx11-dev
export LIBCLANG_PATH="$(dirname "$(ls /usr/lib/llvm-*/lib/libclang.so* | head -1)")"
```

## Run, check and build

```bash
npm install
npm run tauri dev        # run with hot reload
npm run check            # formatting, clippy, unit tests, type check (same as CI)
```

Build installers:

```bash
npm run tauri build -- --bundles nsis            # Windows
npm run tauri build -- --bundles deb,appimage    # Linux
```

Output goes to `src-tauri/target/release/bundle/`. For GPU transcription, add
`--features vulkan` (needs the Vulkan SDK).

whisper.cpp is compiled for an AVX2 baseline (see `src-tauri/.cargo/config.toml`),
so a build behaves the same no matter which machine produced it.

## How the app fits together

```mermaid
flowchart TB
    subgraph Backend["Rust backend (src-tauri/src)"]
        hotkey["hotkey/<br/>talk key listener"] -->|Pressed / Released| dictation["dictation.rs<br/>state machine"]
        dictation --> audio["audio.rs<br/>microphone, 16 kHz mono"]
        dictation --> asr["asr.rs<br/>whisper.cpp + voice detection"]
        asr --> cleanup["cleanup.rs<br/>fillers, stutters, casing"]
        cleanup --> insert["insert/<br/>paste and restore clipboard"]
        dictation --> history["history.rs"]
        settings["settings.rs"] -.-> dictation
        models["models.rs<br/>verified downloads"] -.-> asr
    end
    subgraph Frontend["Frontend (src/)"]
        ui["Settings window<br/>main.ts, models.ts, history.ts"]
        overlay["Floating indicator<br/>overlay.ts"]
    end
    ui <-->|commands.rs| settings
    ui <-->|commands.rs| models
    dictation -->|events| overlay
```

A dictation goes through these steps:

1. **Talk key.** `hotkey/` installs a low-level keyboard hook on Windows
   (`WH_KEYBOARD_LL`) or listens through X11 XRecord on Linux. It sends
   `Pressed`, `Released` or `Interrupted` (another key was pressed while holding,
   so the user meant a shortcut such as Right Ctrl+C).
2. **Recording.** `audio.rs` captures the chosen microphone with cpal and
   resamples it to 16 kHz mono in memory. Nothing is written to disk.
3. **Transcription.** `asr.rs` runs Silero voice detection, then Whisper with
   the audio context sized to the clip, which makes short dictations several
   times faster. Dictionary words go into the initial prompt.
4. **Cleanup.** `cleanup.rs` applies replacements, then the chosen cleanup level.
5. **Insertion.** `insert/` takes a snapshot of the whole clipboard, pastes the
   text with Ctrl+V, and puts the snapshot back. On Windows the dictated text is
   marked so it stays out of clipboard history and cloud sync. If the clipboard
   is busy or too large to copy, the text is typed instead.

## Project layout

```
src/                    Settings and overlay UI (TypeScript + Vite)
  main.ts               Tabs, General and Text pages, auto-save
  models.ts, history.ts Models and History pages
  api.ts                Typed backend commands
src-tauri/src/
  hotkey/               Talk key listener (Windows hook, Linux X11)
  insert/               Pasting with clipboard preservation (Windows, Linux)
  audio.rs              Microphone capture, resampled to 16 kHz mono
  asr.rs                whisper.cpp transcription with voice detection
  cleanup.rs            Filler, stutter, spacing and casing cleanup
  dictation.rs          Key → record → transcribe → clean up → insert
  history.rs            Recent dictations
  keys.rs               Talk key catalog
  models.rs             Model catalog and verified downloader
  settings.rs           settings.json with validation
  commands.rs, speech.rs  Commands for the Settings window
  overlay.rs, tray.rs   Floating indicator and tray icon
assets/                 Logo files and README screenshots
site/                   Website (static HTML, deployed to GitHub Pages)
scripts/                Version and release-notes helpers
docs/                   Developer and release documentation
```

## Testing

**Unit tests** run with `npm run check` (or `cargo test` in `src-tauri`). They
cover cleanup, settings validation, the key catalog and model handling.

**Transcription against WAV files** (16 kHz mono):

```bash
cd src-tauri
VOCELLE_TEST_MODEL=path/to/ggml-base.en.bin VOCELLE_TEST_WAV="a.wav;b.wav" \
  cargo test transcribe_wav_files -- --ignored --nocapture
```

**End-to-end test hooks** (debug builds only; release builds ignore them):

- `VOCELLE_TEST_INJECTED_KEYS=1` accepts a talk key sent with `SendInput`
  (Windows), so a script can press it.
- `VOCELLE_TEST_AUDIO=clip.wav` uses a 16 kHz mono 16-bit WAV instead of the
  microphone.

With both set, a script can focus a text box, press the talk key, and check
what arrives. Use a throwaway text box rather than an editor that restores
unsaved documents.

## Releasing

See [RELEASING.md](RELEASING.md).
