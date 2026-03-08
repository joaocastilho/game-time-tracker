# Game Time Tracker
[![CI](https://github.com/joaocastilho/game-time-tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/joaocastilho/game-time-tracker/actions/workflows/ci.yml)

A blazing fast, zero-friction background application for Windows that automatically tracks how long you play your games. Powered by **Tauri 2.0**, it combines a robust Rust backend with a lightweight web frontend. No cloud accounts, no overhead, and no manual start/stop required.

## Features

- **Blazing Fast backend**: Built on Rust for extremely low memory footprint and CPU utilization while running silently in the system tray.
- **Tauri 2.0 Web GUI**: A clean, responsive management interface built using Vanilla HTML/JS styled natively, compiling down to a tiny standalone executable.
- **Automatic Tracking**: Detects when configured games start and stop, recording play sessions accurately in the background without polling overhead.
- **Crash Resilient**: Gracefully recovers pending sessions if your PC shuts down unexpectedly.
- **Local Data**: All data is stored locally in editable JSON files. You own your data.
- **Auto-Start**: Can be configured to launch automatically when you log into Windows via Windows Registry.

## Installation

Game Time Tracker is distributed as a single standalone executable, bundling both the Rust tracker and the web frontend into one frictionless binary.

1. Download the latest `gtt.exe` binary.
2. Place it in a permanent location on your drive.
3. To enable auto-start on Windows login, open a terminal in that folder and run:

```bash
gtt install
```

To disable auto-start later, run:

```bash
gtt uninstall
```

## Usage

Simply run `gtt.exe` to start the tracker. It will appear in your system tray.

### System Tray Menu (Right-Click)

- **Manage Games**: Opens the Tauri web management window where you can:
  - View a list of your tracked games, their running status, and their total play time.
  - Add new games by specifying a display name and the executable filename (e.g., `RocketLeague.exe`).
  - Edit or remove existing games.
- **Edit Sessions**: Directly opens your `sessions.json` file in your default text editor for manual corrections.
- **Open Data Folder**: Opens the directory where all tracker data is stored.
- **Quit**: Stops the tracker process cleanly and exits the application.

## Data Storage

All configuration and session data is stored in your user configuration folder: `%APPDATA%/game-time-tracker/` (e.g., `C:\Users\<user>\AppData\Roaming\game-time-tracker\`).

Files include:
- `games.json`: Your configured games.
- `sessions.json`: Your recorded session history. You can manually edit `start`/`end` times here to correct tracking mistakes.
- `state.json`: Internal tracking state used for crash recovery. 

*Note: All files are human-readable JSON.*

## Building from Source

To build Game Time Tracker yourself, you will need Rust 1.75+ and Windows 10 or later.
Tauri 2.0 automatically bundles the `ui` folder into the final binary during the `cargo build` phase.

```bash
# Clone the repository
git clone https://github.com/joaocastilho/game-time-tracker
cd game-time-tracker

# Build native Windows binary with the Tauri web view
cargo build --release
```

The compiled binary will be located at `target/release/gtt.exe`.

## Requirements

- **OS**: Windows 10 or later (Requires WebView2, which is pre-installed on Windows 10/11)
- **Build**: Rust 1.75+ (only required if compiling from source)
