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
    /// Install the tracker as a background service/startup process
    Install,
    /// Uninstall the tracker
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

fn handle_cli(cli: Cli) -> Option<()> {
    match &cli.command {
        Some(Commands::Install) => {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let path = r#"Software\Microsoft\Windows\CurrentVersion\Run"#;
            if let Ok(key) = hkcu.open_subkey_with_flags(path, KEY_SET_VALUE) {
                if let Ok(exe) = env::current_exe() {
                    if let Some(exe_str) = exe.to_str() {
                        let _ = key.set_value("GameTimeTracker", &exe_str);
                        println!("Successfully installed auto-start registry key.");
                    }
                }
            }
            Some(())
        }
        Some(Commands::Uninstall) => {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let path = r#"Software\Microsoft\Windows\CurrentVersion\Run"#;
            if let Ok(key) = hkcu.open_subkey_with_flags(path, KEY_SET_VALUE) {
                let _ = key.delete_value("GameTimeTracker");
                println!("Successfully uninstalled auto-start registry key.");
            }
            Some(())
        }
        None => None,
    }
}

fn check_single_instance() -> bool {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let current_exe = std::env::current_exe().ok();
    let mut count = 0;
    for process in sys.processes().values() {
        if let Some(exe) = current_exe.as_ref() {
            if let Some(process_exe) = process.exe() {
                if process_exe == exe {
                    count += 1;
                }
            }
        } else if process.name().eq_ignore_ascii_case("gtt.exe")
            || process.name().eq_ignore_ascii_case("game-time-tracker.exe")
        {
            count += 1;
        }
    }
    count > 1
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();

    let cli = Cli::parse();
    if handle_cli(cli).is_some() {
        return;
    }

    if check_single_instance() {
        println!("Another instance is already running. Exiting.");
        return;
    }

    info!("Starting Game Time Tracker daemon mapped to Tauri.");

    let active_count = Arc::new(AtomicUsize::new(0));
    let should_stop = Arc::new(AtomicBool::new(false));
    let should_stop_run = should_stop.clone();
    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();

    let tauri_app = tauri::Builder::default()
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
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::DoubleClick { .. } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
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
                let mut tracker = AppTracker::new(active_count_tracker, should_stop_tracker, stop_rx);
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
                        let _ = tray_handle.set_tooltip(Some(format!(
                            "Game Time Tracker ({} active)",
                            current
                        )));
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_ui_data,
            add_game,
            remove_game
        ])
        .on_window_event(|window, event| match event {
            // When the user clicks the "X" button, hide the window instead of killing the app
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let _ = window.hide();
                api.prevent_close();
            }
            _ => {}
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
