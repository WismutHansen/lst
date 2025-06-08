### Overall Plan

The core strategy is to first build the new sync engine in `lst-syncd` and the encrypted relay in `lst-server`. Once the backend is functional, the `lst-cli` sync commands will be updated to manage the new system.

---

### Phase 1: Foundational Backend Changes (`lst-syncd` & `lst-server`)

#### ✅ `lst-syncd` (Client Sync Engine)

- [ ] **Dependencies**: Add `automerge` and `rusqlite` to `lst-syncd/Cargo.toml`.
- [ ] **Local Database (`syncd.db`)**:
  - [ ] Implement a new module to manage a local SQLite database (`syncd.db`).
  - [ ] Define the schema: a `documents` table mapping file paths to Automerge document states (`automerge_state` BLOB).
- [ ] **Automerge Integration**:
  - [ ] Create logic to parse a Markdown file into a structured `automerge::Map`.
  - [ ] Implement the reverse: render an `automerge::Map` back into a Markdown file.
  - [ ] Refactor `watcher.rs` to trigger this conversion on file changes, calculate `automerge::Change`s, and update the local `automerge_state`.
- [ ] **Client-Side Encryption**:
  - [ ] Add a crypto library (e.g., `ring` or a higher-level crate for XChaCha20).
  - [ ] Implement `encrypt(data, key)` and `decrypt(data, key)` functions.
  - [ ] The master encryption key should be loaded securely (placeholder for now, to be integrated with `lst sync setup`).
- [ ] **WebSocket Client**:
  - [ ] Implement logic to connect to the server's `/api/sync` WebSocket endpoint.
  - [ ] Implement the client-side sync protocol: sending encrypted changes and processing incoming ones.

#### ✅ `lst-server` (Encrypted Relay)

- [ ] **API Refactoring**:
  - [ ] **Deprecate/Remove** the existing REST content API at `/api/content`. The handlers (`create_content_handler`, etc.) and their routes should be removed.
  - [ ] Add a new WebSocket endpoint at `/api/sync`.
- [ ] **Server Database (`content.db`)**:
  - [ ] **Modify Schema**: Change the `content` table to store `doc_id` (UUID), `user_id`, and `encrypted_snapshot` (BLOB).
  - [ ] Add a new `document_changes` table to log incoming encrypted change sets (`doc_id`, `device_id`, `encrypted_change` BLOB).
- [ ] **WebSocket Handler**:
  - [ ] Implement handler to manage WebSocket connections.
  - [ ] Authenticate connections using the client's JWT.
  - [ ] Implement the server-side sync protocol:
    - Receive encrypted changes and store them in `document_changes`.
    - Broadcast encrypted changes to other connected devices for that user.
    - Handle snapshot requests and compaction logic.

### Phase 2: Updating Shared Code & Configuration

#### ✅ `lst-proto` (Shared Types)

- [ ] **Remove Old Types**: Deprecate or remove the old `SyncMessage` and `SyncPayload` structs.
- [ ] **Add New Types**: Define new structs/enums for the WebSocket protocol (e.g., `ClientMessage::PushChanges`, `ServerMessage::NewChanges`).

#### ✅ `lst-cli` (Configuration)

- [ ] **Modify `config.rs`**:
  - [ ] Update the `Config` and `SyncdConfig` structs to match the new spec.
  - [ ] Add fields for `syncd.database_path` and `syncd.encryption_key_ref`.
  - [ ] Update the default config generation to reflect these changes.

### Phase 3: Updating the User-Facing CLI

#### ✅ `lst-cli` (Commands)

- [ ] **`lst sync setup`**:
  - [ ] This is the most critical CLI change.
  - [ ] Rework the command to guide the user through the login flow to get a JWT.
  - [ ] **Add logic to generate a master encryption key.**
  - [ ] **Implement secure storage for this key** (using a crate like `keyring` for OS credential managers).
  - [ ] Save the server URL, JWT, and key reference to `lst.toml`.
- [ ] **`lst sync start`**:
  - [ ] Modify the command to launch the newly implemented `lst-syncd` daemon.
- [ ] **`lst sync status / stop / logs`**:
  - [ ] Update these commands to correctly interact with the new daemon process.
- [ ] **No Changes Needed For**: `ls`, `add`, `done`, `note`, `dl`, etc. These should continue to work on local Markdown files, with `lst-syncd` handling the sync transparently in the background
