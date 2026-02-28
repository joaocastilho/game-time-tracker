---
description: "Implementation plan for Phase 4: Auto-Start & Polish"
---

# Feature: Auto-Start & Polish (Phase 4)

The following plan should be complete, but its important that you validate documentation and codebase patterns and task sanity before you start implementing.

## Feature Description

Enable the application to automatically start with Windows by implementing the `gtt install` and `gtt uninstall` CLI commands, which will interact with the Windows Registry (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`). Additionally, perform an error handling audit to remove all `.unwrap()` or `.expect()` calls in production code, ensuring graceful degradation, robust error handling, and appropriate logging according to the PRD.

## User Story

As a gamer
I want the app to start automatically when I log into Windows
So that I never forget to run it and consistently track my games

## Problem Statement

Currently, the app must be manually started after every reboot because the CLI commands for installing and uninstalling auto-start are stubbed out. Furthermore, there are a few lingering `.unwrap()` and `.expect()` calls in the codebase that could cause the app to crash unpredictably instead of handling errors gracefully.

## Solution Statement

Implement the Windows Registry modification logic using the `winreg` crate for the `Install` and `Uninstall` commands in the CLI. Audit the codebase, primarily `src/main.rs` and `src/tracker.rs`, for panicking methods (`.unwrap()` and `.expect()`), replacing them with robust error propagation (using `?`, `anyhow::Context`, or `if let`).

## Feature Metadata

**Feature Type**: New Capability & Refactor
**Estimated Complexity**: Low
**Primary Systems Affected**: `src/main.rs`, `src/tracker.rs`, `Cargo.toml`
**Dependencies**: `winreg = "0.55"`

---

## CONTEXT REFERENCES

### Patterns to Follow

**Error Handling:**
The app relies on the `anyhow` crate for application-level error handling. Error flows should propagate to the top of `main.rs` using the `?` operator. For Option types, use robust checks like `if let` or `let Some(...) = ... else { ... }` rather than `.unwrap()`.
Example of a safe fallback pattern currently in `src/tracker.rs`:
```rust
let end_time = state.last_seen.unwrap_or_else(Utc::now);
```

**Logging Pattern:**
Log information about the system state and any gracefully handled edge cases using the `log` crate (`info!`, `error!`, `warn!`).
Example from `src/main.rs`:
```rust
info!("Starting game-time-tracker with system tray...");
```

---

## IMPLEMENTATION PLAN

### Phase 1: Foundation
**Tasks:**
- Add `winreg` dependency to `Cargo.toml`.

### Phase 2: Core Implementation
**Tasks:**
- Implement the auto-start logic in `src/main.rs` for `Commands::Install`. It will add the app's executable path to `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
- Implement `Commands::Uninstall` to remove the registry key.

### Phase 3: Integration & Audit
**Tasks:**
- Perform error handling audit in `src/main.rs`. Replace `.unwrap()` and `.expect()` with proper `?` bubbling and `anyhow::Context`.
- Perform error handling audit in `src/tracker.rs`. Replace `.unwrap()` on `active_sessions.remove()` with an `let Some(...) else { ... }` block to avoid panic if the state becomes desynced.

### Phase 4: Testing & Validation
**Tasks:**
- Compile the code targeting Windows.
- Ensure no warnings or clippy errors remain.

---

## STEP-BY-STEP TASKS

IMPORTANT: Execute every task in order, top to bottom. Each task is atomic and independently testable.

### UPDATE `Cargo.toml`
- **IMPLEMENT**: Add `winreg = "0.55"` to the `[dependencies]` list.
- **VALIDATE**: `cargo check`

### UPDATE `src/main.rs`
- **IMPLEMENT**: Implement the `Commands::Install` and `Commands::Uninstall` match arms. Use `winreg::RegKey` to open `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` and set/delete the `"GameTimeTracker"` value pointing to `std::env::current_exe()`. Ensure you convert the path to a string safely. 
- **IMPORTS**: `use winreg::enums::*;`, `use winreg::RegKey;`, `use std::env;`.
- **GOTCHA**: `env::current_exe()` returns `PathBuf`. Convert it to a string cleanly and handle invalid UTF-8 by mapping it to an error: `exe_path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid executable path"))?`.
- **VALIDATE**: `cargo check`

### REFACTOR `src/main.rs`
- **IMPLEMENT**: Remove `.unwrap()` and `.expect()` calls in `main.rs`. Specifically:
  - Change `menu.append_items(...).unwrap();` to `menu.append_items(...)?;`
  - Change `tray::setup_tray(&menu).expect(...)` to `tray::setup_tray(&menu).context("Failed to setup tray icon")?;`
- **IMPORTS**: `use anyhow::Context;`
- **VALIDATE**: `cargo check`

### REFACTOR `src/tracker.rs`
- **IMPLEMENT**: In the `run` method, change `let mut session = state.active_sessions.remove(&game_id).unwrap();` (around line 103) to safely handle situations where it doesn't exist. Use `let Some(mut session) = state.active_sessions.remove(&game_id) else { continue; };`.
- **VALIDATE**: `cargo clippy -- -D warnings`

---

## TESTING STRATEGY

### Unit Tests
Run existing cargo tests. The core logic added here heavily interacts with the Windows OS (Registry), which is best validated manually or via careful code review rather than mocked unit tests.

### Edge Cases
- Missing permissions for registry: Handled safely since we write to the HKCU (Current User) space, avoiding admin escalation.
- Path to executable containing invalid UTF-8: App handles it gracefully by checking `.to_str()` and returning an `anyhow` error.
- Desynchronized active states missing from the tracked map: Avoid app panic by removing the `.unwrap()`.

---

## VALIDATION COMMANDS

Execute every command to ensure zero regressions and 100% feature correctness.

### Level 1: Syntax & Style
`cargo fmt -- --check`
`cargo clippy -- -D warnings`

### Level 2: Unit Tests
`cargo test`

### Level 4: Manual Validation
1. Build and run: `cargo run -- install`
2. Open Windows Registry Editor (Run: `regedit`).
3. Verify that `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` contains `GameTimeTracker` pointing to your `.exe`.
4. Run: `cargo run -- uninstall`
5. Check that the registry key was removed.

---

## ACCEPTANCE CRITERIA

- [ ] Feature implements all specified functionality
- [ ] All validation commands pass with zero errors
- [ ] Code follows project conventions and patterns
- [ ] No `.unwrap()` or `.expect()` calls remain in production code (except tests)
- [ ] Installation commands correctly modify the registry

---

## COMPLETION CHECKLIST

- [ ] All tasks completed in order
- [ ] Each task validation passed immediately
- [ ] All validation commands executed successfully
- [ ] No linting or type checking errors
- [ ] Manual testing confirms feature works

---

## NOTES
This phase focuses heavily on application robustness. A crash-resistant tracker is essential because users rely on it silently running in the background without intervention.
