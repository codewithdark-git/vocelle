<p align="center">
  <img src="assets/logo-animated.svg" alt="Vocelle" width="520">
</p>

<p align="center">
  <strong>Private, offline voice dictation for Windows and Linux.</strong><br>
  Hold a key, speak, and your words appear in any app, transcribed on your own computer.
</p>

<p align="center">
  <a href="https://github.com/codewithdark-git/vocelle/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/codewithdark-git/vocelle?label=download&color=5C7CFA"></a>
  <a href="https://github.com/codewithdark-git/vocelle/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/codewithdark-git/vocelle/ci.yml?branch=main&label=CI"></a>
  <img alt="Platforms: Windows 10/11, Linux" src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20%7C%20Linux-0078D4">
  <a href="LICENSE"><img alt="License: proprietary" src="https://img.shields.io/badge/license-proprietary-lightgrey"></a>
</p>

<p align="center">
  <a href="https://github.com/codewithdark-git/vocelle/releases/latest"><b>Download</b></a>
  ·
  <a href="https://codewithdark-git.github.io/vocelle/">Website</a>
  ·
  <a href="#quick-start">Quick start</a>
  ·
  <a href="#using-vocelle">Using Vocelle</a>
  ·
  <a href="#troubleshooting">Troubleshooting</a>
  ·
  <a href="CHANGELOG.md">What's new</a>
</p>

---

## Why Vocelle

- **Works offline.** Speech recognition runs on your computer with open-source Whisper models. No account, no internet needed after the model download.
- **Private by design.** Your audio never leaves your computer. No screenshots, no telemetry.
- **Types anywhere.** Word, Chrome, Slack, Teams, VS Code, Notepad: if it has a text cursor, Vocelle can type into it.
- **Reads like you typed it.** Filler words, stutters and spacing are cleaned up, and your dictionary gets names right.
- **Stays out of the way.** A small tray app with a floating indicator that never steals focus.

## Features

| | |
|---|---|
| 🎙️ **Your talk key** | Hold to talk, or press once to start and again to stop. Right Ctrl by default; pick Caps Lock, an F-key and more. |
| 🧠 **Local Whisper models** | Six models from Tiny (78 MB) to Large Turbo (574 MB). Vocelle recommends one for your computer. |
| ✨ **Text cleanup** | Removes "um" and "uh", fixes stutters, spacing and capitals. Three levels, with a live preview. |
| 📖 **Dictionary and replacements** | Teach Vocelle names and jargon, and set rules like "vocel" → "Vocelle". |
| 🕘 **History** | Your last 200 dictations, text only, stored on your computer. Copy or delete any of them, or turn history off. |
| 📋 **Clipboard safe** | Whatever you had copied (text, images, files, formatting) is put back after each dictation. |
| 🤫 **Voice detection** | Silence and background noise aren't typed. |
| 🌍 **21 languages** | Automatic language detection with multilingual models, or pick one. |
| 🔒 **Verified downloads** | Models come from the official whisper.cpp repositories and are checked with SHA-256. |

## System requirements

| | Windows | Linux (preview) |
|---|---|---|
| OS | Windows 10 (1903+) or 11, 64-bit | x86-64 desktop with an **X11** session (Ubuntu 22.04+, Debian 12+, Fedora, Arch…) |
| CPU | AVX2: Intel 4th gen (2013) or newer, AMD Ryzen or newer | Same |
| Memory | 4 GB (8 GB for the larger models) | Same |
| Disk | ~200 MB with a small model, up to 800 MB with the largest | Same |

And a microphone.

## Quick start

1. **Download** from the [latest release](https://github.com/codewithdark-git/vocelle/releases/latest):
   - **Windows:** `Vocelle_x.y.z_x64-setup.exe`, then run it.
     > The installer isn't code-signed yet, so Windows may show *"Windows protected your PC"*. Click **More info → Run anyway**.
   - **Debian / Ubuntu:** `Vocelle_x.y.z_amd64.deb`, then `sudo apt install ./Vocelle_x.y.z_amd64.deb`.
   - **Other Linux:** `Vocelle_x.y.z_amd64.AppImage`, then `chmod +x Vocelle_*.AppImage` and run it.
2. **Open Settings.** Vocelle starts in the system tray (the purple sound-wave icon). Click it to open Settings.
3. **Download a model.** On the **Models** tab, click **Download** next to the model tagged *Recommended for this PC*. The General tab shows *ready* when it's loaded.
4. **Dictate.** Click into any text box, **hold Right Ctrl**, speak, and **release**. The indicator at the bottom of the screen shows *Listening → Transcribing → Done*, and your text appears at the cursor.

## Using Vocelle

### Talk key and modes

On the **General** tab, choose a key from the list or click **Press a key…** and press the one you want. The change applies immediately.

- **Hold to talk** (default): record while the key is held down. Pressing another key while holding it (for example Right Ctrl+C) cancels, so normal shortcuts keep working.
- **Press to start, press to stop**: hands-free mode for longer dictation. Recordings stop on their own after 5 minutes.

Keys you can use: Right Ctrl, Left Ctrl, Right Shift, Caps Lock, Scroll Lock, Pause, Insert and F1–F12 (plus Menu and F13–F24 on Windows). Letters, digits and Space can't be used, because holding them would type characters. On Windows, a non-modifier talk key such as Caps Lock is reserved for Vocelle, so it won't also toggle caps.

### Text cleanup

On the **Text** tab:

| Level | What it does | Example: *"um so i think the the plan works, uh, for friday"* |
|---|---|---|
| Off | Inserts exactly what the model heard | um so i think the the plan works, uh, for friday |
| Light (default) | Removes "um", "uh" and similar; fixes spacing; capitalizes the first letter | So i think the the plan works for friday |
| Full | Light, plus stutters, sentence capitals and a lone "i" | So I think the plan works for friday |

Type in **Try it** to preview any level. Vocelle also adds a space after each dictation so back-to-back dictations don't run together; you can turn that off.

### Dictionary and replacements

- **Dictionary:** one name or term per line (for example *Kubernetes* or *Dr. Okafor*). These are given to the speech model as spelling hints.
- **Replacements:** whole-word rules applied to every dictation, not case-sensitive. Use them for words the model keeps getting wrong, or for shortcuts such as *"my email"* → *you@example.com*.

### History

The **History** tab lists your last 200 dictations with the time and length of each. **Copy** puts one back on the clipboard; **Delete** or **Clear all** removes them. Turn history off on the General tab. Audio is never saved, only the final text.

## Choosing a model

| Model | Size | Languages | Best for |
|---|---|---|---|
| Tiny (English) | 78 MB | English | Older or low-power computers; short phrases |
| **Base (English)** | 148 MB | English | Most laptops; everyday dictation |
| Base (multilingual) | 148 MB | 99 languages | Like Base, for other languages |
| Small (English) | 190 MB | English | Higher accuracy on a recent 4+ core CPU |
| Small (multilingual) | 190 MB | 99 languages | Like Small, for other languages |
| Large Turbo | 574 MB | 99 languages | Best accuracy; 8+ core CPU |

English-only models are faster and more accurate for English, and always transcribe English. You can download several and switch with **Use** on the Models tab.

## Configuration

Everything is set from the Settings window, which saves automatically. The files live here:

| What | Windows | Linux |
|---|---|---|
| Settings | `%APPDATA%\com.codewithdark.vocelle\settings.json` | `~/.config/com.codewithdark.vocelle/settings.json` |
| History | `%LOCALAPPDATA%\com.codewithdark.vocelle\history.json` | `~/.local/share/com.codewithdark.vocelle/history.json` |
| Models | `%LOCALAPPDATA%\com.codewithdark.vocelle\models\` | `~/.local/share/com.codewithdark.vocelle/models/` |

<details>
<summary><b>settings.json reference</b></summary>

```json
{
  "model": "base.en",
  "talk_key": "ControlRight",
  "talk_mode": "hold",
  "microphone": null,
  "language": "auto",
  "cleanup": "light",
  "trailing_space": true,
  "dictionary": ["Vocelle", "Kubernetes"],
  "replacements": [{ "from": "vocel", "to": "Vocelle" }],
  "history": true
}
```

| Key | Values | Default |
|---|---|---|
| `model` | `tiny.en`, `base.en`, `base`, `small.en-q5_1`, `small-q5_1`, `large-v3-turbo-q5_0` | none until you download a model |
| `talk_key` | A key code: `ControlRight`, `ControlLeft`, `ShiftRight`, `CapsLock`, `ScrollLock`, `Pause`, `Insert`, `ContextMenu`, `F1`–`F24` | `ControlRight` |
| `talk_mode` | `hold`, `toggle` | `hold` |
| `microphone` | A device name from the Settings list, or `null` for the system default | `null` |
| `language` | `auto` or a language code (`en`, `es`, `fr`, `de`, `hi`, `ur`, `zh`, `ja`…) | `auto` |
| `cleanup` | `off`, `light`, `full` | `light` |
| `trailing_space` | `true`, `false` | `true` |
| `dictionary` | List of words and phrases | `[]` |
| `replacements` | List of `{ "from": "…", "to": "…" }` | `[]` |
| `history` | `true`, `false` | `true` |

Missing keys get their defaults, so older settings files keep working. Edit the file only while Vocelle is closed; it's read at start-up.
</details>

## Privacy

- Audio is recorded only while you dictate, is processed in memory, and is never saved to disk.
- Transcription runs entirely on your computer. The only network requests Vocelle makes are the model downloads you start in Settings, from `huggingface.co`.
- History stores text only, on your computer, and can be turned off or cleared.
- On Windows, dictated text is kept out of clipboard history (Win+V) and cloud clipboard sync.
- No accounts, analytics or telemetry.

## Troubleshooting

<details>
<summary><b>Nothing happens when I press the talk key</b></summary>

- Make sure Vocelle is running (look for the tray icon).
- Open Settings and check that the General tab shows the model as *ready*.
- Check which key and mode are set on the General tab.
- Some keyboards and remote-desktop tools remap Right Ctrl. Try Caps Lock or an F-key instead.
</details>

<details>
<summary><b>Linux: the talk key does nothing</b></summary>

Vocelle listens for the talk key through X11. On a Wayland session, desktop security doesn't let apps see global key presses. Log in with an "Xorg" or "X11" session (choose it on the login screen). The `.deb` package installs the needed X11 libraries; for the AppImage, make sure `libxtst6` is installed.
</details>

<details>
<summary><b>"Can't type into an app running as administrator" (Windows)</b></summary>

Windows blocks normal apps from typing into apps that run as administrator. Either run that app normally, or start Vocelle as administrator too.
</details>

<details>
<summary><b>"No speech heard"</b></summary>

Vocelle didn't detect speech. Pick the right microphone on the General tab and speak a little closer to it.
</details>

<details>
<summary><b>A name keeps coming out wrong</b></summary>

Add it to the **Dictionary** on the Text tab. If it still comes out wrong, add a **Replacement** from what Vocelle types to what you want.
</details>

<details>
<summary><b>Transcription is slow</b></summary>

Switch to a smaller model: *Base (English)* or *Tiny (English)*. Closing CPU-heavy apps also helps.
</details>

<details>
<summary><b>Vocelle closes when loading a model or dictating</b></summary>

Your CPU may not support AVX2 (see [System requirements](#system-requirements)). Support for older CPUs is planned.
</details>

## Uninstall

- **Windows:** *Settings → Apps → Installed apps → Vocelle → Uninstall.*
- **Debian / Ubuntu:** `sudo apt remove vocelle`
- **AppImage:** delete the file.

To also remove downloaded models, history and settings, delete the folders listed under [Configuration](#configuration).

## Roadmap

- **Next:** optional AI rewriting (local or with your own API key), spoken commands like "new line", per-app styles.
- **Later:** signed installer with automatic updates, support for CPUs without AVX2, GPU acceleration.

See [CHANGELOG.md](CHANGELOG.md) for what's in each release.

---

## Development

<details>
<summary><b>Build from source</b></summary>

**Windows requirements**

- Rust stable (MSVC toolchain)
- Visual Studio 2022 Build Tools with *Desktop development with C++* (includes CMake and the Windows SDK)
- LLVM, used to generate whisper.cpp bindings: `winget install LLVM.LLVM` (the build looks in `C:\Program Files\LLVM\bin`; override with `LIBCLANG_PATH`)
- Node.js 22+ and npm

**Linux requirements** (Debian/Ubuntu package names)

```bash
sudo apt install build-essential cmake libclang-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev \
  libasound2-dev libxtst-dev libx11-dev
export LIBCLANG_PATH="$(dirname "$(ls /usr/lib/llvm-*/lib/libclang.so* | head -1)")"
```

**Run**

```bash
npm install
npm run tauri dev
```

**Check** (formatting, lint, tests, type check; the same as CI)

```bash
npm run check
```

**Build installers**

```bash
npm run tauri build -- --bundles nsis            # Windows
npm run tauri build -- --bundles deb,appimage    # Linux
```

Output goes to `src-tauri/target/release/bundle/`. For GPU transcription, add `--features vulkan` (needs the Vulkan SDK).

**Test transcription against WAV files** (16 kHz mono)

```bash
cd src-tauri
VOCELLE_TEST_MODEL=path/to/ggml-base.en.bin VOCELLE_TEST_WAV="a.wav;b.wav" \
  cargo test transcribe_wav_files -- --ignored --nocapture
```

**End-to-end test hooks** (debug builds only; release builds ignore them):

- `VOCELLE_TEST_INJECTED_KEYS=1` accepts a talk key sent with `SendInput` (Windows).
- `VOCELLE_TEST_AUDIO=clip.wav` uses a 16 kHz mono 16-bit WAV instead of the microphone.
</details>

<details>
<summary><b>Project layout</b></summary>

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
assets/                 Logo files
site/                   Website (static HTML, deployed to GitHub Pages)
docs/RELEASING.md       How releases are versioned and published
```
</details>

## License

Vocelle is proprietary software. You may download and use official releases; the source is published for transparency only. See [LICENSE](LICENSE). Bundled open-source components keep their own licenses.
