# `lst` - Specification v0.5

## 1. Guiding Principles

| Principle                   | Manifestation                                                                                                     |
| :-------------------------- | :---------------------------------------------------------------------------------------------------------------- |
| **Plain-Text Primary**      | All user content is created and edited as local Markdown files. The CLI is the ground truth for user interaction. |
| **Local-First Sync**        | The application works fully offline. Synchronization via `lst-syncd` is an optional, opportunistic layer.         |
| **Zero-Knowledge Server**   | User content is client-side encrypted before sync. The server stores and relays opaque blobs it cannot read.      |
| **One Core, Many Surfaces** | A single Rust core powers multiple clients: a fast CLI, a future GUI, mobile apps, and other integrations.        |
| **Self-Hosted by Default**  | The entire stack is designed to be easily run on personal infrastructure, like a home server or a small VPS.      |

---

## 2. High-Level Architecture

The `lst` ecosystem is composed of three main components that work together to provide a seamless plain-text to multi-device experience.

```mermaid
graph TD
    subgraph User Interaction
        A[lst CLI]
        B[Tauri GUI (Future)]
    end

    subgraph Client Machine
        A -- Modifies/Reads --> C{Markdown Files};
        B -- Modifies/Reads --> C;

        D[lst-syncd] -- Watches --> C;
        C -- Notifies --> D;

        D <--> E[syncd.db (SQLite)];
        D -- Encrypts/Decrypts --> F{Automerge CRDTs};
        F -- Generates/Applies Changes --> D;
    end

    subgraph Network
        D -- WebSocket (TLS) --> G[lst-server];
    end

    subgraph Server (LXC/VM)
        G -- Relays Encrypted Blobs --> G;
        G <--> H[content.db (SQLite)];
        G <--> I[tokens.db (SQLite)];
    end

    A: `lst` CLI is the primary interface for users to edit Markdown files.
    D: `lst-syncd` runs in the background, detects file changes, converts them to an Automerge CRDT format, encrypts them, and syncs with the server. It also applies encrypted changes from the server back to the local Markdown files.
    E: `syncd.db` is a local SQLite database that maps files to CRDT documents and tracks sync state.
    G: `lst-server` is the central relay and persistence layer. It handles authentication and stores/relays encrypted data blobs without ever decrypting them.
    H/I: The server uses SQLite to store authentication tokens and the encrypted user content.
```

---

## 3. Storage & Sync Model

This model is designed to bridge the plain-text file system with a robust, conflict-free, multi-device synchronization system.

### 3.1 `lst-cli` & Plain-Text Files

The user-facing experience is centered on Markdown files in the `content` directory, as configured in `lst.toml`. `lst-cli` is responsible for creating, reading, and modifying these files. This layer is intentionally unaware of the sync mechanism.

- **File Formats**: As described in `README.md`, content is structured Markdown with YAML frontmatter.
- **Organization**: Users can organize files into subdirectories (e.g., `lists/groceries/pharmacy.md`).

### 3.2 `lst-syncd`: The Sync Engine

`lst-syncd` is the background daemon responsible for synchronization. It acts as the bridge between the file system and the Automerge CRDT model.

#### Local SQLite Database (`syncd.db`)

`lst-syncd` maintains a local SQLite database to manage sync state.

- **Location**: `~/.config/lst/syncd.db`
- **Schema**:
  - `documents` table:
    - `doc_id` (UUID, Primary Key): A unique ID for the Automerge document.
    - `file_path` (TEXT, UNIQUE): The relative path to the Markdown file (e.g., `lists/groceries.md`).
    - `doc_type` (TEXT): The type of document, e.g., 'list', 'note'.
    - `last_sync_hash` (TEXT): The hash of the file content at the last successful sync, to avoid reprocessing unchanged files.
    - `automerge_state` (BLOB): The full, unencrypted Automerge document state. This allows for efficient change calculation without re-parsing the file every time.

#### Sync Lifecycle

1.  **Initial Scan**: On first start, `lst-syncd` scans the content directory. For each Markdown file, it creates an Automerge document, generates a `doc_id`, and populates the `syncd.db`.
2.  **File Watching**: `lst-syncd` watches the content directory for changes.
    - When a file is modified, it compares the new content hash with `last_sync_hash`.
    - If different, it loads the corresponding `automerge_state` from its database.
    - It computes the changes required to bring the Automerge document in sync with the new file content. For lists, this should be a line-by-line diff. for notes, it can be a full text replacement.
    - It generates an Automerge change set (`Vec<u8>`).
3.  **Applying Remote Changes**:
    - When `lst-syncd` receives an encrypted change set from the server, it decrypts it.
    - It loads the relevant document from `automerge_state` in `syncd.db`.
    - It applies the Automerge changes.
    - It re-renders the Automerge document back into Markdown format and overwrites the local file.
    - It updates its local `automerge_state` and `last_sync_hash`.

### 3.3 Client-Side Encryption

Privacy is paramount. All content is encrypted on the client before being transmitted.

- **Algorithm**: **XChaCha20-Poly1305** is recommended for its performance and security.
- **Key Management**: A master encryption key will be required by `lst-syncd`. Initially, this can be stored in the system's secure credential manager (e.g., macOS Keychain, Freedesktop Secret Service). The key should be generated on first run of `lst sync setup`.
- **Process**:
  1.  `lst-syncd` generates an Automerge change set (`Vec<u8>`).
  2.  This change set is encrypted using the master key.
  3.  The resulting ciphertext (opaque blob) is sent to `lst-server`.
  4.  The reverse process happens for incoming changes.

### 3.4 `lst-server`: The Encrypted Relay

The server's role is intentionally limited to authentication and acting as a dumb, reliable relay for encrypted data. **It never holds decryption keys and cannot read user content.**

#### Server SQLite Database (`content.db`)

- **Location**: Managed by the server, typically in a data directory alongside `tokens.db`.
- **Schema**:
  - `documents` table:
    - `doc_id` (UUID, Primary Key): The same ID used by the client.
    - `user_id` (TEXT): The user's email or a unique ID.
    - `encrypted_snapshot` (BLOB): The full, encrypted Automerge document. This serves as the "source of truth" for new devices joining the sync network.
    - `updated_at` (TIMESTAMP).
  - `document_changes` table:
    - `change_id` (INTEGER, Primary Key).
    - `doc_id` (UUID, Foreign Key).
    - `device_id` (TEXT): The ID of the device that sent the change.
    - `encrypted_change` (BLOB): An individual encrypted Automerge change set.
    - `created_at` (TIMESTAMP).

#### Sync Protocol (WebSocket)

The sync protocol is designed around exchanging encrypted Automerge changes.

1.  **Connection**: A client (`lst-syncd`) establishes a WebSocket connection to the server and authenticates using its JWT.
2.  **Initial Sync (for a new device)**:
    - The client requests the full list of `doc_id`s for its user.
    - For each `doc_id`, it requests the `encrypted_snapshot` from the server.
    - It decrypts the snapshot, reconstructs the Automerge document, and saves it to its local `syncd.db` and writes the initial Markdown file.
3.  **Sending Changes**:
    - `lst-syncd` sends a message: `PushChanges { doc_id: Uuid, device_id: String, changes: Vec<Vec<u8>> }`.
    - `changes` is a list of encrypted Automerge change sets.
    - The server saves these changes to its `document_changes` table and broadcasts a `NewChanges` message to all other connected devices for that user.
4.  **Receiving Changes**:
    - Clients receive `NewChanges { doc_id: Uuid, changes: Vec<Vec<u8>> }`.
    - They decrypt and apply the changes locally, updating their Markdown file and `syncd.db`.
5.  **Compaction**:
    - To prevent the `document_changes` log from growing infinitely, the server will periodically request a compaction.
    - Server sends a `RequestCompaction { doc_id: Uuid }` message to one of its connected clients.
    - The client loads its local Automerge document, saves a new full snapshot, encrypts it, and sends it to the server in a `PushSnapshot { doc_id: Uuid, snapshot: Vec<u8> }` message.
    - The server replaces its `encrypted_snapshot` in the `documents` table and deletes all entries from `document_changes` for that `doc_id`.

---

## 4. API Specification

The existing REST API for content (`/api/content`) will be **deprecated and replaced** by the WebSocket-based sync protocol. The authentication endpoints remain unchanged.

### 4.1 Authentication API (REST)

- `POST /api/auth/request`: Unchanged. Requests a one-time login token via email.
- `POST /api/auth/verify`: Unchanged. Verifies the token and returns a long-lived JWT.

### 4.2 Sync API (WebSocket)

- **Endpoint**: `/api/sync`
- **Protocol**: All messages are JSON-encoded.

#### Client-to-Server Messages

- `Authenticate { jwt: String }`: Sent immediately after connection.
- `RequestDocumentList`: Asks the server for all `doc_id`s and their `updated_at` timestamps for the user.
- `RequestSnapshot { doc_id: Uuid }`: Asks for the full encrypted snapshot of a document.
- `PushChanges { doc_id: Uuid, device_id: String, changes: Vec<Vec<u8>> }`: Pushes one or more encrypted change sets.
- `PushSnapshot { doc_id: Uuid, snapshot: Vec<u8> }`: Responds to a compaction request with a new full encrypted snapshot.

#### Server-to-Client Messages

- `Authenticated { success: bool }`: Confirms authentication status.
- `DocumentList { documents: Vec<{doc_id: Uuid, updated_at: Timestamp}> }`: Response to `RequestDocumentList`.
- `Snapshot { doc_id: Uuid, snapshot: Vec<u8> }`: Response to `RequestSnapshot`.
- `NewChanges { doc_id: Uuid, from_device_id: String, changes: Vec<Vec<u8>> }`: Broadcasts new changes to other clients.
- `RequestCompaction { doc_id: Uuid }`: Asks the client to generate and push a new snapshot.

---

## 5. CLI Specification

The CLI interface remains largely the same, focusing on user-friendly interaction with local Markdown files.

- **List Management**: `lst ls`, `add`, `done`, `undone`, `rm`, `pipe`, `dl`.
- **Note Management**: `lst note new`, `add`, `open`, `rm`, `ls`, `dn`.
- **Sync Management**:
  - `lst sync setup`: Guides the user through logging in and generating the client-side encryption key.
  - `lst sync start/stop/status`: Manages the `lst-syncd` daemon process.

---

## 6. Configuration (`lst.toml`)

The unified configuration file will be updated to include sync and encryption settings.

```toml
# ... [server], [ui], [fuzzy] sections remain ...

[paths]
# Base directory for all CLI content (lists, notes, etc.)
content_dir = "~/Documents/lst"

[syncd]
# URL of the sync server's WebSocket endpoint (e.g., wss://lists.example.com/api/sync)
url = "wss://lists.example.com/api/sync"

# JWT auth token, obtained via 'lst sync setup' and stored automatically
auth_token = "your-jwt-token"

# Unique ID for this device, auto-generated
device_id = "auto-generated-uuid"

# Path to the sync daemon's local database
database_path = "~/.config/lst/syncd.db"

# Reference to the encryption key in the system's credential manager
encryption_key_ref = "lst-master-key"
```

---

## 7. Roadmap

This roadmap focuses on implementing the described sync architecture.

| Phase                                     | Duration | Key Deliverables                                                                                                                                                                                                                                                                                             |
| :---------------------------------------- | :------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1: CRDT & Encryption Foundation** | 4 Weeks  | 1. Integrate `automerge` crate into `lst-syncd`.<br>2. Implement client-side encryption/decryption logic (XChaCha20).<br>3. Implement `syncd.db` (SQLite) for state management.<br>4. Develop logic to convert Markdown file changes to Automerge changes and vice-versa.                                    |
| **Phase 2: Server & Sync Protocol**       | 3 Weeks  | 1. Implement WebSocket endpoint on `lst-server`.<br>2. Implement server-side logic for relaying and storing encrypted blobs in `content.db`.<br>3. Implement full client-server sync protocol (push/pull changes, compaction).<br>4. Refine `lst-proto` with the new WebSocket message types.                |
| **Phase 3: CLI Integration & Hardening**  | 2 Weeks  | 1. Update `lst sync setup` to handle encryption key generation and storage.<br>2. Improve `lst sync start/stop/status` to be more robust.<br>3. Add comprehensive unit and integration tests for the entire sync pipeline.<br>4. Document the new sync and encryption architecture for users and developers. |
| **Phase 4: Future Features**              | Ongoing  | 1. Build Tauri GUI and mobile clients that leverage the `lst-syncd` logic.<br>2. Implement `share` command for multi-user collaboration (will require a key exchange mechanism like Sealed Boxes).<br>3. Add support for `posts` and `media` to the sync engine.                                             |
