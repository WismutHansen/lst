mod config;
mod crypto;
mod sync;
mod watcher;
mod database;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::config::{load_syncd_config};
use crate::sync::SyncManager;
use crate::watcher::FileWatcher;

#[derive(Parser)]
#[command(name = "lst-syncd", about = "Background sync daemon for lst")]
struct Args {
    /// Path to sync daemon configuration file
    #[arg(long, default_value = "~/.config/lst/lst.toml")]
    config: String,

    /// Run in foreground mode (don't daemonize)
    #[arg(long)]
    foreground: bool,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Expand config path
    let config_path = if args.config.starts_with("~/") {
        dirs::home_dir().unwrap().join(&args.config[2..])
    } else {
        PathBuf::from(args.config)
    };

    // Load configuration
    let config = load_syncd_config(&config_path)?;

    if args.verbose {
        println!("lst-syncd starting with config: {}", config_path.display());
        println!(
            "Watching content directory: {}",
            config.get_content_dir().display()
        );
        if let Some(ref syncd) = config.syncd {
            if let Some(ref server_url) = syncd.url {
                println!("Syncing to server: {}", server_url);
            } else {
                println!("No server configured - running in local-only mode");
            }
        } else {
            println!("No sync daemon configuration found - running in local-only mode");
        }
    }

    // Initialize file watcher
    let content_dir = config.get_content_dir();
    let mut watcher = FileWatcher::new(&content_dir)?;

    // Initialize sync manager
    let mut sync_manager = SyncManager::new(config.clone()).await?;

    if !args.foreground {
        println!("lst-syncd daemon started");
        // TODO: Daemonize process (platform-specific)
    }

    // Main event loop
    loop {
        tokio::select! {
            // Handle file system events
            event = watcher.next_event() => {
                if let Some(event) = event {
                    if args.verbose {
                        println!("File event: {:?}", event);
                    }
                    sync_manager.handle_file_event(event).await?;
                }
            }

            // Periodic sync check (every 30 seconds)
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                if args.verbose {
                    println!("Performing periodic sync check");
                }
                sync_manager.periodic_sync().await?;
            }

            // Handle shutdown signals
            _ = tokio::signal::ctrl_c() => {
                println!("Received shutdown signal, stopping lst-syncd");
                break;
            }
        }
    }

    Ok(())
}

