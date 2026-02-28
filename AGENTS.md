# AGENTS.md

This file provides guidance to the agents when working with code in this repository.

## Project Overview

Game Time Tracker is a lightweight Rust background application for Windows that automatically tracks how long you play games. It runs as a system tray application that detects game processes via polling, and records play sessions to local JSON files. It includes a small native UI window (egui) for game management and supports CLI-based auto-start setup via the Windows Registry.

---

## Tech Stack

| Technology | Purpose |
|------------|---------|
| Rust | Core programming language |
| `eframe` / `egui` | Native UI window for game management |
| `tray-icon` | System tray icon and context menu |
| `sysinfo` | Process enumeration and detection |
| `serde` & `serde_json` | Serialization and deserialization of JSON data |
| `chrono` | Timestamps and duration math |
| `dirs` | Platform-specific app data paths |
| `winreg` | Windows registry (auto-start) |
| `clap` | CLI argument parsing |

---

## Commands

```bash
# Development
cargo build

# Build (Native Windows)
cargo build --release

# Build (Cross-compile from Linux)
cargo build --release --target x86_64-pc-windows-gnu

# Test
cargo test

# Lint
cargo clippy -- -D warnings
cargo fmt -- --check
```

---

## Project Structure

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
├── docs/                # Documentation
│   └── PRD.md           # Product Requirements Document
├── README.md            # Setup and overview
└── Cargo.toml           # Rust manifest and dependencies
```

---

## Architecture

The application is built with a modular architecture where each concern is handled by a specific module. The `main.rs` file wires components together without containing business logic. The application uses **shared state via `Arc<Mutex<>>`** allowing the background tracker and the egui UI to communicate through shared state.

High-level flow:
- A `Tracker` runs a polling loop to scan for running processes matching configured executables.
- It records session data using atomic file writes (write temp, rename) to avoid data corruption.
- State, sessions, and game configs are stored locally as JSON in `%APPDATA%/game-time-tracker/`.
- The system tray icon provides quick access to the config folder, session JSON, and the egui Game Management Window.

---

## Code Patterns

### File & Organization
- **One module per concern**: UI, process polling, storage, tray, CLI, models.
- **Thin `main.rs`**: Only wires components; no business logic.

### Error Handling & Data Integrity
- Atomic file writes are used to prevent data corruption.
- Use `anyhow` for application-level error handling and `thiserror` for library-level error types.
- **No unwraps in production code**: Always handle or propagate errors gracefully.
- If a JSON file is missing (e.g., first run), fall back to defaults; if malformed, log a warning but do not crash.

### State Management
- Use `Arc<Mutex<>>` for sharing state between the background tracker and the UI/Tray systems.

---

## Testing

- **Run tests**: `cargo test`
- **Pattern**: Unit tests should cover core tracker logic (with mocked process detection) and data struct serde operations. E2E uses manual validation or isolated integration tests.

---

## Validation

Before committing, ensure code quality and strictly warning-free codebase:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

---

## Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point, CLI handling, bootstrap |
| `src/tracker.rs` | Polling loop, session recording, crash recovery |
| `src/ui.rs` | Game management interface (egui) |
| `src/process.rs` | Detecting processes with `sysinfo` |
| `src/store.rs` | Valid/atomic JSON I/O |
| `docs/PRD.md` | Core source of truth for features and behavior |

---

## Notes

- **Target OS**: Windows 10+ only (requires valid Windows paths/registry handling).
- The JSON configuration and data files are meant to be human-readable and editable, thus must not be encrypted or obfuscated.
- The UI handles the management of tracking rules while the background polling happens via a detached loop.
