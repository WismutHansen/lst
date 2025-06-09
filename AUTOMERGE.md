This guide focuses on the core logic within `lst-syncd`.

### Goal

`lst-syncd` will watch for changes in your Markdown files, convert them into Automerge documents, store them locally in SQLite, and sync changes with a remote server, enabling conflict-free collaboration across multiple devices.

---

### Step 1: Project Setup

First, add the necessary dependencies to your `lst-syncd/Cargo.toml`:

```toml
[dependencies]
# The core CRDT library
automerge = { version = "0.5.2", features = ["rusqlite"] }

# For the local database
rusqlite = { version = "0.31", features = ["bundled"] }

# For watching file system events
notify = "6.1"

# For handling asynchronous operations (like networking)
tokio = { version = "1", features = ["full"] }

# A simple HTTP client for server communication
reqwest = "0.12"

# For unique document IDs
uuid = { version = "1", features = ["v4"] }

# For serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

---

### Step 2: The Local Sync Database (`syncd.db`)

`lst-syncd` needs a local database to map files to Automerge documents and store their state. This is crucial for calculating changes efficiently without re-parsing files constantly.

Create a function to initialize the database:

```rust
// in lst-syncd/src/db.rs
use rusqlite::{Connection, Result};

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS documents (
            doc_id      TEXT PRIMARY KEY,
            file_path   TEXT NOT NULL UNIQUE,
            automerge_doc BLOB NOT NULL
        )",
        [],
    )?;
    Ok(())
}
```

---

### Step 3: Handling File Changes

When a file is created or modified, we need to update its corresponding Automerge document.

1.  **Read the file content.**
2.  **Load the existing Automerge doc** from `syncd.db`. If it doesn't exist, create a new one.
3.  **Apply the changes** to the Automerge document within a transaction.
4.  **Save the new document state** back to the database.

```rust
// in lst-syncd/src/sync.rs
use automerge::{Automerge, Change, ReadDoc, Transactable};
use rusqlite::Connection;
use std::path::Path;
use std::fs;

// A helper to load a document from the DB or create a new one
fn load_or_create_doc(conn: &Connection, path_str: &str) -> Automerge {
    conn.query_row(
        "SELECT automerge_doc FROM documents WHERE file_path = ?",
        [path_str],
        |row| {
            let doc_bytes: Vec<u8> = row.get(0)?;
            Ok(Automerge::load(&doc_bytes).expect("Failed to load doc"))
        },
    )
    .unwrap_or_else(|_| Automerge::new()) // Create new if not found
}

// This function is the core of local file processing
pub fn process_file_change(conn: &Connection, file_path: &Path) {
    let path_str = file_path.to_str().unwrap();
    println!("Processing change for: {}", path_str);

    // 1. Read the new content from the file
    let new_content = fs::read_to_string(file_path).unwrap_or_default();

    // 2. Load the old document state
    let mut doc = load_or_create_doc(conn, path_str);

    // Get the state *before* changes to calculate the diff later
    let old_heads = doc.get_heads();

    // 3. Apply changes in a transaction
    let mut tx = doc.transaction();
    // For a note, we can treat the whole content as a single text object.
    // 'update_text' is smart and calculates the diff for us.
    tx.put(automerge::ROOT, "content", "").unwrap(); // Ensure the field exists
    tx.update_text(automerge::ROOT, "content", &new_content).unwrap();
    tx.commit();

    // 4. Save the new, updated document state locally
    let doc_bytes = doc.save();
    let doc_id = get_or_create_doc_id(conn, path_str);

    conn.execute(
        "REPLACE INTO documents (doc_id, file_path, automerge_doc) VALUES (?, ?, ?)",
        (doc_id, path_str, doc_bytes),
    ).expect("Failed to save doc");

    // 5. Get only the new changes to send to the server
    let changes_to_send = doc.get_changes_added(&old_heads);

    // In a real app, this would be an async task
    // send_changes_to_server(doc_id, changes_to_send);
    println!("Generated {} changes to send to server.", changes_to_send.len());
}

// Simplified helper to get a document's UUID
fn get_or_create_doc_id(conn: &Connection, path_str: &str) -> String {
    // ... implementation to fetch or create and insert a UUID for the path ...
    // For this example, we'll just return a placeholder.
    conn.query_row(
        "SELECT doc_id FROM documents WHERE file_path = ?",
        [path_str],
        |row| row.get(0),
    ).unwrap_or_else(|_| uuid::Uuid::new_v4().to_string())
}
```

---

### Step 4: Syncing with the Server

Your `lst-syncd` daemon needs to periodically communicate with `lst-server`.

#### A. Sending Local Changes

The `changes_to_send` from the previous step is what you send to the server. These are compact, binary representations of _only what changed_.

```rust
// A sketch of the sending logic
async fn send_changes_to_server(doc_id: String, changes: Vec<Change>) {
    let client = reqwest::Client::new();
    let change_bytes: Vec<Vec<u8>> = changes.into_iter().map(|c| c.raw_bytes().to_vec()).collect();

    // This is a simplified API call. You would use a real API structure.
    let _res = client.post("https://your-lst-server.com/api/sync/push")
        .json(&serde_json::json!({
            "doc_id": doc_id,
            "changes": change_bytes,
        }))
        .send()
        .await;
    // Handle response...
}
```

#### B. Receiving and Applying Remote Changes

When you fetch changes from the server, you apply them to your local Automerge doc and then **overwrite the local file**. This closes the sync loop.

```rust
// A sketch of the receiving logic
pub fn fetch_and_apply_changes(conn: &Connection, doc_id: &str) {
    // 1. Fetch changes from the server (e.g., using reqwest)
    // let remote_changes_bytes: Vec<Vec<u8>> = fetch_from_server(doc_id).await;
    // let remote_changes = remote_changes_bytes.into_iter()
    //     .map(|bytes| Change::from_bytes(bytes).unwrap())
    //     .collect();

    // For the tutorial, we'll simulate receiving changes.
    let remote_changes: Vec<Change> = vec![]; // Replace with actual fetch

    if remote_changes.is_empty() { return; }

    // 2. Load the document and its path
    let (mut doc, file_path_str): (Automerge, String) = conn.query_row(
        "SELECT automerge_doc, file_path FROM documents WHERE doc_id = ?",
        [doc_id],
        |row| {
            let doc_bytes: Vec<u8> = row.get(0)?;
            let doc = Automerge::load(&doc_bytes).unwrap();
            Ok((doc, row.get(1)?))
        },
    ).expect("Document not found");

    // 3. Apply the changes
    doc.apply_changes(remote_changes).unwrap();

    // 4. Re-render the document to a string
    let text_value = doc.get(automerge::ROOT, "content").unwrap();
    if let Some((automerge::Value::Object(obj_id), _)) = text_value {
        if let Some(automerge::Value::Text(text)) = doc.get(obj_id, "") {
            let new_content = text.to_string();

            // 5. Overwrite the local file with the new merged content
            fs::write(&file_path_str, new_content).expect("Could not write file");
        }
    }

    // 6. Save the new state to the local DB
    conn.execute(
        "UPDATE documents SET automerge_doc = ? WHERE doc_id = ?",
        (doc.save(), doc_id),
    ).expect("Failed to save updated doc");

    println!("Applied remote changes to {}", file_path_str);
}
```

---

### Step 5: The Server (`lst-server`)

The server's job is simpler: it's a hub that stores and relays changes. It doesn't need to understand the content of the Automerge documents, only the changes.

**Server DB Schema (`content.db`):**

```sql
CREATE TABLE IF NOT EXISTS documents (
    doc_id TEXT PRIMARY KEY,
    -- Other metadata like user_id, etc.
);

CREATE TABLE IF NOT EXISTS changes (
    change_id INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id TEXT NOT NULL,
    change_data BLOB NOT NULL,
    -- Metadata like device_id, timestamp
    FOREIGN KEY (doc_id) REFERENCES documents (doc_id)
);
```

**Server Logic:**

1.  **Endpoint `/api/sync/push`:**
    - Receives `{ "doc_id": "...", "changes": [...] }`.
    - For each `change` in `changes`, insert it into the `changes` table.
    - Broadcast these new changes to other connected clients (e.g., via WebSockets).
2.  **Endpoint `/api/sync/pull`:**
    - Receives a request like `{ "doc_id": "...", "since_change_id": 123 }`.
    - Responds with all changes for that `doc_id` where `change_id > 123`.

This design ensures `lst-server` is a simple, scalable relay. For true local-first operation, you would add client-side encryption before sending changes to the server.

---

### Putting It All Together in `lst-syncd`

Your `main` function in `lst-syncd` would look something like this:

1.  Initialize the database connection.
2.  Set up `notify` to watch the content directory.
3.  In the file watcher event handler, call `process_file_change`.
4.  In a separate `tokio` task, periodically call a function that loops through all your document IDs and runs `fetch_and_apply_changes`.
