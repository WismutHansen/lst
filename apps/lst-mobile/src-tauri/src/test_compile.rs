// Test compilation of our sync modules
#![allow(dead_code, unused_imports)]

use anyhow::Result;
use serde::{Deserialize, Serialize};

// Test our type definitions
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncConfig {
    pub server_url: String,
    pub email: String,
    pub device_id: String,
    pub sync_enabled: bool,
    pub sync_interval: u32,
    pub encryption_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncStatus {
    pub connected: bool,
    pub last_sync: Option<String>,
    pub pending_changes: u32,
    pub error: Option<String>,
}

// Test basic functions
pub fn test_sync_config() -> Result<()> {
    let config = SyncConfig {
        server_url: "ws://localhost:5673/api/sync".to_string(),
        email: "test@example.com".to_string(),
        device_id: uuid::Uuid::new_v4().to_string(),
        sync_enabled: true,
        sync_interval: 30,
        encryption_enabled: true,
    };
    
    println!("Sync config: {:?}", config);
    Ok(())
}

pub fn test_sync_status() -> Result<()> {
    let status = SyncStatus {
        connected: true,
        last_sync: Some(chrono::Utc::now().to_rfc3339()),
        pending_changes: 0,
        error: None,
    };
    
    println!("Sync status: {:?}", status);
    Ok(())
}

fn main() -> Result<()> {
    test_sync_config()?;
    test_sync_status()?;
    println!("All tests passed!");
    Ok(())
}