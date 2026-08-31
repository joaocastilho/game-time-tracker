# Game Time Tracker
[![CI](https://github.com/joaocastilho/game-time-tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/joaocastilho/game-time-tracker/actions/workflows/ci.yml)

A blazing fast, zero-friction background application for Windows that automatically tracks how long you play your games. Powered by **Tauri 2.0**, it combines a robust Rust backend with a lightweight web frontend. No cloud accounts, no overhead, and no manual start/stop required.

## Features

- **Blazing Fast backend**: Built on Rust for extremely low memory footprint and CPU utilization while running silently in the system tray.
- **Tauri 2.0 Web GUI**: A clean, responsive management interface (Vanilla HTML/JS) that remembers its last window size and uses the full window with internal scrollbars.
- **Automatic Tracking**: Detects configured games via polling (5s, single snapshot per tick) and records play sessions accurately. Non-system `*.exe` dropdown ordered by **last started** (most recent first) lets you see exactly what the tracker sees.
- **Process Finder Dropdown**: Add/Edit game `Executable` uses a filterable dropdown of running non-system applications (filters `WINDIR`, only `*.exe`, deduped). Type to filter, `▼` to see all, `↻` to refresh (no auto-refresh).
- **Game Management**: `Tracked Games` list shows `Running` badge, total play time, session count. `➕ Add Game` is at the **bottom** with extra spacing, behind a toggle (hidden until opened). Executable autocomplete is manual-refresh only.
- **Edit Tracking & Sessions**: `✎ Edit` corrects `Name`/`Executable` (validates 100-char, alphanumeric name, `.exe`, duplicate exe check, `↻` dropdown). Expand `Show Sessions (N) ▼` (hidden behind button) to see history, then `+ Add Session` (also hidden) to add. Add Session uses **Hours/Minutes → auto-fills Start from End (default `now`)**, but `Start`/`End` (`datetime-local` with seconds `step="1"`) remain editable. Edit/delete individual sessions, orphaned sessions listed separately.
- **Full-Window Layout**: `Tracked Games` fills available height; expanded `Session list` (`45vh`) and `Add Session` form (`32vh`) gain internal vertical scrollbars instead of growing the window. Generous gaps (`28-32px`) between header, list, and `Add Game`.
- **Crash Resilient**: Atomic writes (`*.tmp.<pid>-<nanos>`, `fsync`, randomized temp, `create_dir_all` parent, cross-device `tmp2`+`rename`), pending-session recovery with per-session zero-duration fallback, zombie-session close, gap detection (`600s`), pruning (`100` per game), and malformed JSON `warn` + default (no crash).
- **Local Data**: All data stored locally in editable JSON files. You own your data.
- **Auto-Start**: `gtt install` / `gtt uninstall` via `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (quoted path for spaces, preserves `REG_SZ` vs `REG_EXPAND_SZ`, exact PATH match, streaming `md5`).

## Installation

Game Time Tracker is distributed as a single Windows standalone executable (`nsis` bundle, `targets: ["nsis"]`).

1. Download the latest `gtt.exe` from Releases (Windows only, `target/release/gtt.exe`).
2. Place it in a permanent location on your drive.
3. To enable auto-start on Windows login, open a terminal in that folder and run:

```bash
gtt install
```

To disable auto-start later, run:

```bash
gtt uninstall
```

Other OS releases have been cleared – this project is Windows-only.

## Usage

Simply run `gtt.exe` to start the tracker. It will appear in your system tray. The window remembers its last size.

### System Tray Menu (Right-Click)

- **Manage Games**: Opens the management window where you can:
  - View tracked games with `● tracking` / `○ idle`, `Total Play Time`, `Average` and `Last played` (visible in `✎ Edit` as `Total Play Time: … across N session(s)` + `Average / Last played`).
  - Expand `Show Sessions (N)` (hidden behind button) to see history. `+ Add Session` is also hidden – click to reveal the add form.
  - **Add Game** at the **bottom** (centered, `12px 24px`) – click to open the add form (hidden until then). Pick `Executable` from the dropdown (non-system, last-started first, `↻` to refresh).
  - `✎ Edit` a game to correct `Name`/`Executable` (dropdown) and manage its sessions without losing focus (auto-refresh paused while editing).
- **Edit Sessions**: Directly opens your `sessions.json` in your default text editor for manual corrections.
- **Open Data Folder**: Opens the directory where all tracker data is stored.
- **Quit**: Cleanly stops the tracker (`should_stop` + `stop_tx`, `SeqCst`) and exits.

### Add Session

Open a game → `Show Sessions` → `+ Add Session`:
- Enter `Hours`/`Mins` → `Start` auto-calculated as `End - duration` (`End` defaults to now). Or edit `Start`/`End` directly (both `datetime-local` with seconds).
- `Add` validates `End >= Start`, recomputes `duration_secs`, persists via atomic save.

## Data Storage

All configuration and session data is stored in your user configuration folder: `%APPDATA%/game-time-tracker/` (e.g., `C:\Users\<user>\AppData\Roaming\game-time-tracker\` – falls back to `%TEMP%/game-time-tracker` if unwritable).

Files include:
- `games.json`: Your configured games (`id` truncated to 64, `name`/`executable` max 100).
- `sessions.json`: Your recorded session history. You can manually edit `start`/`end` times; `duration_secs` is recomputed on next edit.
- `state.json`: Internal tracking state used for crash recovery (`active_sessions`, `last_seen` with `#[serde(default)]`).

*Note: All files are human-readable JSON. Corrupted JSON is logged as `warn` and defaults are used (no crash, zombie handling deferred to avoid data loss).*

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

The compiled Windows binary will be located at `target/release/gtt.exe` (also `target/x86_64-pc-windows-msvc/release/gtt.exe` when cross-built; other OS targets have been removed).

## Requirements

- **OS**: Windows 10 or later (Requires WebView2, which is pre-installed on Windows 10/11) – Windows-only, other OS releases cleared.
- **Build**: Rust 1.75+ (only required if compiling from source)
