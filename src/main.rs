pub mod config;
pub mod models;
pub mod process;
pub mod store;
pub mod tracker;

use clap::{Parser, Subcommand};
use log::{error, info};
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
        }
        Some(Commands::Uninstall) => {
            info!("Uninstall command stubbed out.");
        }
        None => {
            info!("Starting game-time-tracker...");
            let mut tracker = AppTracker::new();
            if let Err(e) = tracker.run() {
                error!("Tracker stopped due to an error: {}", e);
            }
        }
    }

    Ok(())
}
