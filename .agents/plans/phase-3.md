# Feature: Phase 3 - egui Game Management Window

## Feature Description

Add a graphical interface for managing games using `egui` via the `eframe` crate. The UI will display a list of configured games, their running status, total play time, and allow adding/editing/removing games. It will also show an expandable session history per game. The window will open from the existing "Manage Games" system tray menu item.

## User Story

As a gamer
I want to add and remove games from a simple UI
So that I can configure which games to track without editing files directly

## Problem Statement

Currently, users must manually edit `games.json` to add or remove games to be tracked, which is error-prone and tedious for non-technical users.

## Solution Statement

Implement a native UI window using `eframe` (egui) that provides a user-friendly interface for managing the `games.json` and viewing `sessions.json` data. The window will run on a separate thread spawned from the tray icon event loop in `main.rs`.

## Feature Metadata

**Feature Type**: New Capability
**Estimated Complexity**: Medium
**Primary Systems Affected**: `src/ui.rs` (new), `src/main.rs`, `Cargo.toml`
**Dependencies**: `eframe = "0.30"`

---

## CONTEXT REFERENCES

### Patterns to Follow

**Architectural Pattern:**
- **One module per concern**: All UI logic goes into `src/ui.rs`.
- **Thin `main.rs`**: The tray event loop in `main.rs` catches the "Manage Games" menu event and spawns the `eframe` window.
- **Threading**: `muda`'s menu events run on the `tao` event loop in the main thread. We can spawn `eframe::run_native` on a new thread so it runs its own event loop without blocking the tray icon.

**Error Handling:**
- Use `anyhow` for application-level handling.
- Graceful degradation: Log warnings using `log::warn!` or show errors in UI if JSON is malformed. No `.unwrap()` in production code.

**State Management:**
- The tracker and UI can communicate via the JSON files in `%APPDATA%`. `ui.rs` can simply read `games.json` and `state.json` via the existing `store::load` utilities to get the latest status, avoiding complex cross-thread locks. Or use `Arc<Mutex<State>>` if refactoring `tracker.rs`. For this phase, atomic JSON reads are safe and follow `store.rs` patterns.

---

## IMPLEMENTATION PLAN

### Phase 1: Foundation

**Tasks:**
- Add `eframe` dependency to `Cargo.toml`.
- Create `src/ui.rs` and define the `GameManagerApp` struct implementing `eframe::App`.
- Expose a public function `spawn_ui()` in `src/ui.rs`.

### Phase 2: Core Implementation

**Tasks:**
- Implement the `update` trait method for `GameManagerApp`.
- Create the layout: table of games, "Add Game" button, edit/remove row actions.
- Read/write `games.json` when adding/editing/removing games using `store::load` and `store::save`.
- Display session history by reading `sessions.json`.
- Display running status by reading `state.json`.

### Phase 3: Integration

**Tasks:**
- Modify `src/main.rs` to handle the `manage_games_item` event.
- When the event is received, call `ui::spawn_ui()`. Prevent spawning duplicate windows if one is already open (use an `Arc<AtomicBool>` to track window status).

### Phase 4: Testing & Validation

**Tasks:**
- Run `cargo fmt`, `cargo clippy`, and `cargo test`.
- Perform manual validation by launching the app, clicking "Manage Games", adding a game, and verifying `games.json`.

---

## STEP-BY-STEP TASKS

IMPORTANT: Execute every task in order, top to bottom. Each task is atomic and independently testable.

### UPDATE `Cargo.toml`
- **IMPLEMENT**: Add `eframe = "0.30"` under `[dependencies]`.
- **VALIDATE**: `cargo check`

### CREATE `src/ui.rs`
- **IMPLEMENT**: Define `GameManagerApp` struct implementing `eframe::App`. Use `egui::CentralPanel`. Implement table for games, read/write to files via `crate::store`. Add `pub fn spawn_ui(is_open: Arc<AtomicBool>)`.
- **IMPORTS**: `eframe`, `egui`, `crate::store`, `crate::models`, `std::sync::atomic`.
- **GOTCHA**: Ensure UI does not block the polling loop. Read JSON files cautiously to handle potential concurrent writes from `tracker.rs`. Handle `is_open` correctly (set to true on open, false on close).
- **VALIDATE**: `cargo check`

### UPDATE `src/main.rs`
- **IMPLEMENT**: Add `pub mod ui;` at the top. Handle `manage_games_item.id()` click by spawning a thread that calls `ui::spawn_ui()`. Keep track of `is_open` so we don't open multiple windows.
- **PATTERN**: `main.rs:105` (replace the info log).
- **VALIDATE**: `cargo clippy -- -D warnings`

---

## TESTING STRATEGY

### Unit Tests

Test `models.rs` serialization if new models are added, but mainly rely on manual UI tests for `ui.rs`.

### Integration Tests

No automated integration tests for UI. Rely on manual validation.

### Edge Cases

- Opening "Manage Games" multiple times (should only open one window).
- Adding a game with empty fields (should show error or disable save).
- Deleting a game that is currently running (tracker should gracefully ignore or stop tracking on next poll).

---

## VALIDATION COMMANDS

Execute every command to ensure zero regressions and 100% feature correctness.

### Level 1: Syntax & Style

`cargo fmt -- --check`
`cargo clippy -- -D warnings`

### Level 2: Unit Tests

`cargo test`

### Level 4: Manual Validation

1. Run `cargo run`.
2. Right click tray icon -> Manage Games.
3. Verify window opens. Add a game, click Save. Check if `games.json` is updated in `%APPDATA%/game-time-tracker`.
4. Close window and try opening again.
5. Exit app via tray icon -> Quit.

---

## ACCEPTANCE CRITERIA

- [ ] Feature implements all specified functionality
- [ ] All validation commands pass with zero errors
- [ ] Code follows project conventions and patterns
- [ ] No regressions in existing functionality
- [ ] "Manage Games" spawns an egui window successfully without blocking tray.

---

## COMPLETION CHECKLIST

- [ ] All tasks completed in order
- [ ] Each task validation passed immediately
- [ ] All validation commands executed successfully
- [ ] Full test suite passes
- [ ] No linting or type checking errors
- [ ] Manual testing confirms feature works
- [ ] Acceptance criteria all met
