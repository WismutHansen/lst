# TODO List for lst Project

## Core Infrastructure

- [x] Set up Rust project structure with Cargo
- [x] Create basic command-line interface structure
- [x] Implement core storage model for content directories
- [x] Create file format parsers for lists
- [x] Implement anchor generation and tracking
- [x] Convert project to a Cargo workspace
- [x] Create `lst-proto` crate for shared wire types
- [ ] **lst-syncd (Client-side Sync Daemon):**
  - [x] Scaffold `lst-syncd` daemon with file watching
  - [ ] **Integrate `automerge` crate for CRDT-based list and note synchronization:**
    - [x] Add `automerge` (with `rusqlite` feature), `rusqlite` (for `syncd.db`), and `uuid` dependencies to `lst-syncd/Cargo.toml`.
    - [x] **Implement `syncd.db` (SQLite) for local Automerge state management (`lst-syncd/src/database.rs` or similar):**
      - [x] Define `documents` table schema: `doc_id` (UUID PK), `file_path` (TEXT UNIQUE), `doc_type` (TEXT, e.g., 'list', 'note'), `last_sync_hash` (TEXT), `automerge_state` (BLOB for the full Automerge document), `owner` (TEXT), `writers` (TEXT), `readers` (TEXT).
      - [x] Implement function to initialize the database and table.
    - [x] **Develop logic for processing local file changes into Automerge documents (`lst-syncd/src/sync.rs` or similar):**
      - [x] On file change, read content and compare its hash with `last_sync_hash` from `syncd.db`.
      - [x] If different, load `automerge_state` for the file. If no state, create a new `Automerge` document.
      - [ ] Generate Automerge changes:
        - For lists: Apply line-by-line diffs to the Automerge document (or structured diff based on itemization).
        - [x] For notes: Use `tx.update_text()` on a root "content" field in an Automerge transaction.
      - [x] Save the updated full `automerge_state` back to `syncd.db` and update `last_sync_hash`.
      - [x] Extract compact Automerge changes/diffs (`Vec<u8>`) using `doc.get_changes_added()` for network transmission.
    - [ ] **Develop logic for applying remote Automerge changes to local files:**
      - [ ] After receiving an encrypted Automerge change set from `lst-server` and decrypting it:
      - [ ] Load the corresponding `automerge_state` from `syncd.db`.
      - [ ] Apply the decrypted Automerge changes to the document (`doc.apply_changes()`).
      - [ ] Re-render the full Automerge document back into Markdown format (preserving frontmatter if possible).
      - [ ] Overwrite the local Markdown file with the new content.
      - [ ] Save the updated `automerge_state` to `syncd.db` and update `last_sync_hash`.
  - [ ] **Implement client-side encryption (XChaCha20-Poly1305) for Automerge data sent to `lst-server`:**
    - [ ] Encrypt generated Automerge change sets (`Vec<u8>`) before sending via WebSocket.
    - [ ] Decrypt received Automerge change sets after receiving via WebSocket.
    - [ ] Handle encryption of full Automerge snapshots for initial sync or compaction events as per `SPEC.md`.
  - [ ] **Implement WebSocket networking to communicate with `lst-server` for Automerge sync (as per `SPEC.md`):**
    - [ ] Connect to `/api/sync` WebSocket endpoint.
    - [ ] Implement client-side message handling for `Authenticate`, `RequestDocumentList`, `RequestSnapshot`, `PushChanges`, `PushSnapshot`.
    - [ ] Process incoming server messages: `Authenticated`, `DocumentList`, `Snapshot`, `NewChanges`, `RequestCompaction`.
  - [ ] Implement robust file event handling (debouncing, better temp/hidden file filtering).
  - [ ] Implement proper daemonization (beyond `--foreground` flag).
- [ ] Create file format parsers for notes and posts
- [ ] Define and implement sync strategy for notes/posts (e.g., Git-based 3-way merge or simpler CRDT, also client-side encrypted)

## Server Components (`lst-server`)

- [x] Build Axum API server (auth part implemented)
- [x] Implement authentication via human-friendly and QR passwordless login tokens
- [ ] **Sync & Data Handling:**
  - [ ] Add WebSocket endpoint for real-time sync message relay (handling **encrypted CRDT blobs**)
  - [ ] Implement persistence for **encrypted CRDT list data blobs** (e.g., using sled or flat files)
  - [ ] Implement logic for relaying encrypted CRDT blobs between connected `lst-syncd` clients (server is a "dumb pipe" for data content)
- [ ] **Security & Configuration:**
  - [ ] Make JWT secret configurable (env var or config file) instead of hardcoded
- [ ] Set up SMTP email delivery with lettre (currently logs to console)
- [ ] Configure server deployment for Proxmox LXC (scripts, best practices)
- [ ] Set up reverse proxy configuration (Caddy/Traefik examples)

## CLI Implementation (`lst`)

- [x] Implement `lst ls` command
- [x] Implement `lst add <list> <text>` command
- [x] Implement `lst done <list> <target>` command with basic fuzzy matching
- [x] Implement `lst pipe <list>` command
- [x] Add `--json` output option for all commands
- [x] Implement note commands (`note new`, `note add`, `note open`, `note rm`, `note ls`)
- [x] Support directory structures (e.g. groceries/pharmacy.md) while still supporting fuzzy search only by name (without having to always specify the directory)
- [x] Implement daily list commands (`dl add`, `dl done`, `dl undone`, `dl ls`, `dl rm`) with automatic organization in `daily_lists/` subdirectory
- [x] Implement daily note command (`dn`) with automatic organization in `daily_notes/` subdirectory
- [ ] Add `share` and `unshare` commands to manage document members (will involve key exchange mechanisms)
- [ ] **Improvements & Polish:**
  - [ ] Improve `fuzzy_find` beyond simple "contains"
  - [ ] Enhance error handling: replace `unwrap()`/`expect()` with user-friendly messages and proper error propagation
  - [ ] Review and improve robustness of file/path operations
- [ ] Implement post commands (`post new`, `post list`, `post publish`) (Post content also client-side encrypted for sync)
- [ ] Implement image commands (`img add`, `img paste`, `img list`, `img rm`) (Image data also client-side encrypted for sync)

## Client Applications

- [ ] Build Tauri slim GUI
  - [ ] Create toggleable, always-on-top window
  - [ ] Implement Markdown viewer/editor
  - [ ] Add sync status tray icon
- [ ] Develop Tauri 2 mobile app
  - [ ] Implement offline SQLite cache with CRDT sync (hooking into `lst-syncd` logic or its library form, handling encryption)
  - [ ] Add share-sheet "Add to list" functionality
  - [ ] Create AppIntents integration
- [ ] Build Apple Shortcuts integration
  - [ ] Implement AddItem, RemoveItem intents
  - [ ] Implement GetList, DraftPost intents
- [ ] Develop AGNO voice agent
  - [ ] Integrate Whisper transcription
  - [ ] Create AGNO agent for natural language processing
  - [ ] Implement JSON action interface

## Configuration & Infrastructure

- [x] Implement configuration loading from `~/.config/lst/lst.toml`
- [x] Unified configuration system across all components
- [x] Auto-generate device_id for syncd on first startup
- [x] Separate CLI and syncd server configurations (within the unified file)

## Testing

- [ ] Add unit tests for `storage/markdown.rs` (parsing, item manipulation, anchors)
- [ ] Add unit tests for `models/list.rs`
- [ ] Add unit tests for client-side encryption/decryption logic in `lst-syncd`
- [ ] Expand server integration tests for auth and WebSocket sync endpoints (testing relay of opaque blobs)
- [ ] Add integration tests for `lst-syncd` once networking, CRDT, and encryption logic are in place
- [ ] Set up CI pipeline to run tests automatically

## Next Immediate Tasks (Focus: Automerge-based Encrypted List Sync MVP)

1.  **[Syncd] Setup Automerge Core & Local Persistence (`syncd.db`):**
    *   [ ] Add `automerge` (with `rusqlite` feature), `rusqlite`, and `uuid` to `lst-syncd/Cargo.toml`.
    *   [ ] Implement `syncd.db` (SQLite) initialization in `lst-syncd` with `documents` table (`doc_id` UUID PK, `file_path` TEXT UNIQUE, `doc_type` TEXT, `last_sync_hash` TEXT, `automerge_state` BLOB) as per `SPEC.md`.
2.  **[Syncd] Implement Local File to Automerge Document Sync Logic:**
    *   [ ] Develop logic to read Markdown files, load/create `Automerge` documents from/to `automerge_state` in `syncd.db`.
    *   [ ] Implement conversion of file content changes (line-by-line for lists, text diffs for notes) into Automerge transactions.
    *   [ ] Save updated `automerge_state` to `syncd.db` and calculate `last_sync_hash`.
    *   [ ] Extract Automerge changes (`Vec<u8>`) for network sync using `doc.get_changes_added()`.
3.  **[Syncd] Implement Client-Side Encryption/Decryption for Automerge Data:**
    *   [ ] Integrate XChaCha20-Poly1305 (e.g., using `chacha20poly1305` crate or `ring`).
    *   [ ] Encrypt Automerge changes/snapshots before sending to `lst-server`.
    *   [ ] Decrypt received Automerge changes/snapshots from `lst-server`.
    *   *(Key management is a separate, larger task; for MVP, a configurable/fixed key can be used for testing).*
4.  **[Server] Implement WebSocket Sync Endpoint on `lst-server` for Encrypted Automerge Blobs:**
    *   [ ] Define/update `lst-proto` messages for Automerge sync based on `SPEC.md` (e.g., `Authenticate`, `RequestDocumentList`, `RequestSnapshot`, `PushChanges { doc_id, device_id, changes: Vec<Vec<u8>> }`, `PushSnapshot`, `NewChanges`, `RequestCompaction`).
    *   [ ] Implement WebSocket connection handling, authentication, and relay of these encrypted Automerge blobs (server remains zero-knowledge).
5.  **[Syncd] Network `lst-syncd` with `lst-server` using Automerge Sync Protocol:**
    *   [ ] Connect to the `lst-server` WebSocket endpoint (`/api/sync`).
    *   [ ] Implement client-side handling for all `SPEC.md` sync protocol messages (sending encrypted changes/snapshots, requesting documents, responding to compaction).
6.  **[Syncd] Implement Remote Automerge Change Application:**
    *   [ ] After receiving and decrypting Automerge changes from the server, apply them to the local `Automerge` document loaded from `syncd.db`.
    *   [ ] Re-render the Automerge document to Markdown and overwrite the local file.
    *   [ ] Update `automerge_state` and `last_sync_hash` in `syncd.db`.
7.  **[Server] Persistence for Encrypted Automerge Data on `lst-server`:**
    *   [ ] Implement `content.db` schema on server (`documents` table: `doc_id`, `user_id`, `encrypted_snapshot`; `document_changes` table: `change_id`, `doc_id`, `device_id`, `encrypted_change`) as per `SPEC.md`.
    *   [ ] Store received encrypted Automerge changes and snapshots. Handle compaction logic.
8.  **[Testing] Initial Automerge-based Encrypted Sync Tests:**
    *   [ ] Manual E2E tests: modify a list/note on one client, verify update on another via `lst-syncd` and `lst-server`, ensuring data is unreadable on the server.
    *   [ ] Add basic unit tests for Automerge document conversion, change application, encryption/decryption, and WebSocket message handling.
9.  **[CLI] Polish & Error Handling:** (Remains important)
    *   [ ] Address critical `unwrap()`/`expect()` calls in existing CLI commands.
    *   [ ] Improve `fuzzy_find` for `lst done`.

## DevOps (Simplified)

- [ ] Create systemd service file for `lst-server`
- [ ] Create systemd service file for `lst-syncd` (if not using user-level services)
- [ ] Document basic deployment steps for `lst-server` on a Linux machine (e.g., Proxmox LXC or simple VM)
- [ ] Configure DNS for email (SPF/DKIM) if using SMTP
- [ ] Set up CI pipeline (GitHub Actions or similar) for builds and tests

## Documentation

- [x] Create initial SPEC.md
- [ ] Write installation guide
- [ ] Create user documentation for CLI and sync setup (including client-side encryption concepts)
- [ ] Document API endpoints (auth, sync)
- [ ] Write developer documentation (architecture, CRDT choices, encryption strategy)
- [ ] Document file formats and schemas
- [ ] Create/update architecture diagrams
