# Game Time Tracker — Product Requirements Document

## Overview

A lightweight Rust background application for Windows that automatically tracks how long you play games. It runs as a system tray application that starts on boot, detects game processes, and records play sessions. A small native UI window (egui) provides game management.

## Goals

- **Zero-friction tracking** — once configured, the app runs silently and records everything automatically.
- **Reliability** — the app must never miss a game session. If the system crashes, in-progress sessions should be recoverable.
- **Simplicity** — minimal native UI for game management; data is stored in a human-readable/editable format (JSON).
- **Accuracy** — sessions are defined by the game process lifetime (start → stop of the `.exe`).

## Core Features

### 1. Background Process Monitoring

The app runs as a system tray application that starts automatically with Windows.

- **Detection method:** Process polling — periodically scan the Windows process list for configured game executables.
- **Polling interval:** ~5 seconds (configurable).
- **Matching:** By executable filename (e.g., `RocketLeague.exe`).
- **One instance per game:** Only one active tracking session per game at a time.

### 2. Game Configuration

Games are defined in a JSON configuration file (`games.json`):

```json
{
  "games": [
    {
      "id": "rocket-league",
      "name": "Rocket League",
      "executable": "RocketLeague.exe"
    },
    {
      "id": "elden-ring",
      "name": "Elden Ring",
      "executable": "eldenring.exe"
    }
  ]
}
```

Each game entry contains:
| Field        | Type   | Required | Description                            |
|--------------|--------|----------|----------------------------------------|
| `id`         | String | Yes      | Unique identifier (slug)               |
| `name`       | String | Yes      | Display name                           |
| `executable` | String | Yes      | Process name to match (case-insensitive)|

### 3. Session Tracking

Each time a game process is detected and then disappears, a **session** is recorded:

```json
{
  "game_id": "rocket-league",
  "sessions": [
    {
      "start": "2026-02-28T15:00:00Z",
      "end": "2026-02-28T16:30:00Z",
      "duration_secs": 5400
    }
  ],
  "total_play_time_secs": 5400
}
```

**Session data:**
| Field              | Type     | Description                                      |
|--------------------|----------|--------------------------------------------------|
| `start`            | DateTime | When the game process was first detected          |
| `end`              | DateTime | When the game process was no longer detected      |
| `duration_secs`    | u64      | Computed duration in seconds                      |

**Aggregate data per game:**
| Field                  | Type | Description                              |
|------------------------|------|------------------------------------------|
| `total_play_time_secs` | u64  | Sum of all session durations             |

### 4. Crash Recovery

To handle unexpected shutdowns (PC crash, power loss):

- When a session starts, immediately write a **"pending session"** entry with the `start` timestamp and no `end`.
- On each polling cycle, update a `last_seen` timestamp for active sessions.
- On startup, check for pending sessions and close them using `last_seen` as the `end` time.

### 5. Data Storage

- **Format:** JSON files, human-readable and editable.
- **Location:** `%APPDATA%/game-time-tracker/` (e.g., `C:\Users\<user>\AppData\Roaming\game-time-tracker\`).
- **Files:**
  - `games.json` — game configuration (user-editable).
  - `sessions.json` — all recorded session data (user-editable for corrections).
  - `state.json` — runtime state (currently active sessions, last_seen timestamps). Auto-managed.

### 6. System Tray

The app lives in the Windows system tray (notification area) to show it's running.

**Tray icon context menu (right-click):**

| Menu Item           | Action                                                    |
|---------------------|-----------------------------------------------------------|
| **Manage Games**    | Opens the game management window (see §7)                 |
| **Edit Sessions**   | Opens `sessions.json` in the system's default text editor |
| **Open Data Folder**| Opens the `%APPDATA%/game-time-tracker/` folder in Explorer|
| **Quit**            | Stops the tracker and exits                               |

**Tray icon tooltip:** Shows the app name and number of currently tracked (active) games.

### 7. Game Management Window (egui)

A small native window built with `egui` (via `eframe`) that opens from the tray menu. This is the primary way to manage games.

**Features:**

- **Game list** — table showing all configured games with columns:
  - Name
  - Executable
  - Total play time (formatted as `XXh YYm`)
  - Status (🟢 Running / ⚪ Idle)
- **Add game** — button that shows inline fields for name and executable, with a "Save" button.
- **Edit game** — click a game row to edit its name or executable inline.
- **Remove game** — delete button per row with a confirmation prompt.
- **Session history** — expandable section per game showing individual sessions (start, end, duration).

The window can be closed and reopened from the tray menu without affecting the tracker.

### 8. Editing (Manual)

For advanced edits, the JSON files can also be modified directly:

- **Edit session times** — modify `start`/`end` in `sessions.json`; `duration_secs` and `total_play_time_secs` will be recalculated on next load.
- **Edit game metadata** — modify `name`/`executable` in `games.json`.
- **Delete sessions** — remove entries from `sessions.json`.

The tray menu provides quick access to these files via "Edit Sessions" and "Open Data Folder".

### 9. CLI Commands

The app exposes a minimal CLI for setup:

| Command                  | Description                                      |
|--------------------------|--------------------------------------------------|
| `gtt`                    | Start the app (tray icon + background tracker)   |
| `gtt install`            | Enable auto-start on Windows login               |
| `gtt uninstall`          | Disable auto-start on Windows login              |

Game management is done through the UI window, not the CLI.

### 10. Auto-Start on Boot

The app registers itself in the Windows Startup registry (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`) so the tracker starts automatically on login.

- `gtt install` / `gtt uninstall` to enable/disable auto-start.

## Non-Goals (Out of Scope)

- No cloud sync.
- No Steam/Epic/GOG integration.
- No multi-user support.
- No game screenshots or media capture.
- No charts or graphs (simple text/table output only).

## Technical Approach

### Detection: Process Polling

Use the `sysinfo` crate to enumerate running processes every ~5 seconds. Match process names (case-insensitive) against configured game executables.

**Why polling over WMI/ETW:**
- Simpler to implement and debug.
- No elevated permissions required.
- 5-second granularity is more than sufficient for game sessions (typically hours).
- Can be upgraded to event-driven later if needed.

### Data Format: JSON

- Human-readable and editable with any text editor.
- Easy to parse with `serde` + `serde_json`.
- Sufficient for a personal tool tracking dozens of games.

### Architecture

```
┌──────────────┐     ┌──────────────┐     ┌─────────────┐
│  System Tray  │────▶│  Tracker     │────▶│ JSON Store  │
│  + egui UI   │     │  (polling    │     │ (files on   │
│  + CLI       │     │   loop)      │     │  disk)      │
└──────────────┘     └──────────────┘     └─────────────┘
                          │
                    ┌─────┴──────┐
                    │ Process    │
                    │ Monitor    │
                    │ (sysinfo)  │
                    └────────────┘
```

### Key Crates

| Crate           | Purpose                            |
|-----------------|------------------------------------|
| `eframe`/`egui` | Native UI window for game mgmt     |
| `tray-icon`     | System tray icon and context menu  |
| `clap`          | CLI argument parsing               |
| `sysinfo`       | Process enumeration                |
| `serde`         | Serialization/deserialization      |
| `serde_json`    | JSON format                        |
| `chrono`        | Timestamps and duration math       |
| `dirs`          | Platform-specific app data paths   |
| `winreg`        | Windows registry (auto-start)      |
| `log` + `env_logger` | Logging                      |

### Platform

- **Target:** Windows only (`x86_64-pc-windows-msvc`).
- **Minimum Windows version:** Windows 10.
- **Can be cross-compiled** from Linux using `x86_64-pc-windows-gnu` if needed.
