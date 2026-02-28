pub mod config;
pub mod models;
pub mod process;
pub mod store;
pub mod tracker;
pub mod tray;

use clap::{Parser, Subcommand};
use log::{error, info};
use std::sync::atomic::Ordering;
use tracker::AppTracker;

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
    env_logger::init();

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Install) => {
            info!("Install command stubbed out.");
            Ok(())
        }
        Some(Commands::Uninstall) => {
            info!("Uninstall command stubbed out.");
            Ok(())
        }
        None => {
            info!("Starting game-time-tracker with system tray...");

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
            ])
            .unwrap();

            let tray_icon = tray::setup_tray(&menu).expect("Failed to setup tray icon");

            let active_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

            let active_count_clone = active_count.clone();
            std::thread::spawn(move || {
                let mut tracker = AppTracker::new(active_count_clone);
                if let Err(e) = tracker.run() {
                    error!("Tracker stopped due to an error: {}", e);
                }
            });

            let menu_channel = muda::MenuEvent::receiver();
            let tray_channel = tray_icon::TrayIconEvent::receiver();

            let mut last_active_count = 0;

            event_loop.run(move |_event, _, control_flow| {
                *control_flow = tao::event_loop::ControlFlow::WaitUntil(
                    std::time::Instant::now() + std::time::Duration::from_millis(500),
                );

                let current_count = active_count.load(Ordering::Relaxed);
                if current_count != last_active_count {
                    last_active_count = current_count;
                    let tooltip = format!("Game Time Tracker ({} active)", current_count);
                    if let Err(e) = tray_icon.set_tooltip(Some(&tooltip)) {
                        error!("Failed to update tray tooltip: {}", e);
                    }
                }

                if let Ok(event) = menu_channel.try_recv() {
                    if event.id == quit_item.id() {
                        *control_flow = tao::event_loop::ControlFlow::Exit;
                    } else if event.id == open_data_item.id() {
                        if let Err(e) = open::that(config::data_dir()) {
                            error!("Failed to open data folder: {}", e);
                        }
                    } else if event.id == edit_sessions_item.id() {
                        if let Err(e) = open::that(config::data_dir().join("sessions.json")) {
                            error!("Failed to open sessions.json: {}", e);
                        }
                    } else if event.id == manage_games_item.id() {
                        info!("Manage Games clicked - functionality not yet implemented.");
                    }
                }

                if let Ok(event) = tray_channel.try_recv() {
                    info!("Tray icon event: {:?}", event);
                }
            });
        }
    }
}
