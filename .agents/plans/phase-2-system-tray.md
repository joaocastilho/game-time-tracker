# Feature: Phase 2 - System Tray Integration

The following plan should be complete, but its important that you validate documentation and codebase patterns and task sanity before you start implementing.

Pay special attention to naming of existing utils types and models. Import from the right files etc.

## Feature Description

Phase 2 introduces system tray integration for the Game Time Tracker. This makes the app run as a proper background application with a tray icon. The tray icon provides a context menu with options to Manage Games, Edit Sessions, Open Data Folder, and Quit. 

## User Story

As a gamer
I want the app to run silently in the system tray with a right-click context menu
So that I can easily access my data, manage games, or quit without needing a persistent terminal window.

## Problem Statement

Currently, the app only runs as a terminal process (running `AppTracker::run()`), which is not ideal for a background daemon. Users need a persistent way to interact with the tracker without keeping a terminal open.

## Solution Statement

Integrate the `tray-icon` and `tao` (or `winit`) crates to create a system tray icon. The main thread will run the tray event loop, while the `AppTracker::run` process will be moved to a background thread. In this phase, we will implement the tray icon, its tooltip (showing active game count), and a basic context menu handling actions like opening the data folder, opening the `sessions.json` file in notepad, and quitting the app safely.

## Feature Metadata

**Feature Type**: Enhancement
**Estimated Complexity**: Medium
**Primary Systems Affected**: `main.rs`, `tray.rs` (new), `tracker.rs`
**Dependencies**: `tray-icon`, `tao`, `muda` (usually required by `tray-icon` for menus), `open` (for opening files/folders)

---

## CONTEXT REFERENCES

### Patterns to Follow

**Shared State:**
The application uses `Arc<Mutex<State>>` or similar shared state mechanisms to allow the background tracker and the UI/Tray systems to communicate.

**Error Handling:**
Atomic file writes and `anyhow` are used. No unwraps in production code.

---

## IMPLEMENTATION PLAN

### Phase 1: Foundation

**Tasks:**
- Add required dependencies to `Cargo.toml`: `tray-icon`, `tao`, `muda`, `open`.
- Ensure Windows specific features if necessary, though `tray-icon` handles this.

### Phase 2: Core Implementation

**Tasks:**
- Create `src/tray.rs`.
- Define the context menu using `muda` (Manage Games, Edit Sessions, Open Data Folder, Quit).
- Initialize the `tray-icon` with the menu and a basic icon (can be a generated placeholder array of pixels for now, or load from a resource if you prefer, but a generated small RGBA buffer is easiest to start with).

### Phase 3: Integration

**Tasks:**
- Update `src/main.rs`.
- Move `AppTracker::run()` into a separate spawned thread (`std::thread::spawn`).
- Initialize `tao::event_loop::EventLoop` on the main thread.
- Set up the tray icon and menu before the event loop runs.
- Handle `muda::MenuEvent` and `tray_icon::TrayIconEvent` in the event loop.
- Implement action handlers:
  - "Quit": exit the event loop.
  - "Open Data Folder": use `open::that(config::data_dir())`.
  - "Edit Sessions": use `open::that(config::data_dir().join("sessions.json"))`.
  - "Manage Games": stub with an `info!` log (real UI comes in Phase 3).
- Implement tooltip updating. The tracker needs to communicate the number of active sessions to the tray icon so it can update its tooltip. This can be done by sharing an `Arc<AtomicUsize>` active game count, updated by the tracker thread and read by the tray.

### Phase 4: Testing & Validation

**Tasks:**
- Verify that `cargo build` passes.
- Manually run the application to ensure the tray icon appears and the menu works.

---

## STEP-BY-STEP TASKS

IMPORTANT: Execute every task in order, top to bottom. Each task is atomic and independently testable.

### UPDATE Cargo.toml
- **IMPLEMENT**: Add `tray-icon`, `tao`, `muda`, and `open` to `[dependencies]`. Use latest stable versions or "*" if unsure, but prefer specific versions (e.g. `tray-icon = "0.19"`, `tao = "0.30"`, `muda = "0.15"`, `open = "5.3"`).
- **VALIDATE**: `cargo check`

### CREATE src/tray.rs
- **IMPLEMENT**: Create `tray.rs`. Define a function `setup_tray(menu: &Menu) -> Result<TrayIcon, anyhow::Error>` that builds and returns the tray icon. Include a dummy icon generator (e.g., 32x32 RGBA buffer) so that the tray icon shows up.
- **IMPORTS**: `tray_icon::{TrayIconBuilder, Icon}`, `muda::Menu`.
- **VALIDATE**: `cargo check` (after adding `pub mod tray;` to `main.rs`)

### UPDATE src/tracker.rs
- **IMPLEMENT**: Modify `AppTracker` to accept a shared state for the active game count, e.g., `active_count: Arc<AtomicUsize>`. Update this count inside the `run` loop when `state.active_sessions.len()` changes. Pass this into `AppTracker::new()`.
- **VALIDATE**: `cargo check`

### UPDATE src/main.rs
- **IMPLEMENT**: 
  - Add `pub mod tray;`.
  - In `main()`, under `None` (running the tracker):
    - Create `EventLoopBuilder::new().build()`.
    - Create the `muda::Menu` with items: "Manage Games", "Edit Sessions", "Open Data Folder", "Quit".
    - Store the IDs of these menu items to match in events.
    - Call `tray::setup_tray(&menu)`.
    - Create an `Arc<AtomicUsize>` for `active_count`.
    - Spawn a new thread for `AppTracker::new(active_count.clone()).run()`.
    - Run the `EventLoop`. Inside the event loop closure, listen to `MenuEvent::receiver()` and `TrayIconEvent::receiver()`.
    - On "Quit", call `*control_flow = ControlFlow::Exit`.
    - On "Open Data Folder", use `open::that(config::data_dir())`.
    - On "Edit Sessions", use `open::that(config::data_dir().join("sessions.json"))`.
- **GOTCHA**: Windows requires the event loop to run on the main thread, and `tray-icon` must be created on the same thread as the event loop. `tao`'s `run` method diverges and takes over the main thread entirely, which is correct here.
- **VALIDATE**: `cargo build`

---

## TESTING STRATEGY

### Manual Validation
Because system tray integration involves OS-level UI features that cannot be easily unit-tested headlessly, the primary testing strategy is manual execution.
- Run `cargo run`.
- Verify a tray icon appears.
- Right-click the tray icon.
- Click "Open Data Folder" -> verify explorer opens.
- Click "Edit Sessions" -> verify text editor opens.
- Click "Quit" -> verify the app exits cleanly and the tray icon disappears.

---

## VALIDATION COMMANDS

### Level 1: Syntax & Style
`cargo fmt -- --check`
`cargo clippy -- -D warnings`

### Level 2: Unit Tests
`cargo test`

### Level 4: Manual Validation
`cargo run` (Follow manual steps above)
