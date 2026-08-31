#![windows_subsystem = "windows"]

pub mod config;
pub mod icon;
pub mod models;
pub mod process;
pub mod store;
pub mod tracker;

use clap::{Parser, Subcommand};
use log::{error, info};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager,
};

fn normalize_windows_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    if let Some(stripped) = path_str.strip_prefix("\\\\?\\") {
        stripped.to_string()
    } else {
        path_str.to_string()
    }
}

use models::{Game, Session, State};
use tracker::AppTracker;
use winreg::enums::*;
use winreg::RegKey;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Install,
    Uninstall,
}

// Data payload returned to the UI
#[derive(serde::Serialize)]
struct UiData {
    games: Vec<Game>,
    sessions: HashMap<String, Vec<Session>>,
    state: State,
}

#[tauri::command]
fn get_ui_data() -> Result<UiData, String> {
    let dir = config::data_dir();
    let games: Vec<Game> = match store::load(dir.join("games.json")) {
        Ok(opt) => opt.unwrap_or_default(),
        Err(e) => {
            log::warn!("Failed to load games.json: {} — using default", e);
            Vec::new()
        }
    };
    let sessions: std::collections::HashMap<String, Vec<Session>> =
        match store::load(dir.join("sessions.json")) {
            Ok(opt) => opt.unwrap_or_default(),
            Err(e) => {
                log::warn!("Failed to load sessions.json: {} — using default", e);
                Default::default()
            }
        };
    let state: State = match store::load(dir.join("state.json")) {
        Ok(opt) => opt.unwrap_or_default(),
        Err(e) => {
            log::warn!("Failed to load state.json: {} — using default", e);
            State::default()
        }
    };

    Ok(UiData {
        games,
        sessions,
        state,
    })
}

#[tauri::command]
fn add_game(name: String, executable: String) -> Result<(), String> {
    let dir = config::data_dir();
    let games_path = dir.join("games.json");
    let mut games: Vec<Game> = store::load(&games_path)
        .map_err(|e| format!("Load error: {}", e))?
        .unwrap_or_default();

    let trimmed_name = name.trim().to_string();
    let trimmed_exec = executable.trim().to_string();

    if trimmed_name.is_empty() || trimmed_exec.is_empty() {
        return Err("All fields must be filled out".into());
    }
    if trimmed_name.chars().count() > 100 {
        return Err("Name must be at most 100 characters".into());
    }
    if trimmed_exec.chars().count() > 100 {
        return Err("Executable must be at most 100 characters".into());
    }
    if !trimmed_name.chars().any(|c| c.is_alphanumeric()) {
        return Err("Name must contain alphanumeric characters".into());
    }

    let game_id = Game::generate_id(&trimmed_name);
    // generate_id never returns empty (falls back to "unnamed-game"), but guard
    if game_id.is_empty() {
        return Err("Name must contain alphanumeric characters".into());
    }

    if games.iter().any(|g| g.id == game_id) {
        return Err("Game already exists".into());
    }
    if games
        .iter()
        .any(|g| g.executable.eq_ignore_ascii_case(&trimmed_exec))
    {
        return Err("Executable already tracked for another game".into());
    }

    if trimmed_exec.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("Executable name contains invalid characters".into());
    }

    if !trimmed_exec.to_lowercase().ends_with(".exe") {
        return Err("Executable must have .exe extension".into());
    }

    games.push(Game {
        id: game_id,
        name: trimmed_name,
        executable: trimmed_exec,
    });

    store::save(&games, &games_path).map_err(|e| format!("Save error: {}", e))?;
    Ok(())
}

#[tauri::command]
fn get_sessions() -> Result<std::collections::HashMap<String, Vec<Session>>, String> {
    let dir = config::data_dir();
    let sessions: std::collections::HashMap<String, Vec<Session>> =
        match store::load(dir.join("sessions.json")) {
            Ok(opt) => opt.unwrap_or_default(),
            Err(e) => {
                log::warn!("Failed to load sessions.json: {} — using default", e);
                Default::default()
            }
        };
    let games: Vec<Game> = match store::load(dir.join("games.json")) {
        Ok(opt) => opt.unwrap_or_default(),
        Err(e) => {
            log::warn!("Failed to load games.json: {} — using default", e);
            Vec::new()
        }
    };
    let game_ids: std::collections::HashSet<String> = games.iter().map(|g| g.id.clone()).collect();

    let orphaned: std::collections::HashMap<String, Vec<Session>> = sessions
        .into_iter()
        .filter(|(id, _)| !game_ids.contains(id))
        .collect();

    Ok(orphaned)
}

#[tauri::command]
fn remove_orphaned_session(game_id: String) -> Result<(), String> {
    let dir = config::data_dir();
    let sessions_path = dir.join("sessions.json");
    let games_path = dir.join("games.json");

    // Only allow deleting orphaned sessions (games that no longer exist)
    let games: Vec<Game> = match store::load(&games_path) {
        Ok(opt) => opt.unwrap_or_default(),
        Err(e) => {
            log::warn!("Failed to load games.json for orphan check: {}", e);
            Vec::new()
        }
    };
    if games.iter().any(|g| g.id == game_id) {
        return Err("Cannot delete sessions for an active game — remove the game first".into());
    }

    let mut sessions: std::collections::HashMap<String, Vec<Session>> = store::load(&sessions_path)
        .map_err(|e| format!("Load error: {}", e))?
        .unwrap_or_default();

    if sessions.remove(&game_id).is_none() {
        return Err("No orphaned sessions found for this game".into());
    }

    store::save(&sessions, &sessions_path).map_err(|e| format!("Save error: {}", e))?;
    Ok(())
}

#[tauri::command]
fn remove_game(id: String) -> Result<(), String> {
    let dir = config::data_dir();
    let games_path = dir.join("games.json");
    let sessions_path = dir.join("sessions.json");
    let state_path = dir.join("state.json");

    let mut games: Vec<Game> = store::load(&games_path)
        .map_err(|e| format!("Load error: {}", e))?
        .unwrap_or_default();

    let original_len = games.len();
    games.retain(|g| g.id != id);
    if games.len() == original_len {
        return Err("Game not found".into());
    }

    store::save(&games, &games_path).map_err(|e| format!("Save error: {}", e))?;

    // Remove sessions for the removed game – log but don't fail the whole op
    match store::load::<std::collections::HashMap<String, Vec<Session>>, _>(&sessions_path) {
        Ok(Some(mut sessions)) => {
            if sessions.remove(&id).is_some() {
                if let Err(e) = store::save(&sessions, &sessions_path) {
                    log::warn!("Failed to clean sessions for removed game {}: {}", id, e);
                }
            }
        }
        Ok(None) => {}
        Err(e) => {
            log::warn!("Failed to load sessions for cleanup: {}", e);
        }
    }

    match store::load::<models::State, _>(&state_path) {
        Ok(Some(mut state)) => {
            if state.active_sessions.remove(&id).is_some() {
                if let Err(e) = store::save(&state, &state_path) {
                    log::warn!("Failed to clean state for removed game {}: {}", id, e);
                }
            }
        }
        Ok(None) => {}
        Err(e) => {
            log::warn!("Failed to load state for cleanup: {}", e);
        }
    }

    Ok(())
}

#[tauri::command]
fn get_running_processes() -> Result<Vec<String>, String> {
    let mut monitor = process::ProcessMonitor::new();
    Ok(monitor.list_processes())
}

#[tauri::command]
fn update_game(id: String, name: String, executable: String) -> Result<(), String> {
    let dir = config::data_dir();
    let games_path = dir.join("games.json");
    let mut games: Vec<Game> = store::load(&games_path)
        .map_err(|e| format!("Load error: {}", e))?
        .unwrap_or_default();

    let pos = games
        .iter()
        .position(|g| g.id == id)
        .ok_or("Game not found")?;

    let trimmed_name = name.trim().to_string();
    let trimmed_exec = executable.trim().to_string();

    if trimmed_name.is_empty() || trimmed_exec.is_empty() {
        return Err("All fields must be filled out".into());
    }
    if trimmed_name.chars().count() > 100 {
        return Err("Name must be at most 100 characters".into());
    }
    if trimmed_exec.chars().count() > 100 {
        return Err("Executable must be at most 100 characters".into());
    }
    if !trimmed_name.chars().any(|c| c.is_alphanumeric()) {
        return Err("Name must contain alphanumeric characters".into());
    }

    if trimmed_exec.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
        return Err("Executable name contains invalid characters".into());
    }

    if !trimmed_exec.to_lowercase().ends_with(".exe") {
        return Err("Executable must have .exe extension".into());
    }
    if games
        .iter()
        .enumerate()
        .any(|(i, g)| i != pos && g.executable.eq_ignore_ascii_case(&trimmed_exec))
    {
        return Err("Executable already tracked for another game".into());
    }

    games[pos].name = trimmed_name;
    games[pos].executable = trimmed_exec;

    store::save(&games, &games_path).map_err(|e| format!("Save error: {}", e))?;
    Ok(())
}

fn parse_session_datetime(s: &str) -> Result<chrono::DateTime<chrono::Utc>, String> {
    s.parse::<chrono::DateTime<chrono::Utc>>()
        .map_err(|e| format!("Invalid datetime '{}': {}", s, e))
}

#[tauri::command]
fn update_session(
    game_id: String,
    index: usize,
    start: String,
    end: Option<String>,
) -> Result<(), String> {
    let dir = config::data_dir();
    let sessions_path = dir.join("sessions.json");

    let mut sessions: std::collections::HashMap<String, Vec<Session>> = store::load(&sessions_path)
        .map_err(|e| format!("Load error: {}", e))?
        .unwrap_or_default();

    let list = sessions
        .get_mut(&game_id)
        .ok_or("No sessions for this game")?;

    if index >= list.len() {
        return Err("Session index out of bounds".into());
    }

    let start_dt = parse_session_datetime(&start)?;
    let end_dt = match &end {
        Some(e) if !e.trim().is_empty() => Some(parse_session_datetime(e)?),
        _ => None,
    };

    if let Some(end_val) = end_dt {
        if end_val < start_dt {
            return Err("End time must be after start time".into());
        }
    }

    let duration_secs = if let Some(end_val) = end_dt {
        (end_val - start_dt).num_seconds().max(0) as u64
    } else {
        0
    };

    list[index] = Session {
        start: start_dt,
        end: end_dt,
        duration_secs,
    };

    store::save(&sessions, &sessions_path).map_err(|e| format!("Save error: {}", e))?;
    Ok(())
}

#[tauri::command]
fn delete_session(game_id: String, index: usize) -> Result<(), String> {
    let dir = config::data_dir();
    let sessions_path = dir.join("sessions.json");

    let mut sessions: std::collections::HashMap<String, Vec<Session>> = store::load(&sessions_path)
        .map_err(|e| format!("Load error: {}", e))?
        .unwrap_or_default();

    let list = sessions
        .get_mut(&game_id)
        .ok_or("No sessions for this game")?;

    if index >= list.len() {
        return Err("Session index out of bounds".into());
    }

    list.remove(index);
    if list.is_empty() {
        sessions.remove(&game_id);
    }

    store::save(&sessions, &sessions_path).map_err(|e| format!("Save error: {}", e))?;
    Ok(())
}

#[tauri::command]
fn add_session(game_id: String, start: String, end: Option<String>) -> Result<(), String> {
    let dir = config::data_dir();
    let sessions_path = dir.join("sessions.json");
    let games_path = dir.join("games.json");

    // Validate game exists (or allow orphaned? require existence)
    let games: Vec<Game> = store::load(&games_path)
        .map_err(|e| format!("Load error: {}", e))?
        .unwrap_or_default();
    if !games.iter().any(|g| g.id == game_id) {
        return Err("Game not found".into());
    }

    let start_dt = parse_session_datetime(&start)?;
    let end_dt = match &end {
        Some(e) if !e.trim().is_empty() => Some(parse_session_datetime(e)?),
        _ => None,
    };

    if let Some(end_val) = end_dt {
        if end_val < start_dt {
            return Err("End time must be after start time".into());
        }
    }

    let duration_secs = if let Some(end_val) = end_dt {
        (end_val - start_dt).num_seconds().max(0) as u64
    } else {
        0
    };

    let mut sessions: std::collections::HashMap<String, Vec<Session>> = store::load(&sessions_path)
        .map_err(|e| format!("Load error: {}", e))?
        .unwrap_or_default();

    sessions.entry(game_id).or_default().push(Session {
        start: start_dt,
        end: end_dt,
        duration_secs,
    });

    store::save(&sessions, &sessions_path).map_err(|e| format!("Save error: {}", e))?;
    Ok(())
}

fn calculate_md5(path: &Path) -> Result<String, anyhow::Error> {
    let mut file = fs::File::open(path)?;
    // Stream to avoid OOM on large files
    let mut ctx = md5::Context::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = std::io::Read::read(&mut file, &mut buf)?;
        if n == 0 {
            break;
        }
        ctx.consume(&buf[..n]);
    }
    let digest = ctx.compute();
    Ok(format!("{:x}", digest))
}

fn decode_registry_path_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    // REG_SZ / REG_EXPAND_SZ are UTF-16LE with trailing 0x00 0x00.
    // Decode as UTF-16LE, handling odd length gracefully.
    let wide: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let mut s = String::from_utf16_lossy(&wide);
    // Trim trailing nulls (the REG_SZ terminator) and any embedded nulls
    s = s.trim_end_matches('\0').to_string();
    s.replace('\0', "")
}

fn add_to_path(dir: &Path) -> Result<(), anyhow::Error> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;

    let (current_path, orig_type) = if let Ok(raw) = env.get_raw_value("Path") {
        (decode_registry_path_bytes(&raw.bytes), raw.vtype)
    } else {
        (String::new(), winreg::enums::RegType::REG_EXPAND_SZ)
    };

    let dir_str = dir.to_string_lossy();
    let normalized_dir = dir_str.trim_end_matches('\\').to_string();

    let mut parts: Vec<String> = current_path
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| {
            let normalized = s.trim_end_matches('\\');
            !normalized.is_empty() && !normalized.eq_ignore_ascii_case(&normalized_dir)
        })
        .collect();

    // Avoid duplicate – only push if not already present (case-insensitive)
    if !parts.iter().any(|p| {
        p.trim_end_matches('\\')
            .eq_ignore_ascii_case(&normalized_dir)
    }) {
        parts.push(normalized_dir);
    }
    let new_path = parts.join(";");

    // Preserve original type if it was REG_SZ, otherwise use EXPAND_SZ
    let vtype = if orig_type == winreg::enums::RegType::REG_SZ {
        winreg::enums::RegType::REG_SZ
    } else {
        winreg::enums::RegType::REG_EXPAND_SZ
    };

    let utf16_bytes: Vec<u8> = std::ffi::OsString::from(new_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .flat_map(|u| u.to_le_bytes())
        .collect();

    env.set_raw_value(
        "Path",
        &winreg::RegValue {
            vtype,
            bytes: utf16_bytes,
        },
    )?;

    let _ = std::process::Command::new("powershell")
        .args(["-Command", "[Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')"])
        .creation_flags(0x08000000)
        .spawn();

    Ok(())
}

fn remove_from_path(dir: &Path) -> Result<(), anyhow::Error> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;

    let (current_path, orig_type) = if let Ok(raw) = env.get_raw_value("Path") {
        (decode_registry_path_bytes(&raw.bytes), raw.vtype)
    } else {
        (String::new(), winreg::enums::RegType::REG_EXPAND_SZ)
    };

    let dir_str = dir.to_string_lossy();
    let normalized_dir = dir_str.trim_end_matches('\\').to_string();

    let parts: Vec<String> = current_path
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| {
            let normalized = s.trim_end_matches('\\');
            !normalized.is_empty() && !normalized.eq_ignore_ascii_case(&normalized_dir)
        })
        .collect();

    let new_path = parts.join(";");
    // Compare decoded forms to avoid false positive when original had stray nulls/spaces
    let cleaned_decoded = current_path
        .split(';')
        .map(|s| s.trim().trim_end_matches('\\').to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(";");
    if new_path != cleaned_decoded {
        let vtype = if orig_type == winreg::enums::RegType::REG_SZ {
            winreg::enums::RegType::REG_SZ
        } else {
            winreg::enums::RegType::REG_EXPAND_SZ
        };
        let utf16_bytes: Vec<u8> = std::ffi::OsString::from(new_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .flat_map(|u| u.to_le_bytes())
            .collect();

        env.set_raw_value(
            "Path",
            &winreg::RegValue {
                vtype,
                bytes: utf16_bytes,
            },
        )?;

        let _ = std::process::Command::new("powershell")
            .args(["-Command", "[Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')"])
            .creation_flags(0x08000000)
            .spawn();
    }
    Ok(())
}

fn install_logic(silent: bool) -> Result<(), anyhow::Error> {
    let current_exe = env::current_exe()?;
    let install_dir = config::bin_dir();
    let target_exe = install_dir.join("gtt.exe");

    let current_exe_str = normalize_windows_path(&current_exe);
    let target_exe_str = normalize_windows_path(&target_exe);

    let mut needs_copy = false;

    if !target_exe.exists() {
        needs_copy = true;
    } else if !current_exe_str.eq_ignore_ascii_case(&target_exe_str) {
        let current_hash = calculate_md5(&current_exe)?;
        let target_hash = calculate_md5(&target_exe)?;
        if current_hash != target_hash {
            if !silent {
                println!("Version mismatch detected, updating installed executable.");
            }
            needs_copy = true;
        }
    }

    if needs_copy {
        if !install_dir.exists() {
            fs::create_dir_all(&install_dir)?;
        }

        if !current_exe_str.eq_ignore_ascii_case(&target_exe_str) {
            if !silent {
                println!(
                    "Installing/Updating executable to: {}",
                    target_exe.display()
                );
            }
            fs::copy(&current_exe, &target_exe)?;
        }
    }

    // Ensure startup registry key exists and points to the target_exe
    // Quote the path because %APPDATA% often contains spaces (e.g. "C:\Users\First Last\...")
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_path = r#"Software\Microsoft\Windows\CurrentVersion\Run"#;
    match hkcu.open_subkey_with_flags(run_path, KEY_SET_VALUE) {
        Ok(key) => {
            let exe_str = normalize_windows_path(&target_exe);
            let exe_quoted = format!("\"{}\"", exe_str);
            if let Err(e) = key.set_value("GameTimeTracker", &exe_quoted) {
                log::warn!("Failed to set autostart registry value: {}", e);
            }
        }
        Err(e) => {
            log::warn!("Failed to open Run registry key: {}", e);
        }
    }

    // Add to PATH
    if let Err(e) = add_to_path(&install_dir) {
        log::warn!("Failed to add to PATH: {}", e);
    }

    if !silent {
        println!("Successfully installed.");
    }

    Ok(())
}

fn uninstall_logic() -> Result<(), anyhow::Error> {
    // Remove from PATH
    let install_dir = config::bin_dir();
    if let Err(e) = remove_from_path(&install_dir) {
        log::warn!("Failed to remove from PATH: {}", e);
    }

    // Remove startup key
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_path = r#"Software\Microsoft\Windows\CurrentVersion\Run"#;
    if let Ok(key) = hkcu.open_subkey_with_flags(run_path, KEY_SET_VALUE) {
        key.delete_value("GameTimeTracker").ok();
    }

    println!("Successfully uninstalled. You can now manually delete the data folder if desired.");
    Ok(())
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();

    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Install) => {
            if let Err(e) = install_logic(false) {
                error!("Installation failed: {}", e);
            }
            return;
        }
        Some(Commands::Uninstall) => {
            if let Err(e) = uninstall_logic() {
                error!("Uninstallation failed: {}", e);
            }
            return;
        }
        None => {}
    }

    info!("Starting Game Time Tracker daemon mapped to Tauri.");

    let active_count = Arc::new(AtomicUsize::new(0));
    let should_stop = Arc::new(AtomicBool::new(false));
    let should_stop_run = should_stop.clone();
    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
    let stop_tx_run = stop_tx.clone();

    let tauri_app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.get_webview_window("main").map(|w| {
                let _ = w.show();
                let _ = w.set_focus();
            });
        }))
        .setup(move |app| {
            // Setup system tray menu
            let manage_i = MenuItemBuilder::with_id("manage", "Manage Games").build(app)?;
            let sessions_i = MenuItemBuilder::with_id("sessions", "Edit Sessions").build(app)?;
            let data_i = MenuItemBuilder::with_id("data", "Open Data Folder").build(app)?;
            let quit_i = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&manage_i, &sessions_i, &data_i, &quit_i])
                .build()?;

            let active_clone = active_count.clone();
            let should_stop_menu = should_stop.clone();
            let stop_tx_menu = stop_tx.clone();

            let tray_icon = app
                .default_window_icon()
                .cloned()
                .unwrap_or_else(|| icon::icon_tauri_image());

            let tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Game Time Tracker")
                .icon(tray_icon)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::DoubleClick { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .on_menu_event(move |app_handle, event| {
                    if event.id() == "manage" {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    } else if event.id() == "sessions" {
                        let _ = open::that(config::data_dir().join("sessions.json"));
                    } else if event.id() == "data" {
                        let _ = open::that(config::data_dir());
                    } else if event.id() == "quit" {
                        should_stop_menu.store(true, Ordering::SeqCst);
                        let _ = stop_tx_menu.send(());
                        app_handle.exit(0);
                    }
                })
                .build(app)?;

            // Background thread setup
            let active_count_tracker = active_count.clone();
            let should_stop_tracker = should_stop.clone();
            std::thread::spawn(move || {
                let mut tracker =
                    AppTracker::new(active_count_tracker, should_stop_tracker, stop_rx);
                if let Err(e) = tracker.run() {
                    error!("Tracker stopped: {}", e);
                }
            });

            // Updating tray tooltip based on active count (polling task)
            let tray_handle = tray.clone();
            let should_stop_tooltip = should_stop.clone();
            std::thread::spawn(move || {
                let mut last = 0;
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if should_stop_tooltip.load(Ordering::SeqCst) {
                        break;
                    }
                    let current = active_clone.load(Ordering::SeqCst);
                    if current != last {
                        last = current;
                        let _ = tray_handle
                            .set_tooltip(Some(format!("Game Time Tracker ({} active)", current)));
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_ui_data,
            add_game,
            remove_game,
            get_sessions,
            remove_orphaned_session,
            get_running_processes,
            update_game,
            update_session,
            delete_session,
            add_session
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // When the user clicks the "X" button, hide the window instead of killing the app
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            error!("Failed to build tauri application: {}", e);
            std::process::exit(1);
        });

    tauri_app.run(move |_app_handle, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            // Clean shutdown for tracker
            should_stop_run.store(true, Ordering::SeqCst);
            let _ = stop_tx_run.send(());
        }
    });
}
