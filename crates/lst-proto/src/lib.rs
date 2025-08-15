use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Information about a document stored on the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub doc_id: Uuid,
    pub filename: String, // Encrypted filename
    pub updated_at: DateTime<Utc>,
}

/// Messages sent from the client to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Authenticate { jwt: String },
    RequestDocumentList,
    RequestSnapshot { doc_id: Uuid },
    PushChanges { doc_id: Uuid, device_id: String, changes: Vec<Vec<u8>> },
    PushSnapshot { doc_id: Uuid, filename: String, snapshot: Vec<u8> },
}

/// Messages sent from the server to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Authenticated { success: bool },
    DocumentList { documents: Vec<DocumentInfo> },
    Snapshot { doc_id: Uuid, filename: String, snapshot: Vec<u8> },
    NewChanges { doc_id: Uuid, from_device_id: String, changes: Vec<Vec<u8>> },
    RequestCompaction { doc_id: Uuid },
}
