# Game Time Tracker — Agent Guidelines

## Project Context

This is a Rust background application for Windows that tracks game play time via process polling. It runs as a **system tray app** with a small **egui window** for game management. Data is stored in JSON files.

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
├── docs/
│   └── PRD.md           # Product requirements
├── agents.md            # This file
├── README.md
├── Cargo.toml
└── Cargo.lock
```

## Coding Standards

### General Principles

- **Simplicity first** — no abstractions for single-use code. If it can be 50 lines, don't write 200.
- **Surgical changes** — when modifying code, touch only what's necessary. Match existing style.
- **No speculative features** — only implement what the PRD specifies.

### Rust Best Practices

1. **Error handling:**
   - Use `thiserror` for library-level error types.
   - Use `anyhow` for application-level error handling in `main.rs` and CLI commands.
   - Never use `.unwrap()` in production code. `.expect("reason")` is acceptable only for truly impossible failures (e.g., static regex compilation).
   - Propagate errors with `?` — don't swallow them.

2. **Naming:**
   - Use `snake_case` for functions, variables, modules.
   - Use `PascalCase` for types and enums.
   - Use `SCREAMING_SNAKE_CASE` for constants.
   - Name booleans as questions: `is_running`, `has_sessions`.

3. **Module organization:**
   - One module per concern (process detection, storage, tracking logic, CLI).
   - Keep `main.rs` thin — it should only wire things together.
   - Avoid circular dependencies between modules.

4. **Data structures:**
   - Derive `Serialize`, `Deserialize` for all persistent types.
   - Derive `Debug` for all types.
   - Derive `Clone` only when needed.
   - Use `chrono::DateTime<Utc>` for timestamps.

5. **Testing:**
   - Unit tests go in the same file as the code under test (`#[cfg(test)] mod tests { ... }`).
   - Integration tests go in `tests/`.
   - Mock the process detection layer for testing the tracker logic.
   - Tests must use `?` for error propagation, not `.expect()`.

6. **Logging:**
   - Use `log` crate macros (`info!`, `warn!`, `error!`, `debug!`).
   - `info!` for session start/stop events.
   - `warn!` for recovered pending sessions.
   - `error!` for I/O failures.
   - `debug!` for polling cycle details.

7. **Performance:**
   - The polling loop should be lightweight — avoid unnecessary allocations.
   - Only refresh the process list once per poll cycle, not per game.
   - Sleep between polls using `std::thread::sleep` or `tokio::time::sleep`.

8. **egui UI:**
   - Keep the UI simple — one window, minimal state.
   - The UI communicates with the tracker via shared state (e.g., `Arc<Mutex<>>`) or channels.
   - Don't block the UI thread with I/O — file reads/writes happen on a background thread.
   - Use `egui::CentralPanel` for layout. Avoid complex nested panels.

9. **Platform-specific code:**
   - All Windows-specific code (registry, process enumeration) should be behind `#[cfg(target_os = "windows")]`.
   - Use `dirs` crate for paths — don't hardcode `C:\Users\...`.

### Code Style

- Run `cargo fmt` before every commit.
- Run `cargo clippy -- -D warnings` and fix all warnings.
- No `#[allow(...)]` without a comment explaining why.

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`.

Examples:
- `feat(tracker): add process polling loop`
- `fix(store): handle missing sessions file on first run`
- `docs: add PRD and README`

### Dependencies

- Prefer well-maintained crates with minimal dependency trees.
- Pin major versions in `Cargo.toml` (e.g., `serde = "1"`).
- Audit new dependencies before adding — check download counts, last update, and license.

### File I/O Safety

- Always write to a temporary file first, then atomically rename — never write directly to the data file.
- Load files with graceful fallback to defaults if they don't exist (first run).
- Validate JSON structure after loading — log warnings for malformed data but don't crash.
