use anyhow::{Context, Result};
use notify::Event;
use std::collections::HashMap;
use tokio::time::Instant;

use crate::config::Config;
use lst_proto::SyncMessage;

pub struct SyncManager {
    config: Config,
    client: Option<reqwest::Client>,
    last_sync: Instant,
    pending_changes: HashMap<String, SyncMessage>,
}

impl SyncManager {
    pub async fn new(config: Config) -> Result<Self> {
        let client = if config.syncd.as_ref().and_then(|s| s.url.as_ref()).is_some() {
            Some(reqwest::Client::new())
        } else {
            None
        };
        
        // Ensure CRDT storage directory exists
        if let Some(ref storage) = config.storage {
            tokio::fs::create_dir_all(&storage.crdt_dir)
                .await
                .with_context(|| format!("Failed to create CRDT directory: {}", storage.crdt_dir.display()))?;
        }
        
        Ok(Self {
            config,
            client,
            last_sync: Instant::now(),
            pending_changes: HashMap::new(),
        })
    }
    
    pub async fn handle_file_event(&mut self, event: Event) -> Result<()> {
        // TODO: Process file changes into CRDT operations
        // TODO: Store changes locally
        // TODO: Queue for remote sync if server is configured
        
        for path in event.paths {
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    // Skip temporary files and hidden files
                    if filename_str.starts_with('.') || 
                       filename_str.ends_with(".tmp") || 
                       filename_str.ends_with(".swp") {
                        continue;
                    }
                }
            }
            
            println!("Processing file change: {}", path.display());
            
            // TODO: Convert file changes to CRDT operations
            // TODO: Encrypt CRDT data
            // TODO: Store in local CRDT storage
        }
        
        Ok(())
    }
    
    pub async fn periodic_sync(&self) -> Result<()> {
        if let Some(ref _client) = self.client {
            if let Some(ref syncd) = self.config.syncd {
                if let Some(ref server_url) = syncd.url {
                    // TODO: Sync pending changes to server
                    // TODO: Fetch remote changes from server
                    // TODO: Merge remote changes with local state
                    
                    println!("Would sync {} pending changes to {}", 
                        self.pending_changes.len(), server_url);
                }
            }
        }
        
        Ok(())
    }
}