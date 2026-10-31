<p align="center">
  <img src="assets/logo-animated.svg" alt="Vocelle" width="520">
</p>

<p align="center">
  <strong>Private, offline voice dictation for Windows.</strong><br>
  Hold a key, speak, and your words appear in any app, transcribed on your own PC.
</p>

<p align="center">
  <a href="https://github.com/codewithdark-git/vocelle/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/codewithdark-git/vocelle?label=download&color=5C7CFA"></a>
  <a href="https://github.com/codewithdark-git/vocelle/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/codewithdark-git/vocelle/ci.yml?branch=main&label=CI"></a>
  <img alt="Platform: Windows 10/11" src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D4">
  <a href="LICENSE"><img alt="License: proprietary" src="https://img.shields.io/badge/license-proprietary-lightgrey"></a>
</p>

<p align="center">
  <a href="https://github.com/codewithdark-git/vocelle/releases/latest"><b>Download for Windows</b></a>
  ·
  <a href="#quick-start">Quick start</a>
  ·
  <a href="#troubleshooting">Troubleshooting</a>
  ·
  <a href="CHANGELOG.md">What's new</a>
</p>

---

## Why Vocelle

- **Works offline.** Speech recognition runs on your PC with open-source Whisper models. No account, no internet needed after the model download.
- **Private by design.** Your audio never leaves your computer. No screenshots, no telemetry.
- **Types anywhere.** Word, Chrome, Slack, Teams, VS Code, Notepad: if it has a text cursor, Vocelle can type into it.
- **Stays out of the way.** A small tray app with a floating indicator that never steals focus.

## Features

| | |
|---|---|
| 🎙️ **Hold to talk** | Hold **Right Ctrl**, speak, release. Your words are pasted at the cursor. |
| 🧠 **Local Whisper models** | Six models from Tiny (78 MB) to Large Turbo (574 MB). Vocelle recommends one for your PC. |
| 🤫 **Voice detection** | Silence and background noise aren't typed. |
| 📋 **Clipboard safe** | Your previous clipboard text is put back after each dictation. |
| ⌨️ **Shortcut aware** | Right Ctrl+C and other shortcuts still work; they cancel dictation instead of starting it. |
| 🔒 **Verified downloads** | Models come from the official whisper.cpp repositories and are checked with SHA-256. |

## System requirements

- Windows 10 (version 1903 or later) or Windows 11, 64-bit
- A CPU with AVX2: Intel 4th generation (2013) or newer, AMD Ryzen or newer
- 4 GB RAM (8 GB recommended for the larger models)
- A microphone
- About 200 MB of disk space for the app and a small model; up to 800 MB for the largest

## Quick start

1. **Download** `Vocelle_x.y.z_x64-setup.exe` from the [latest release](https://github.com/codewithdark-git/vocelle/releases/latest) and run it.
   > The installer isn't code-signed yet, so Windows may show *"Windows protected your PC"*. Click **More info → Run anyway**.
2. **Open Settings.** Vocelle starts in the system tray (the purple sound-wave icon). Click it to open Settings.
3. **Download a model.** Under *Speech models*, click **Download** next to the model tagged *Recommended for this PC*. It's ready when Status shows *ready*.
4. **Dictate.** Click into any text box, **hold Right Ctrl**, speak, and **release**. The indicator at the bottom of the screen shows *Listening → Transcribing → Done*, and your text appears at the cursor.

## Choosing a model

| Model | Size | Languages | Best for |
|---|---|---|---|
| Tiny (English) | 78 MB | English | Older or low-power PCs; short phrases |
| **Base (English)** | 148 MB | English | Most laptops; everyday dictation |
| Base (multilingual) | 148 MB | 99 languages | Like Base, for other languages |
| Small (English) | 190 MB | English | Higher accuracy on a recent 4+ core CPU |
| Small (multilingual) | 190 MB | 99 languages | Like Small, for other languages |
| Large Turbo | 574 MB | 99 languages | Best accuracy; 8+ core CPU |

English-only models are faster and more accurate for English. You can download several and switch with **Use** in Settings.

## Configuration

| What | Where |
|---|---|
| Settings file | `%APPDATA%\com.codewithdark.vocelle\settings.json` |
| Downloaded models | `%LOCALAPPDATA%\com.codewithdark.vocelle\models\` |

`settings.json` is managed by the Settings window:

```json
{
  "model": "base.en"
}
```

| Key | Values | Default |
|---|---|---|
| `model` | `tiny.en`, `base.en`, `base`, `small.en-q5_1`, `small-q5_1`, `large-v3-turbo-q5_0` | none until you download a model |

The talk key is **Right Ctrl** in this release; a configurable key is planned.

## Privacy

- Audio is recorded only while you hold the talk key, is processed in memory, and is never saved to disk.
- Transcription runs entirely on your PC. The only network requests Vocelle makes are the model downloads you start in Settings, from `huggingface.co`.
- No accounts, analytics or telemetry.

## Troubleshooting

<details>
<summary><b>Nothing happens when I hold Right Ctrl</b></summary>

- Make sure Vocelle is running (look for the tray icon).
- Open Settings and check that a model shows *ready*.
- Some keyboards and remote-desktop tools remap Right Ctrl. Try on the laptop's built-in keyboard.
</details>

<details>
<summary><b>"Can't type into an app running as administrator"</b></summary>

Windows blocks normal apps from typing into apps that run as administrator. Either run that app normally, or start Vocelle as administrator too.
</details>

<details>
<summary><b>"No speech heard"</b></summary>

Vocelle didn't detect speech. Check that the right microphone is the Windows default (*Settings → System → Sound → Input*) and speak a little closer to it.
</details>

<details>
<summary><b>Transcription is slow</b></summary>

Switch to a smaller model in Settings: *Base (English)* or *Tiny (English)*. Closing CPU-heavy apps also helps.
</details>

<details>
<summary><b>Vocelle closes when loading a model or dictating</b></summary>

Your CPU may not support AVX2 (see [System requirements](#system-requirements)). Support for older CPUs is planned.
</details>

## Uninstall

*Windows Settings → Apps → Installed apps → Vocelle → Uninstall.* To also remove downloaded models and settings, delete the two folders listed under [Configuration](#configuration).

## Roadmap

- **Next:** cleaner text (filler-word removal, punctuation), a personal dictionary, configurable talk key, and dictation history.
- **Later:** optional AI rewriting, spoken commands, signed installer with automatic updates.

See [CHANGELOG.md](CHANGELOG.md) for what's in each release.

---

## Development

<details>
<summary><b>Build from source</b></summary>

**Requirements**

- Rust stable (MSVC toolchain)
- Visual Studio 2022 Build Tools with *Desktop development with C++* (includes CMake and the Windows SDK)
- LLVM, used to generate whisper.cpp bindings: `winget install LLVM.LLVM` (the build looks in `C:\Program Files\LLVM\bin`; override with `LIBCLANG_PATH`)
- Node.js 22+ and npm

**Run**

```bash
npm install
npm run tauri dev
```

**Check** (formatting, lint, tests, type check; the same as CI)

```bash
npm run check
```

**Build the installer**

```bash
npm run tauri build
```

The installer is written to `src-tauri/target/release/bundle/nsis/`. For GPU transcription, build with `--features vulkan` (needs the Vulkan SDK).

**Test transcription against WAV files** (16 kHz mono)

```bash
cd src-tauri
VOCELLE_TEST_MODEL=path/to/ggml-base.en.bin VOCELLE_TEST_WAV="a.wav;b.wav" \
  cargo test transcribe_wav_files -- --ignored --nocapture
```

**Automated UI tests:** a debug build started with `VOCELLE_TEST_INJECTED_KEYS=1` accepts a talk key sent with `SendInput`. Release builds always ignore injected keys.
</details>

<details>
<summary><b>Project layout</b></summary>

```
src/                  Settings and overlay UI (TypeScript + Vite)
src-tauri/src/
  hotkey.rs           Global talk key (low-level keyboard hook)
  audio.rs            Microphone capture, resampled to 16 kHz mono
  asr.rs              whisper.cpp transcription with voice detection
  models.rs           Model catalog and verified downloader
  speech.rs           Settings commands for models
  dictation.rs        Key → record → transcribe → paste pipeline
  insert.rs           Pasting into the focused app
  overlay.rs          Floating status indicator
  settings.rs         settings.json
  tray.rs             Tray icon and menu
assets/               Logo files
docs/RELEASING.md     How releases are versioned and published
```
</details>

## License

Vocelle is proprietary software. You may download and use official releases; the source is published for transparency only. See [LICENSE](LICENSE). Bundled open-source components keep their own licenses.
