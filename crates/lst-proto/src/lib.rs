use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub payload: SyncPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncPayload {
    ListUpdate(ListUpdate),
    NoteUpdate(NoteUpdate),
    PostUpdate(PostUpdate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListUpdate {
    pub list_name: String,
    pub operation: ListOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ListOperation {
    Add { item: String },
    Remove { item_id: Uuid },
    Complete { item_id: Uuid },
    Uncomplete { item_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteUpdate {
    pub note_id: String,
    pub content: String,
    pub operation: NoteOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoteOperation {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostUpdate {
    pub post_id: String,
    pub content: String,
    pub operation: PostOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PostOperation {
    Create,
    Update,
    Delete,
    Publish,
    Unpublish,
}