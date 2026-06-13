# Kazamo

Voice-to-text for Linux. Local speech recognition with SenseVoice and Paraformer.

![License](https://img.shields.io/badge/license-MIT-blue)
![Platform](https://img.shields.io/badge/platform-Linux%20(Wayland)-brightgreen)

## Features

- **Local speech recognition** — no cloud, no internet required
- **SenseVoice** — multi-language support (Chinese, English, Japanese, Korean, Cantonese)
- **Paraformer** — fast Chinese speech recognition
- **Auto-copy** — transcription results automatically copied to clipboard
- **System tray** — tray icon changes during recording
- **CLI tool** — `kazamo` command for terminal and hotkey integration
- **Dark/Light theme** — follows system preference, with manual override

## Install

### Prerequisites

```bash
# Fedora / RHEL
sudo dnf install -y alsa-lib-devel webkit2gtk4.1-devel cmake gcc-c++ ncat ffmpeg

# Ubuntu / Debian
sudo apt install -y libasound2-dev libwebkit2gtk-4.1-dev cmake g++ ncat ffmpeg
```

### Build

```bash
git clone https://github.com/iamcheyan/kazamo.git
cd kazamo
npm install
cargo build --manifest-path src-tauri/Cargo.toml
```

### Run

```bash
# Start the app
cargo run --manifest-path src-tauri/Cargo.toml

# Or use the CLI
./kazamo start    # Start recording
./kazamo stop     # Stop and transcribe
./kazamo toggle   # Toggle recording
```

## CLI Usage

```
kazamo <command>

Commands:
  toggle       Toggle recording (start or stop + transcribe)
  start        Start recording
  stop         Stop recording and transcribe
  status       Show current status (idle / recording / stopped)
  show         Bring window to front
  help         Show help
```

## Global Hotkey (labwc)

Add to `~/.config/labwc/rc.xml`:

```xml
<keybind key="A-r">
  <action name="Execute">
    <command>kazamo toggle</command>
  </action>
</keybind>
```

## Models

| Model | Language | Size | Auto-download |
|-------|----------|------|---------------|
| SenseVoice | Multi-language | ~480MB | Yes |
| Paraformer | Chinese | ~865MB | Yes |

Models are stored in `~/.cache/chordvoxmini/`.

## Tech Stack

- **Frontend:** React + TypeScript + Vite
- **Backend:** Rust (Tauri v2)
- **Recognition:** SenseVoice.cpp / sherpa-onnx (Paraformer)
- **Audio:** parecord (PipeWire)
- **Window:** Wayland native (no X11 dependency)

## License

MIT
