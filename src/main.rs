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
    let games = store::load(dir.join("games.json"))
        .unwrap_or(None)
        .unwrap_or_default();
    let sessions = store::load(dir.join("sessions.json"))
        .unwrap_or(None)
        .unwrap_or_default();
    let state = store::load(dir.join("state.json"))
        .unwrap_or(None)
        .unwrap_or_default();

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

    let game_id = Game::generate_id(&trimmed_name);
    if game_id.is_empty() {
        return Err("Name must contain alphanumeric characters".into());
    }

    if games.iter().any(|g| g.id == game_id) {
        return Err("Game already exists".into());
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
fn remove_game(id: String) -> Result<(), String> {
    let dir = config::data_dir();
    let games_path = dir.join("games.json");
    let mut games: Vec<Game> = store::load(&games_path)
        .map_err(|e| format!("Load error: {}", e))?
        .unwrap_or_default();

    games.retain(|g| g.id != id);

    store::save(&games, &games_path).map_err(|e| format!("Save error: {}", e))?;
    Ok(())
}

fn calculate_md5(path: &Path) -> Result<String, anyhow::Error> {
    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buffer)?;
    let digest = md5::compute(buffer);
    Ok(format!("{:x}", digest))
}

fn add_to_path(dir: &Path) -> Result<(), anyhow::Error> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
    
    // Read raw to include any nulls or corruption
    let current_path = if let Ok(raw) = env.get_raw_value("Path") {
        String::from_utf8_lossy(&raw.bytes).to_string()
    } else {
        String::new()
    };
    
    let cleaned_path = current_path.replace('\0', "");
    let dir_str = dir.to_string_lossy();
    let normalized_dir = dir_str.trim_end_matches('\\').to_string();

    let mut parts: Vec<String> = cleaned_path.split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| {
            let s_clean = s.trim_end_matches('\\');
            !s_clean.is_empty() && !s_clean.eq_ignore_ascii_case(&normalized_dir) && !s_clean.to_lowercase().contains("game-time-tracker")
        })
        .collect();
    
    parts.push(normalized_dir);
    let new_path = parts.join(";");
    
    // Use REG_EXPAND_SZ with UTF-16 encoding for Windows compatibility
    let utf16_bytes: Vec<u8> = std::ffi::OsString::from(new_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .flat_map(|u| u.to_le_bytes())
        .collect();

    env.set_raw_value("Path", &winreg::RegValue {
        vtype: winreg::enums::RegType::REG_EXPAND_SZ,
        bytes: utf16_bytes,
    })?;
    
    let _ = std::process::Command::new("powershell")
        .args(["-Command", "[Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')"])
        .creation_flags(0x08000000)
        .spawn();

    Ok(())
}

fn remove_from_path(dir: &Path) -> Result<(), anyhow::Error> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
    
    let current_path = if let Ok(raw) = env.get_raw_value("Path") {
        String::from_utf8_lossy(&raw.bytes).to_string()
    } else {
        String::new()
    };
    
    let cleaned_path = current_path.replace('\0', "");
    let dir_str = dir.to_string_lossy();
    let normalized_dir = dir_str.trim_end_matches('\\').to_string();

    let parts: Vec<String> = cleaned_path.split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| {
            let s_clean = s.trim_end_matches('\\');
            !s_clean.is_empty() && !s_clean.eq_ignore_ascii_case(&normalized_dir) && !s_clean.to_lowercase().contains("game-time-tracker")
        })
        .collect();
    
    let new_path = parts.join(";");
    if new_path != cleaned_path {
        let utf16_bytes: Vec<u8> = std::ffi::OsString::from(new_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .flat_map(|u| u.to_le_bytes())
            .collect();

        env.set_raw_value("Path", &winreg::RegValue {
            vtype: winreg::enums::RegType::REG_EXPAND_SZ,
            bytes: utf16_bytes,
        })?;
        
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

    let mut needs_copy = false;

    if !target_exe.exists() {
        needs_copy = true;
    } else {
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
        
        if current_exe != target_exe {
            if !silent {
                println!("Installing/Updating executable to: {}", target_exe.display());
            }
            fs::copy(&current_exe, &target_exe)?;
        }
    }

    // Ensure startup registry key exists and points to the target_exe
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_path = r#"Software\Microsoft\Windows\CurrentVersion\Run"#;
    if let Ok(key) = hkcu.open_subkey_with_flags(run_path, KEY_SET_VALUE) {
        if let Some(exe_str) = target_exe.to_str() {
            let _ = key.set_value("GameTimeTracker", &exe_str);
        }
    }

    // Add to PATH
    let _ = add_to_path(&install_dir);

    if !silent {
        println!("Successfully installed.");
    }

    Ok(())
}

fn uninstall_logic() -> Result<(), anyhow::Error> {
    // Remove from PATH
    let install_dir = config::bin_dir();
    let _ = remove_from_path(&install_dir);

    // Remove startup key
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_path = r#"Software\Microsoft\Windows\CurrentVersion\Run"#;
    if let Ok(key) = hkcu.open_subkey_with_flags(run_path, KEY_SET_VALUE) {
        let _ = key.delete_value("GameTimeTracker");
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

    let tauri_app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app
                .get_webview_window("main")
                .map(|w| {
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

            let tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Game Time Tracker")
                .icon(app.default_window_icon().unwrap().clone())
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
            std::thread::spawn(move || {
                let mut last = 0;
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    let current = active_clone.load(Ordering::Relaxed);
                    if current != last {
                        last = current;
                        let _ = tray_handle
                            .set_tooltip(Some(format!("Game Time Tracker ({} active)", current)));
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_ui_data, add_game, remove_game])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // When the user clicks the "X" button, hide the window instead of killing the app
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    tauri_app.run(move |_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            // Clean shutdown for tracker
            should_stop_run.store(true, Ordering::SeqCst);
            let _ = stop_tx.send(());
        }
    });
}
