# Game Time Tracker

A lightweight Rust background application for Windows that automatically tracks how long you play games.

## How It Works

1. The app runs in the system tray, starting automatically on Windows login.
2. Every few seconds, it checks running processes for configured game executables.
3. When a game starts, a tracking session begins. When the game closes, the session ends and is saved.
4. Right-click the tray icon to manage games, edit sessions, or quit.

## Usage

Just run `gtt.exe` — it starts the system tray app and background tracker.

**Tray menu (right-click the icon):**
- **Manage Games** — opens a window to add, edit, or remove games and view session history
- **Edit Sessions** — opens `sessions.json` in your default text editor
- **Open Data Folder** — opens the data directory in Explorer
- **Quit** — stops tracking and exits

```bash
# Enable auto-start on Windows login
gtt install

# Disable auto-start
gtt uninstall
```

## Configuration

Games are configured in `%APPDATA%/game-time-tracker/games.json`:

```json
{
  "games": [
    {
      "id": "rocket-league",
      "name": "Rocket League",
      "executable": "RocketLeague.exe"
    }
  ]
}
```

## Data

Session data is stored in `%APPDATA%/game-time-tracker/sessions.json`. You can edit this file directly to correct session times or remove entries.

## Building

```bash
# Native Windows build
cargo build --release

# Cross-compile from Linux
cargo build --release --target x86_64-pc-windows-gnu
```

The compiled binary is at `target/release/gtt.exe`.

## Requirements

- Windows 10 or later
- Rust 1.75+ (for building)
