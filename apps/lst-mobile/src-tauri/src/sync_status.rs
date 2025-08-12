use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatusInfo {
    pub connected: bool,
    pub last_sync: Option<String>,
    pub pending_changes: u32,
    pub error: Option<String>,
}

use lazy_static::lazy_static;

lazy_static! {
    static ref SYNC_STATUS: Arc<Mutex<SyncStatusInfo>> = Arc::new(Mutex::new(SyncStatusInfo {
        connected: false,
        last_sync: None,
        pending_changes: 0,
        error: None,
    }));
    static ref LAST_STATUS_UPDATE: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));
}

/// Update sync status
pub fn update_sync_status(
    connected: bool,
    pending_changes: u32,
    error: Option<String>,
) -> Result<()> {
    let mut status = SYNC_STATUS.lock().unwrap();
    let mut last_update = LAST_STATUS_UPDATE.lock().unwrap();
    
    status.connected = connected;
    status.pending_changes = pending_changes;
    status.error = error;
    
    if connected {
        status.last_sync = Some(chrono::Utc::now().to_rfc3339());
    }
    
    *last_update = Instant::now();
    
    Ok(())
}

/// Get current sync status
pub fn get_sync_status() -> Result<SyncStatusInfo> {
    let config = crate::auth::get_current_config();
    let status = SYNC_STATUS.lock().unwrap();
    
    // Check if sync is enabled
    let sync_enabled = config.syncd
        .as_ref()
        .and_then(|s| s.url.as_ref())
        .map(|url| !url.is_empty())
        .unwrap_or(false) && config.is_jwt_valid();
    
    if !sync_enabled {
        return Ok(SyncStatusInfo {
            connected: false,
            last_sync: None,
            pending_changes: 0,
            error: Some("Sync not configured".to_string()),
        });
    }
    
    // Check if status is stale (older than 2 minutes)
    let last_update = LAST_STATUS_UPDATE.lock().unwrap();
    let is_stale = last_update.elapsed() > Duration::from_secs(120);
    
    if is_stale {
        return Ok(SyncStatusInfo {
            connected: false,
            last_sync: status.last_sync.clone(),
            pending_changes: status.pending_changes,
            error: Some("Status outdated".to_string()),
        });
    }
    
    Ok(status.clone())
}

/// Mark sync as connected
pub fn mark_sync_connected() -> Result<()> {
    update_sync_status(true, 0, None)
}

/// Mark sync as disconnected with error
pub fn mark_sync_disconnected(error: String) -> Result<()> {
    update_sync_status(false, 0, Some(error))
}

/// Update pending changes count
pub fn update_pending_changes(count: u32) -> Result<()> {
    let status = SYNC_STATUS.lock().unwrap();
    let connected = status.connected;
    let error = status.error.clone();
    drop(status);
    
    update_sync_status(connected, count, error)
}