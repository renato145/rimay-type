# rimay-type

> _"Rimay" means "talk" in Quechua._

A Linux speech-to-keyboard application.
While you press a hotkey rimay-type starts to record your microphone and when
you release the key, speech is transcribed and typed into the active
application.

> **Note:** This has only been tested on Ubuntu 22.04 with X11. If it doesn't work on your system, please [open an issue](../../issues).

## Features

- Global hotkey to start/stop recording
- Transcription via Groq's Whisper API
- System tray icon with recording status
- Multiple hotkeys with different settings

## Configuration

Create a configuration file at `~/.config/rimay-type/config.toml`:

```toml
groq_key = "YOUR_KEY_HERE"

[[keys]]
hotkey = "Super+;"
# Required ID of the model to use ("whisper-large-v3-turbo" or "whisper-large-v3").
model = "whisper-large-v3-turbo"
# The language of the input audio. Supplying the input language in ISO-639-1 (i.e. en, tr, es)
# format will improve accuracy and latency.
language = "en"
# Prompt to guide the model's style or specify how to spell unfamiliar words. (limited to 224
# tokens)
# prompt = "Use proper punctuation."

[[keys]]
hotkey = "Super+Shift+;"
model = "whisper-large-v3-turbo"
language = "es"
```

### Options

| Option     | Description                                                                 |
| ---------- | --------------------------------------------------------------------------- |
| `hotkey`   | Key combination (e.g., `"Super+;"`, `"Ctrl+Shift+R"`)                       |
| `model`    | `"whisper-large-v3-turbo"` (faster) or `"whisper-large-v3"` (more accurate) |
| `language` | Optional ISO-639-1 code (e.g., `"en"`, `"es"`, `"tr"`)                      |
| `prompt`   | Optional guidance for transcription style or spelling (max 224 tokens)      |

## Installation

### Prerequisites

```bash
sudo apt install libgtk-3-dev libxdo-dev libappindicator3-dev
```

### Build and install

```bash
cargo install --path .
```

### Setting up as a service

```bash
# 1. Copy the service file:
mkdir -p ~/.config/systemd/user
cp rimay-type.service ~/.config/systemd/user/

# 2. Reload systemd:
systemctl --user daemon-reload

# 3. Enable and start:
systemctl --user enable rimay-type
systemctl --user start rimay-type

# 4. Verify:
systemctl --user status rimay-type
```

To view logs: `journalctl --user -u rimay-type -f`

To stop: `systemctl --user stop rimay-type`

To disable: `systemctl --user disable rimay-type`

## Solveit

If you are interested in how I built this app using the [solveit](https://solve.it.com/), check this dialogs:

- [01_intro](https://share.solve.it.com/d/c97af8c034c8b68d2588910b2d1c1fbe)
- [02_mvp](https://share.solve.it.com/d/4098221672c8f85b8957a515926c34f6)
- [03_system_integration](https://share.solve.it.com/d/745671bc221538b77db1160b78dca6c8)
