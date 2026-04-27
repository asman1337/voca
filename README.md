# VOCA — Voice to Cursor, Always

A local-first, always-on-top floating dictation orb for your desktop. Press a hotkey, speak, and your words appear instantly in whatever window you were using — no cloud, no subscription, no microphone data leaving your machine.

Built with [Tauri v2](https://tauri.app), [React 18](https://react.dev), and [whisper.cpp](https://github.com/ggerganov/whisper.cpp) via [`whisper-rs`](https://github.com/tazz4843/whisper-rs).

---

## Features

- **Floating canvas orb** — frameless, transparent, always on top; drag anywhere
- **Local Whisper inference** — runs entirely offline with your choice of model size
- **Global hotkey** — one keystroke starts and stops recording (default `Ctrl+Shift+V`)
- **Smart text injection** — types directly into the focused window via platform APIs; falls back to clipboard when no input is focused
- **System tray** — show/hide, mute, quit without touching the orb
- **Auto-launch on login** — optional; synced across Windows registry / macOS LaunchAgent / Linux XDG autostart
- **Settings panel** — right-click the orb; configure model, language, input mode, VAD sensitivity, auto-launch
- **In-app model downloader** — browse and download any Whisper GGML model with a live progress bar and SHA-256 verification
- **GPU acceleration** — Metal on Apple Silicon (automatic), CUDA on Windows/Linux (`--features cuda`)

---

## Platform support

| Platform | Dictation injection | GPU |
|---|---|---|
| Windows 10/11 | SendInput (Win32) | CUDA (optional) |
| macOS 12+ (Apple Silicon) | AXUIElement (Accessibility) | Metal (automatic) |
| macOS 12+ (Intel) | AXUIElement | CPU |
| Linux (X11) | xdotool | CUDA (optional) |
| Linux (Wayland) | ydotool + ydotoold | CUDA (optional) |

---

## Prerequisites

| Tool | Version |
|---|---|
| [Rust](https://rustup.rs) | 1.80+ |
| [Node.js](https://nodejs.org) | 20+ |
| [Tauri CLI prerequisites](https://tauri.app/start/prerequisites/) | per platform |

**Linux only:** install the injection tool for your session type:
```sh
# X11
sudo apt install xdotool

# Wayland — also start the daemon
sudo apt install ydotool
sudo systemctl enable --now ydotool   # or: ydotoold &
```

---

## Quick start

```sh
git clone https://github.com/asman1337/voca
cd voca

npm install
npm run tauri dev
```

The orb appears on screen. You need a Whisper model before transcription works — open settings (right-click the orb) and download one, or place a `ggml-*.bin` file in `models/`.

### Download a model manually

```sh
# PowerShell (Windows)
./scripts/download_model.ps1

# Bash (macOS / Linux)
./scripts/download_model.sh
```

Models are stored in the platform data directory (`%APPDATA%\voca\models` on Windows, `~/.local/share/voca/models` on Linux/macOS). The in-app downloader handles this automatically.

---

## Build

```sh
# Debug build
npm run tauri build -- --debug

# Release
npm run tauri build

# Release with CUDA (requires CUDA toolkit in PATH or CUDA_PATH env var)
npm run tauri build -- -- --features cuda
```

---

## Project layout

```
src/               React frontend (TypeScript + Vite)
  components/
    Orb.tsx        Canvas-rendered floating orb (5 states, RAF loop)
    Settings.tsx   Right-click settings panel
src-tauri/         Tauri/Rust backend
  src/
    audio/         CPAL microphone capture + VAD
    stt/           whisper-rs inference engine
    inject/        Platform text injection (Win32 / AXUIElement / xdotool)
    pipeline.rs    State machine: Idle → Listening → Transcribing → Injected
    downloader.rs  Streaming model downloader (reqwest + SHA-256)
    config.rs      TOML config (~/.config/voca/config.toml)
    autolaunch.rs  OS login integration
    hotkey/        Global hotkey daemon
models/            Place GGML model files here (git-ignored)
scripts/           Helper scripts for model download
```

---

## Configuration

Config file lives at:

| Platform | Path |
|---|---|
| Windows | `%APPDATA%\voca\config.toml` |
| macOS | `~/Library/Application Support/voca/config.toml` |
| Linux | `~/.config/voca/config.toml` |

```toml
model_path    = "models/ggml-base.bin"
language      = "en"
hotkey        = "ctrl+shift+v"
vad_threshold = 0.5
input_mode    = "inject"   # or "clipboard"
auto_launch   = false
```

All fields are also editable in the Settings panel (right-click the orb).

---

## Contributing

Pull requests are welcome. Please keep commits focused and include a short description of what changed and why.

---

## License

MIT — see [LICENSE](LICENSE).

> VOCA is built by [Asman Mirza](https://github.com/asman1337). The name stands for **V**oice **O**ver **C**ursor **A**lways.
