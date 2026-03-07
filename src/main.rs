#![windows_subsystem = "windows"]

pub mod config;
pub mod icon;
pub mod models;
pub mod process;
pub mod store;
pub mod tracker;
pub mod tray;
pub mod ui;

use anyhow::Context;
use clap::{Parser, Subcommand};
use log::{error, info};
use std::env;
use std::sync::atomic::Ordering;
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

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();

    std::panic::set_hook(Box::new(|panic_info| {
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());
        error!("PANIC at {}: {}", location, msg);
    }));

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Install) => {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let path = r#"Software\Microsoft\Windows\CurrentVersion\Run"#;
            let key = hkcu
                .open_subkey_with_flags(path, KEY_SET_VALUE)
                .context("Failed to open registry key")?;
            let exe_path = env::current_exe().context("Failed to get current executable path")?;
            let exe_path_str = exe_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid executable path"))?;
            key.set_value("GameTimeTracker", &exe_path_str)
                .context("Failed to set registry value")?;
            info!("Successfully installed auto-start registry key.");
            Ok(())
        }
        Some(Commands::Uninstall) => {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let path = r#"Software\Microsoft\Windows\CurrentVersion\Run"#;
            let key = hkcu
                .open_subkey_with_flags(path, KEY_SET_VALUE)
                .context("Failed to open registry key")?;
            match key.delete_value("GameTimeTracker") {
                Ok(_) => info!("Successfully uninstalled auto-start registry key."),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    info!("Auto-start registry key not found, nothing to uninstall.")
                }
                Err(e) => return Err(e).context("Failed to delete registry value"),
            }
            Ok(())
        }
        None => {
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
                } else if process.name().eq_ignore_ascii_case("gtt.exe") || process.name().eq_ignore_ascii_case("game-time-tracker.exe") {
                    count += 1;
                }
            }
            if count > 1 {
                info!("Another instance is already running. Exiting.");
                return Ok(());
            }

            info!("Starting game-time-tracker with system tray...");

            let data_dir = config::data_dir();
            let _ = std::fs::remove_file(data_dir.join("app.lock"));

            let event_loop = tao::event_loop::EventLoopBuilder::new().build();

            let menu = muda::Menu::new();
            let manage_games_item = muda::MenuItem::new("Manage Games", true, None);
            let edit_sessions_item = muda::MenuItem::new("Edit Sessions", true, None);
            let open_data_item = muda::MenuItem::new("Open Data Folder", true, None);
            let quit_item = muda::MenuItem::new("Quit", true, None);

            menu.append_items(&[
                &manage_games_item,
                &edit_sessions_item,
                &open_data_item,
                &muda::PredefinedMenuItem::separator(),
                &quit_item,
            ])?;

            let tray_icon = tray::setup_tray(&menu).context("Failed to setup tray icon")?;

            let active_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

            let active_count_clone = active_count.clone();
            let should_stop_clone = should_stop.clone();
            std::thread::spawn(move || {
                let mut tracker = AppTracker::new(active_count_clone, should_stop_clone);
                if let Err(e) = tracker.run() {
                    error!("Tracker stopped due to an error: {}", e);
                }
                info!("Tracker thread exiting");
            });

            let menu_channel = muda::MenuEvent::receiver();
            let tray_channel = tray_icon::TrayIconEvent::receiver();

            let mut last_active_count = 0;
            let is_ui_open = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            
            let (ctx_tx, ctx_rx) = std::sync::mpsc::channel();
            ui::init_ui_thread(is_ui_open.clone(), ctx_tx);
            let egui_ctx = ctx_rx.recv().context("Failed to receive egui context from UI thread")?;

            let manage_games_id = manage_games_item.into_id();
            let edit_sessions_id = edit_sessions_item.into_id();
            let open_data_id = open_data_item.into_id();
            let quit_id = quit_item.into_id();
            let data_dir = config::data_dir();
            let sessions_path = data_dir.join("sessions.json");

            event_loop.run(move |_event, _, control_flow| {
                *control_flow = tao::event_loop::ControlFlow::Poll;

                let current_count = active_count.load(Ordering::Relaxed);
                if current_count != last_active_count {
                    last_active_count = current_count;
                    let tooltip = format!("Game Time Tracker ({} active)", current_count);
                    if let Err(e) = tray_icon.set_tooltip(Some(&tooltip)) {
                        error!("Failed to update tray tooltip: {}", e);
                    }
                }

                while let Ok(event) = menu_channel.try_recv() {
                    if event.id == quit_id {
                        let _ = std::fs::remove_file(data_dir.join("app.lock"));
                        should_stop.store(true, Ordering::SeqCst);
                        *control_flow = tao::event_loop::ControlFlow::Exit;
                    } else if event.id == open_data_id {
                        if let Err(e) = open::that(&data_dir) {
                            error!("Failed to open data folder: {}", e);
                        }
                    } else if event.id == edit_sessions_id {
                        if let Err(e) = open::that(&sessions_path) {
                            error!("Failed to open sessions.json: {}", e);
                        }
                    } else if event.id == manage_games_id {
                        is_ui_open.store(true, Ordering::SeqCst);
                        egui_ctx.request_repaint();
                    }
                }

                while let Ok(event) = tray_channel.try_recv() {
                    if let tray_icon::TrayIconEvent::DoubleClick { .. } = event {
                        is_ui_open.store(true, Ordering::SeqCst);
                        egui_ctx.request_repaint();
                    }
                }
            });
        }
    }
}
