# Feature: Phase 1 — Core Tracking Engine

## Feature Description
Establish the foundational tracking engine of the Game Time Tracker application. This involves setting up the primary polling loop that detects when configured game processes are running, correctly tracking session start and end times, and safely preserving state into local JSON files with crash recovery support.

## User Story
As a gamer
I want the tracker to automatically detect when I launch or close a game and record these sessions
So that my play time is accurately captured and recovered even if the PC crashes.

## Problem Statement
We need a robust background process that can monitor Windows processes seamlessly with minimal resource usage, accurately measure game session durations, and atomically save this data so it's immune to data corruption during unexpected power losses.

## Solution Statement
Implement a modular Rust backend featuring a `sysinfo`-based process monitor, a detached polling loop running every 5 seconds, and a JSON storage engine relying on atomic writes (write to temp and rename). 

## Feature Metadata
**Feature Type**: New Capability
**Estimated Complexity**: Medium
**Primary Systems Affected**: Core Tracker, Storage, Process Polling
**Dependencies**: `serde`, `serde_json`, `chrono`, `dirs`, `sysinfo`, `clap`, `log`, `env_logger`, `anyhow`, `thiserror`

---

## CONTEXT REFERENCES

### Patterns to Follow

**Naming Conventions:**
- Snake case for modules, files, and functions.
- CamelCase for models (`Game`, `Session`, `State`).

**Error Handling:**
- Use `anyhow::Result` for application-level errors (e.g., in `main` or top-level functions).
- Use `thiserror` for specialized errors (e.g., storage layer `StoreError`).
- **No unwraps**: Return Results and log warnings. If a configuration is missing, fall back to defaults where appropriate.

**Storage Pattern:**
- **Atomic Writes**: Always write data to a temporary file (e.g., `sessions.json.tmp`) in the same directory, then rename it to the target file (`sessions.json`). This ensures no data corruption on power loss.

---

## IMPLEMENTATION PLAN

### Phase 1: Foundation
**Tasks:**
- Initialize the cargo project and add all required dependencies.
- Create data directory resolution logic (`config.rs`).
- Implement the core struct types (`models.rs`).

### Phase 2: Core Implementation
**Tasks:**
- Implement the atomic file store logic (`store.rs`).
- Implement process polling utility wrapping `sysinfo` (`process.rs`).

### Phase 3: Integration
**Tasks:**
- Implement the main tracking engine, polling loop, and crash recovery mechanism (`tracker.rs`).
- Wire the CLI parsing to bootstrap and run the tracker in `main.rs`.

### Phase 4: Testing & Validation
**Tasks:**
- Provide unit tests mocking process detection inside `tracker.rs`.
- Ensure everything compiles with `cargo build`.
- Enforce strict clippy constraints.

---

## STEP-BY-STEP TASKS

### CREATE Cargo.toml
- **IMPLEMENT**: Initialize cargo package using `cargo init`. Add dependencies: `serde` (with derive), `serde_json`, `chrono` (with serde), `dirs`, `sysinfo`, `clap` (with derive), `log`, `env_logger`, `anyhow`, `thiserror`.
- **VALIDATE**: `cargo metadata --format-version 1`

### CREATE src/models.rs
- **IMPLEMENT**: Define `Game` (id, name, executable), `Session` (start, end (Option), duration_secs), `State` (active_sessions map of game_id to Session, and last_seen).
- **IMPORTS**: `serde::{Serialize, Deserialize}`, `chrono::{DateTime, Utc}`.

### CREATE src/config.rs
- **IMPLEMENT**: Add function `data_dir() -> PathBuf` that resolves `%APPDATA%/game-time-tracker/`. Create the directory if it doesn't exist.
- **IMPORTS**: `dirs::config_dir`.

### CREATE src/store.rs
- **IMPLEMENT**: Add generic functions to load/save models `T: Serialize + Deserialize`. `save<T>` must serialize to a `.tmp` file in the same directory, then use `std::fs::rename` to atomically replace the actual file. Add fallback logic if file is missing.
- **GOTCHA**: Ensure the serialization uses pretty printing since files must be human-readable.

### CREATE src/process.rs
- **IMPLEMENT**: Create `ProcessMonitor` struct wrapping `sysinfo::System`. Add a method `is_running(&mut self, executable_name: &str) -> bool` which refreshes processes and performs a case-insensitive check on process names.

### CREATE src/tracker.rs
- **IMPLEMENT**: 
    - Include the main `AppTracker` struct containing `store` interactions and `ProcessMonitor`.
    - `run()` loops every 5 seconds.
    - Check all games in `games.json`. Check if they are running. Start sessions, stop sessions, update `last_seen`.
    - Implement `recover_pending_sessions()` running once at start up. It checks if there are unclosed sessions in `state.json`, ends them using `last_seen`, and saves to `sessions.json`.
- **IMPORTS**: `std::thread::sleep`, `std::time::Duration`, etc.

### CREATE src/main.rs
- **IMPLEMENT**: Define the `clap::Parser` with `command` enum. Run `env_logger::init()`. Match on default run (tracker start) and handle it by invoking the tracker polling loop. Stub out `install`/`uninstall` commands.
- **VALIDATE**: `cargo check`
- **VALIDATE**: `cargo clippy -- -D warnings`
- **VALIDATE**: `cargo test`

---

## TESTING STRATEGY

### Unit Tests
Implement tests within `src/tracker.rs`.
- Extract the tracking logic away from the heavy file I/O if possible, or use a `tempdir` for `data_dir` to test session recording logic and crash recovery correctly.

### Edge Cases
- State file has a pending session but process is no longer running at startup -> handled by crash recovery.
- Starting tracking when data files do not yet exist -> should initialize cleanly.

---

## VALIDATION COMMANDS

### Level 1: Syntax & Style
`cargo fmt -- --check`
`cargo clippy -- -D warnings`

### Level 2: Unit Tests
`cargo mod-mock and tests` (execute unit tests directly via `cargo test`)

### Level 4: Manual Validation
Run `cargo run` and manually launch a target app like Notepad (mapped in `games.json`). Wait 5s, close it, and inspect `%APPDATA%/game-time-tracker/sessions.json`.

---

## ACCEPTANCE CRITERIA

- [ ] Feature implements all specified functionality
- [ ] All validation commands pass with zero errors
- [ ] Code follows project conventions and patterns (no unwraps)
- [ ] Proper error handling built-in with atomic writes.

---

## COMPLETION CHECKLIST

- [ ] All tasks completed in order
- [ ] Each task validation passed immediately
- [ ] All validation commands executed successfully
- [ ] No linting or type checking errors

---

## NOTES
Keep `main.rs` clean by placing logic in `tracker.rs`. Polling interval should be a const `Duration::from_secs(5)`.
