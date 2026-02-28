# Game Time Tracker

A lightweight, zero-friction Rust background application for Windows that automatically tracks how long you play your games. No cloud accounts, no overhead, and no manual start/stop required.

## Features

- **Automatic Tracking**: Detects when configured games start and stop, recording play sessions in the background.
- **Zero Friction**: Runs silently in the system tray. Once configured, you never have to think about it.
- **Game Management UI**: A built-in, lightweight native window to easily add, edit, and remove games, as well as view your play session history and total play time.
- **Crash Resilient**: Gracefully recovers pending sessions if your PC shuts down unexpectedly.
- **Local Data**: All data is stored locally in editable JSON files. You own your data.
- **Auto-Start**: Can be configured to launch automatically when you log into Windows.

## Installation

Game Time Tracker is distributed as a single standalone executable, requiring no installer.

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

- **Manage Games**: Opens the game management window where you can:
  - View a list of your tracked games, their running status, and their total play time.
  - Add new games by specifying a display name and the executable filename (e.g., `RocketLeague.exe`).
  - Edit or remove existing games.
  - Expand a game to see its past play sessions.
- **Edit Sessions**: Directly opens your `sessions.json` file in your default text editor for manual corrections.
- **Open Data Folder**: Opens the directory where all tracker data is stored.
- **Quit**: Stops the tracker process and exits the application.

## Data Storage

All configuration and session data is stored in your AppData folder: `%APPDATA%/game-time-tracker/` (e.g., `C:\Users\<user>\AppData\Roaming\game-time-tracker\`).

Files include:
- `games.json`: Your configured games.
- `sessions.json`: Your recorded session history. You can manually edit `start`/`end` times here to correct tracking mistakes.
- `state.json`: Internal tracking state used for crash recovery. 

*Note: All files are human-readable JSON.*

## Building from Source

To build Game Time Tracker yourself, you will need Rust 1.75+ and Windows 10 or later.

```bash
# Clone the repository
git clone <your-repo-url>
cd game-time-tracker

# Build native Windows binary
cargo build --release

# Note: You can also cross-compile from Linux
cargo build --release --target x86_64-pc-windows-gnu
```

The compiled binary will be located at `target/release/gtt.exe`.

## Requirements

- **OS**: Windows 10 or later.
- **Build**: Rust 1.75+ (only required if compiling from source).
