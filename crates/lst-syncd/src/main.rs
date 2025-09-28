mod config;
mod sync;
mod watcher;
mod database;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use lst_cli::storage;

use crate::config::{load_syncd_config};
use crate::sync::SyncManager;
use crate::watcher::FileWatcher;

#[derive(Parser)]
#[command(name = "lst-syncd", about = "Background sync daemon for lst")]
struct Args {
    /// Path to sync daemon configuration file
    #[arg(long, default_value = "~/.config/lst/config.toml")]
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
        
        // Get content directory with proper path expansion
        eprintln!("DEBUG: About to call storage::get_content_dir()");
        let content_dir = storage::get_content_dir()?;
        eprintln!("DEBUG: storage::get_content_dir() returned: {}", content_dir.display());
        println!(
            "Watching content directory: {}",
            content_dir.display()
        );
        if let Some(ref sync) = config.sync {
            if let Some(ref server_url) = sync.server_url {
                println!("Syncing to server: {}", server_url);
            } else {
                println!("No server configured - running in local-only mode");
            }
        } else {
            println!("No sync configuration found - running in local-only mode");
        }
    }

    // Initialize file watcher
    eprintln!("DEBUG: About to call storage::get_content_dir() for watcher");
    let content_dir = storage::get_content_dir()?;
    eprintln!("DEBUG: storage::get_content_dir() for watcher returned: {}", content_dir.display());
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

