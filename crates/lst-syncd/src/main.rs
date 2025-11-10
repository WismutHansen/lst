mod config;
mod database;
mod sync;
mod trigger;
mod watcher;

use anyhow::Result;
use clap::Parser;
use lst_cli::storage;
use std::path::PathBuf;

use crate::config::load_syncd_config;
use crate::sync::{run_migrations, SyncManager, SyncReason};
use crate::trigger::{ServerTrigger, TriggerEvent};
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

    /// Run database migrations and exit
    #[arg(long)]
    migrate_only: bool,
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

    if args.migrate_only {
        run_migrations()?;
        println!("lst-syncd migrations completed");
        return Ok(());
    }

    if args.verbose {
        println!("lst-syncd starting with config: {}", config_path.display());

        // Get content directory with proper path expansion
        eprintln!("DEBUG: About to call storage::get_content_dir()");
        let content_dir = storage::get_content_dir()?;
        eprintln!(
            "DEBUG: storage::get_content_dir() returned: {}",
            content_dir.display()
        );
        println!("Watching content directory: {}", content_dir.display());
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
    eprintln!(
        "DEBUG: storage::get_content_dir() for watcher returned: {}",
        content_dir.display()
    );
    let mut watcher = FileWatcher::new(&content_dir)?;

    // Initialize sync manager
    let mut sync_manager = SyncManager::new(config.clone()).await?;
    if sync_manager.has_server() {
        sync_manager.sync_now(SyncReason::Startup).await?;
    }

    let mut trigger = ServerTrigger::spawn(&config, &sync_manager.state_snapshot());

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
                    sync_manager.sync_now(SyncReason::LocalChange).await?;
                }
            }

            trigger_event = async {
                match trigger.as_mut() {
                    Some(t) => t.recv().await,
                    None => None,
                }
            }, if trigger.is_some() => {
                match trigger_event {
                    Some(TriggerEvent::RemoteChange) => {
                        if args.verbose {
                            println!("Remote change trigger received");
                        }
                        if let Err(e) = sync_manager.sync_now(SyncReason::RemoteTrigger).await {
                            eprintln!("Remote-triggered sync failed: {e}");
                        }
                    }
                    None => {
                        // Channel closed; attempt to respawn the trigger listener
                        trigger = ServerTrigger::spawn(&config, &sync_manager.state_snapshot());
                    }
                }
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
