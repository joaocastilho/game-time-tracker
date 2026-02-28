# Game Time Tracker — Product Requirements Document

## 1. Executive Summary

Game Time Tracker is a lightweight Rust background application for Windows that automatically tracks how long you play games. It runs as a system tray application that starts on boot, detects game processes via polling, and records play sessions to local JSON files. A small native UI window (egui) provides game management — adding, editing, and removing games — while session data can also be manually edited in the JSON files for advanced corrections.

The core value proposition is **zero-friction game time tracking**: once you configure your games, the app runs silently in the background and records everything automatically — no manual start/stop, no cloud accounts, no overhead.

**MVP Goal:** Deliver a fully functional background tracker with system tray integration, egui game management UI, crash-resilient session recording, and CLI-based auto-start setup.

## 2. Mission

**Mission statement:** Make it effortless to know exactly how much time you spend playing games on your PC.

**Core principles:**

1. **Zero friction** — once configured, the app is invisible and fully automatic.
2. **Reliability** — never miss a session, even through crashes or power losses.
3. **Simplicity** — minimal UI, human-readable data files, no unnecessary complexity.
4. **Transparency** — all data is stored locally in editable JSON; the user owns their data.
5. **Lightweight** — minimal resource usage; the app should be unnoticeable while running.

## 3. Target Users

**Primary persona:** A PC gamer who plays on Windows and wants to track their play time across multiple games without relying on store-specific trackers (Steam, Epic, etc.) or manual logging.

- **Technical comfort:** Comfortable running CLI commands for initial setup. Can edit JSON files if needed.
- **Key needs:**
  - Automatic tracking without having to remember to start/stop timers.
  - A single place to see play time across all games, regardless of launcher.
  - Ability to correct or adjust session data when needed.
- **Pain points:**
  - Steam only tracks Steam games; Epic/GOG have limited tracking.
  - No unified view across launchers.
  - Manual tracking is tedious and easy to forget.

## 4. MVP Scope

### In Scope

**Core Functionality:**
- ✅ Background process polling to detect game executables
- ✅ Automatic session recording (start/end timestamps, duration)
- ✅ Crash recovery via pending session mechanism
- ✅ System tray icon with context menu
- ✅ egui window for game management (add, edit, remove games)
- ✅ Game list view with total play time and running status
- ✅ Session history per game (expandable in UI)

**Technical:**
- ✅ JSON-based data storage (games, sessions, state)
- ✅ Atomic file writes (write to temp, then rename)
- ✅ CLI commands: `gtt`, `gtt install`, `gtt uninstall`
- ✅ Auto-start via Windows Registry
- ✅ Logging via `log` + `env_logger`

**Deployment:**
- ✅ Single `.exe` binary, no installer required
- ✅ Windows 10+ support
- ✅ Cross-compilable from Linux via `x86_64-pc-windows-gnu`

### Out of Scope

- ❌ Cloud sync or remote storage
- ❌ Steam/Epic/GOG API integration
- ❌ Multi-user support
- ❌ Game screenshots or media capture
- ❌ Charts, graphs, or analytics dashboards
- ❌ macOS or Linux support
- ❌ Installer (MSI/NSIS) — distribute as a standalone `.exe`
- ❌ Auto-update mechanism
- ❌ Game cover art or metadata fetching

## 5. User Stories

1. **As a gamer**, I want the tracker to automatically detect when I launch a game, so that I don't have to manually start a timer.

   _Example: I launch `RocketLeague.exe`, and the app immediately starts recording a session._

2. **As a gamer**, I want the tracker to automatically stop recording when I close a game, so that my play time is accurately captured.

   _Example: I close Elden Ring after 2 hours, and a session entry with `start`, `end`, and `duration_secs: 7200` appears in the data._

3. **As a gamer**, I want to see my total play time per game at a glance, so that I can understand how I spend my gaming time.

   _Example: The egui window shows "Rocket League — 42h 15m" and "Elden Ring — 120h 30m" in a table._

4. **As a gamer**, I want the app to recover from crashes gracefully, so that I don't lose session data if my PC shuts down unexpectedly.

   _Example: My PC crashes during a game. On next boot, the app detects the pending session and closes it using the `last_seen` timestamp._

5. **As a gamer**, I want to add and remove games from a simple UI, so that I can configure which games to track without editing files.

   _Example: I click "Add Game", type "Cyberpunk 2077" and "Cyberpunk2077.exe", and click Save._

6. **As a gamer**, I want to manually edit session times in a JSON file, so that I can correct mistakes or add historical data.

   _Example: I open `sessions.json`, change a session's `end` time, and the app recalculates the duration on next load._

7. **As a gamer**, I want the app to start automatically when I log into Windows, so that I never forget to run it.

   _Example: After running `gtt install`, the app launches silently on every Windows login._

8. **As a power user**, I want quick access to the data folder and session file from the tray menu, so that I can make manual edits easily.

   _Example: I right-click the tray icon, click "Edit Sessions", and the file opens in Notepad._

## 6. Core Architecture & Patterns

### High-Level Architecture

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

### Directory Structure

```
game-time-tracker/
├── src/
│   ├── main.rs          # Entry point, CLI, and app bootstrap
│   ├── tray.rs          # System tray icon and context menu
│   ├── ui.rs            # egui game management window
│   ├── tracker.rs       # Main polling loop and session lifecycle
│   ├── process.rs       # Process detection (sysinfo)
│   ├── store.rs         # JSON file read/write operations
│   ├── models.rs        # Data structures (Game, Session, State)
│   └── config.rs        # Configuration and paths
├── docs/
│   └── PRD.md           # This document
├── AGENTS.md            # Agent coding guidelines
├── README.md
├── Cargo.toml
└── Cargo.lock
```

### Key Design Patterns

- **One module per concern** — process detection, storage, tracking logic, CLI, tray, and UI are all separate modules.
- **Thin `main.rs`** — only wires components together; no business logic.
- **Shared state via `Arc<Mutex<>>`** — the tracker and UI communicate through shared state or channels.
- **Atomic file writes** — write to a temp file, then rename, to prevent data corruption.
- **Graceful degradation** — if a JSON file is missing (first run), fall back to defaults; if malformed, log a warning but don't crash.

## 7. Core Features

### 7.1 Background Process Monitoring

The app runs as a system tray application that starts automatically with Windows.

- **Detection method:** Process polling — periodically scan the Windows process list for configured game executables.
- **Polling interval:** ~5 seconds (configurable).
- **Matching:** By executable filename (e.g., `RocketLeague.exe`), case-insensitive.
- **One instance per game:** Only one active tracking session per game at a time.

### 7.2 Game Configuration

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

| Field        | Type   | Required | Description                             |
|--------------|--------|----------|-----------------------------------------|
| `id`         | String | Yes      | Unique identifier (slug)                |
| `name`       | String | Yes      | Display name                            |
| `executable` | String | Yes      | Process name to match (case-insensitive)|

### 7.3 Session Tracking

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

| Field              | Type     | Description                                 |
|--------------------|----------|---------------------------------------------|
| `start`            | DateTime | When the game process was first detected     |
| `end`              | DateTime | When the game process was no longer detected |
| `duration_secs`    | u64      | Computed duration in seconds                 |

**Aggregate data per game:**

| Field                  | Type | Description                  |
|------------------------|------|------------------------------|
| `total_play_time_secs` | u64  | Sum of all session durations |

### 7.4 Crash Recovery

To handle unexpected shutdowns (PC crash, power loss):

- When a session starts, immediately write a **"pending session"** entry with the `start` timestamp and no `end`.
- On each polling cycle, update a `last_seen` timestamp for active sessions.
- On startup, check for pending sessions and close them using `last_seen` as the `end` time.

### 7.5 Data Storage

- **Format:** JSON files, human-readable and editable.
- **Location:** `%APPDATA%/game-time-tracker/` (e.g., `C:\Users\<user>\AppData\Roaming\game-time-tracker\`).
- **Files:**
  - `games.json` — game configuration (user-editable).
  - `sessions.json` — all recorded session data (user-editable for corrections).
  - `state.json` — runtime state (currently active sessions, last_seen timestamps). Auto-managed.

### 7.6 System Tray

The app lives in the Windows system tray (notification area).

**Tray icon context menu (right-click):**

| Menu Item            | Action                                                     |
|----------------------|------------------------------------------------------------|
| **Manage Games**     | Opens the game management window (see §7.7)                |
| **Edit Sessions**    | Opens `sessions.json` in the system's default text editor  |
| **Open Data Folder** | Opens the `%APPDATA%/game-time-tracker/` folder in Explorer|
| **Quit**             | Stops the tracker and exits                                |

**Tray icon tooltip:** Shows the app name and number of currently tracked (active) games.

### 7.7 Game Management Window (egui)

A small native window built with `egui` (via `eframe`) that opens from the tray menu.

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

### 7.8 Manual Editing

For advanced edits, the JSON files can be modified directly:

- **Edit session times** — modify `start`/`end` in `sessions.json`; `duration_secs` and `total_play_time_secs` will be recalculated on next load.
- **Edit game metadata** — modify `name`/`executable` in `games.json`.
- **Delete sessions** — remove entries from `sessions.json`.

The tray menu provides quick access to these files via "Edit Sessions" and "Open Data Folder".

### 7.9 CLI Commands

The app exposes a minimal CLI for setup:

| Command              | Description                                    |
|----------------------|------------------------------------------------|
| `gtt`                | Start the app (tray icon + background tracker) |
| `gtt install`        | Enable auto-start on Windows login             |
| `gtt uninstall`      | Disable auto-start on Windows login            |

Game management is done through the UI window, not the CLI.

### 7.10 Auto-Start on Boot

The app registers itself in the Windows Startup registry (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`) so the tracker starts automatically on login.

- `gtt install` / `gtt uninstall` to enable/disable auto-start.

## 8. Technology Stack

| Crate              | Version | Purpose                           |
|--------------------|---------|-----------------------------------|
| `eframe` / `egui`  | Latest  | Native UI window for game mgmt   |
| `tray-icon`        | Latest  | System tray icon and context menu |
| `clap`             | 4.x     | CLI argument parsing              |
| `sysinfo`          | Latest  | Process enumeration               |
| `serde`            | 1.x     | Serialization/deserialization     |
| `serde_json`       | 1.x     | JSON format                       |
| `chrono`           | 0.4.x   | Timestamps and duration math      |
| `dirs`             | Latest  | Platform-specific app data paths  |
| `winreg`           | Latest  | Windows registry (auto-start)     |
| `log`              | Latest  | Logging facade                    |
| `env_logger`       | Latest  | Logging backend                   |
| `thiserror`        | Latest  | Library-level error types         |
| `anyhow`           | Latest  | Application-level error handling  |

**Platform:**
- **Target:** Windows only (`x86_64-pc-windows-msvc`).
- **Minimum Windows version:** Windows 10.
- **Cross-compilation:** Supported from Linux via `x86_64-pc-windows-gnu`.

## 9. Security & Configuration

### Configuration

- **Data directory:** `%APPDATA%/game-time-tracker/`, resolved via the `dirs` crate. Never hardcoded.
- **Polling interval:** Hardcoded to ~5 seconds in the MVP. Configurable in a future version.
- **Logging level:** Controlled via `RUST_LOG` environment variable (default: `info`).

### Security Scope

**In scope:**
- ✅ Atomic file writes to prevent data corruption.
- ✅ Graceful handling of malformed JSON (log warning, don't crash).
- ✅ No elevated permissions required — runs as a regular user.
- ✅ Registry writes scoped to `HKCU` (current user only).

**Out of scope:**
- ❌ Encryption of data files (they are human-readable by design).
- ❌ Authentication or access control.
- ❌ Network communication of any kind.

### Deployment

- Single `.exe` binary — no installer, no runtime dependencies.
- User downloads the `.exe`, places it somewhere permanent, and runs `gtt install` once.

## 10. Success Criteria

### MVP is successful when

- ✅ The app starts silently in the system tray on boot.
- ✅ It detects configured game processes within ~5 seconds of launch.
- ✅ Sessions are accurately recorded with correct start/end timestamps.
- ✅ Pending sessions are recovered after a crash or unexpected shutdown.
- ✅ Games can be added, edited, and removed from the egui window.
- ✅ Session history is viewable per game in the UI.
- ✅ `gtt install` / `gtt uninstall` correctly manage auto-start.
- ✅ Data files are valid JSON and can be manually edited.
- ✅ The app uses minimal system resources (< 20 MB RAM, < 1% CPU).

### Quality Indicators

- Zero data loss across normal and crash scenarios.
- Clean `cargo clippy -- -D warnings` output.
- All unit tests pass on every commit.

## 11. Implementation Phases

### Phase 1 — Core Tracking Engine

**Goal:** Get the polling loop and session recording working end-to-end.

**Deliverables:**
- ✅ `models.rs` — `Game`, `Session`, `State` structs with serde derives
- ✅ `config.rs` — data directory resolution, file paths
- ✅ `store.rs` — JSON read/write with atomic writes
- ✅ `process.rs` — process detection via `sysinfo`
- ✅ `tracker.rs` — polling loop, session start/stop, crash recovery
- ✅ `main.rs` — basic CLI (`gtt` starts the tracker)

**Validation:** Unit tests for tracker logic with mocked process detection. Manual test: run the tracker, launch a game, close it, verify `sessions.json` has the correct entry.

---

### Phase 2 — System Tray Integration

**Goal:** Make the app run as a proper background application with a tray icon.

**Deliverables:**
- ✅ `tray.rs` — tray icon with context menu (Manage Games, Edit Sessions, Open Data Folder, Quit)
- ✅ Tray tooltip showing active game count
- ✅ "Edit Sessions" opens `sessions.json` in the default editor
- ✅ "Open Data Folder" opens the data directory in Explorer

**Validation:** Manual test: run the app, verify tray icon appears, context menu works, and menu actions behave correctly.

---

### Phase 3 — egui Game Management Window

**Goal:** Provide a graphical interface for managing games.

**Deliverables:**
- ✅ `ui.rs` — egui window with game list, add/edit/remove functionality
- ✅ Game list table with name, executable, total play time, status
- ✅ Session history expandable per game
- ✅ Window opens from tray menu, can be closed without stopping the tracker

**Validation:** Manual test: add a game via UI, verify it appears in `games.json`. Remove a game, verify it's gone. Edit a game's executable, verify the tracker uses the new value.

---

### Phase 4 — Auto-Start & Polish

**Goal:** Enable auto-start and finalize for daily use.

**Deliverables:**
- ✅ `gtt install` / `gtt uninstall` CLI commands for registry-based auto-start
- ✅ Logging throughout all modules
- ✅ Error handling audit — no unwraps in production code
- ✅ README with setup instructions

**Validation:** Manual test: run `gtt install`, reboot, verify the app starts automatically. Run `gtt uninstall`, reboot, verify it doesn't.

## 12. Future Considerations

- **Configurable polling interval** via a settings file or UI option.
- **Game cover art** — fetch or manually assign cover images for a richer UI.
- **Play time statistics** — weekly/monthly breakdowns, charts, and trends.
- **Export** — export data to CSV or other formats.
- **Notifications** — optional alerts after playing for a configurable duration.
- **Multi-executable games** — support games with multiple `.exe` variants.
- **Steam/Epic integration** — auto-discover games from launcher libraries.
- **macOS/Linux support** — extend process detection to other platforms.

## 13. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Game uses a generic `.exe` name** (e.g., `game.exe`) that collides with other apps | False session recordings | Document the limitation; match by full path in a future version |
| **Antivirus flags process scanning** as suspicious behavior | App blocked or quarantined | Use only standard APIs (`sysinfo`); sign the binary if possible |
| **System tray + egui threading** introduces deadlocks or UI freezes | Poor user experience | Keep UI and tracker on separate threads; use channels for communication |
| **JSON data file grows large** over years of use | Slow load times | Keep data structure flat; consider archiving old sessions in a future version |
| **Windows Startup registry** entry gets silently removed by cleanup tools | App stops auto-starting | Document the behavior; consider alternative startup mechanisms |

## 14. Appendix

### Related Documents

- [AGENTS.md](file:///home/joao/code/game-time-tracker/AGENTS.md) — coding guidelines and Rust best practices
- [README.md](file:///home/joao/code/game-time-tracker/README.md) — project overview and setup instructions

### Key Dependencies

- [egui](https://github.com/emilk/egui) — immediate-mode GUI framework
- [tray-icon](https://github.com/tauri-apps/tray-icon) — cross-platform system tray
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) — system/process information
- [clap](https://github.com/clap-rs/clap) — CLI argument parsing

### Detection: Why Polling Over WMI/ETW

- Simpler to implement and debug.
- No elevated permissions required.
- 5-second granularity is more than sufficient for game sessions (typically hours).
- Can be upgraded to event-driven later if needed.

### Data Format: Why JSON

- Human-readable and editable with any text editor.
- Easy to parse with `serde` + `serde_json`.
- Sufficient for a personal tool tracking dozens of games.
