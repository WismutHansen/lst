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
      - [x] Generate Automerge changes:
        - For lists: Apply line-by-line diffs to the Automerge document (or structured diff based on itemization).
        - [x] For notes: Use `tx.update_text()` on a root "content" field in an Automerge transaction.
      - [x] Save the updated full `automerge_state` back to `syncd.db` and update `last_sync_hash`.
      - [x] Extract compact Automerge changes/diffs (`Vec<u8>`) using `doc.get_changes_added()` for network transmission.
    - [x] **Develop logic for applying remote Automerge changes to local files:**
      - [x] After receiving an encrypted Automerge change set from `lst-server` and decrypting it:
      - [x] Load the corresponding `automerge_state` from `syncd.db`.
      - [x] Apply the decrypted Automerge changes to the document (`doc.apply_changes()`).
      - [x] Re-render the full Automerge document back into Markdown format (preserving frontmatter if possible).
      - [x] Overwrite the local Markdown file with the new content.
      - [x] Save the updated `automerge_state` to `syncd.db` and update `last_sync_hash`.
  - [x] **Implement client-side encryption (XChaCha20-Poly1305) for Automerge data sent to `lst-server`:**
    - [x] Encrypt generated Automerge change sets (`Vec<u8>`) before sending via WebSocket.
    - [x] Decrypt received Automerge change sets after receiving via WebSocket.
    - [x] Handle encryption of full Automerge snapshots for initial sync or compaction events as per `SPEC.md`.
  - [x] **Implement WebSocket networking to communicate with `lst-server` for Automerge sync (as per `SPEC.md`):**
    - [x] Connect to `/api/sync` WebSocket endpoint.
    - [x] Implement client-side message handling for `Authenticate`, `RequestDocumentList`, `RequestSnapshot`, `PushChanges`, `PushSnapshot`.
    - [x] Process incoming server messages: `Authenticated`, `DocumentList`, `Snapshot`, `NewChanges`, `RequestCompaction`.
  - [ ] Implement robust file event handling (debouncing, better temp/hidden file filtering).
  - [ ] Implement proper daemonization (beyond `--foreground` flag).
- [ ] Create file format parsers for notes and posts
- [ ] Define and implement sync strategy for notes/posts (e.g., Git-based 3-way merge or simpler CRDT, also client-side encrypted)

## Server Components (`lst-server`)

- [x] Build Axum API server (auth part implemented)
- [x] Implement authentication via human-friendly and QR passwordless login tokens
- [ ] **Sync & Data Handling:**
  - [x] Add WebSocket endpoint for real-time sync message relay (handling **encrypted CRDT blobs**)
  - [x] Implement persistence for **encrypted CRDT list data blobs** (e.g., using sled or flat files)
  - [x] Implement logic for relaying encrypted CRDT blobs between connected `lst-syncd` clients (server is a "dumb pipe" for data content)
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
- [x] Add `share` and `unshare` commands to manage document members (will involve key exchange mechanisms)
- [ ] **Improvements & Polish:**
  - [ ] Improve `fuzzy_find` beyond simple "contains"
  - [ ] Enhance error handling: replace `unwrap()`/`expect()` with user-friendly messages and proper error propagation
  - [ ] Review and improve robustness of file/path operations
- [ ] Implement post commands (`post new`, `post list`, `post publish`) (Post content also client-side encrypted for sync)
- [ ] Implement image commands (`img add`, `img paste`, `img list`, `img rm`) (Image data also client-side encrypted for sync)

## Client Applications

### Desktop App (lst-desktop) - Tauri Implementation

#### Phase 1: Core List Management
- [x] Basic Tauri project setup with TypeScript bindings
- [x] Implement `get_lists()` command to list available lists
- [x] Implement `get_list(name)` command to load specific list
- [x] Implement `get_notes()` command to list available notes
- [x] **Core List Operations (leveraging lst-cli functions):**
  - [x] Add Tauri command for `create_list(title: String)` -> expose `List::new()`
  - [x] Add Tauri command for `add_item(list: String, text: String)` -> expose `lst_cli::cli::commands::add_item()`
  - [x] Add Tauri command for `toggle_item(list: String, target: String)` -> expose done/undone functionality
  - [x] Add Tauri command for `remove_item(list: String, target: String)` -> expose `lst_cli::cli::commands::remove_item()`
  - [x] Add Tauri command for `save_list(list: List)` -> expose `lst_cli::storage::markdown::save_list()`

#### Phase 2: User Interface
- [ ] **List Management UI:**
  - [x] Create list browser/sidebar showing all available lists
  - [x] Implement list view with checkboxes for todo items
  - [x] Add new list creation dialog
  - [x] Add item input field with quick-add functionality
  - [x] Support for directory structure visualization (nested lists)
- [ ] **Item Management UI:**
  - [x] Checkbox interactions for marking items done/undone
  - [x] Inline editing for item text
  - [x] Delete confirmation for items
  - [x] Drag-and-drop reordering (if supported by backend)
  - [x] Multi-select for bulk operations

#### Phase 3: Search and Navigation
- [ ] **Search Integration (leveraging lst-cli fuzzy matching):**
  - [ ] Add Tauri command for `search_lists(query: String)` -> expose fuzzy search
  - [ ] Add Tauri command for `search_items(query: String)` -> expose item search across all lists
  - [ ] Implement global search bar with real-time results
  - [ ] Add keyboard shortcuts for quick navigation (Cmd+P style)
- [ ] **Navigation Features:**
  - [ ] Breadcrumb navigation for nested directories
  - [ ] Recent/frequently used lists
  - [ ] Favorites/pinned lists

#### Phase 4: Notes Management
- [ ] **Note Operations (leveraging lst-cli functions):**
  - [ ] Add Tauri command for `create_note(title: String)` -> expose note creation
  - [ ] Add Tauri command for `get_note(title: String)` -> expose note loading
  - [ ] Add Tauri command for `save_note(title: String, content: String)` -> expose note saving
  - [ ] Add Tauri command for `delete_note(title: String)` -> expose note deletion
- [ ] **Notes UI:**
  - [ ] Notes browser/sidebar
  - [ ] Basic markdown editor or rich text editor
  - [ ] Notes preview pane
  - [ ] Directory structure for notes organization

#### Phase 5: Daily Workflows
- [ ] **Daily Lists Integration:**
  - [ ] Add Tauri commands for daily list operations -> expose `lst_cli::cli::commands::daily_*`
  - [ ] Quick access to today's daily list
  - [ ] Daily list calendar view
  - [ ] Quick add to daily list from any screen
- [ ] **Daily Notes Integration:**
  - [ ] Add Tauri command for daily note access -> expose `lst_cli::cli::commands::daily_note`
  - [ ] Quick access to today's daily note
  - [ ] Daily note template support

#### Phase 6: Sync Integration
- [ ] **Sync Status (leveraging lst-syncd):**
  - [ ] Add Tauri commands for sync status -> expose `lst_cli::cli::commands::sync_status`
  - [ ] Add Tauri commands for sync control -> expose start/stop sync daemon
  - [ ] Sync status indicator in UI (connected/syncing/offline)
  - [ ] Sync conflict resolution UI
- [ ] **Sync Configuration:**
  - [ ] Add Tauri commands for sync setup -> expose `lst_cli::cli::commands::sync_setup`
  - [ ] Server configuration UI
  - [ ] Device management UI
  - [ ] Sync logs viewer

#### Phase 7: Advanced Features
- [ ] **Configuration Management:**
  - [ ] Add Tauri commands for config access -> expose `lst_cli::config::Config`
  - [ ] Settings/preferences UI
  - [ ] Theme customization
  - [ ] Keyboard shortcuts customization
- [ ] **Sharing & Collaboration:**
  - [ ] Add Tauri commands for sharing -> expose `lst_cli::cli::commands::share/unshare`
  - [ ] Share management UI
  - [ ] Collaboration indicators
  - [ ] Permission management

#### Phase 8: Polish & User Experience
- [ ] **Window Management:**
  - [ ] Toggleable, always-on-top window mode
  - [ ] System tray integration with quick actions
  - [ ] Global hotkeys for quick access
  - [ ] Multiple window support
- [ ] **Performance & Reliability:**
  - [ ] Efficient list rendering for large lists
  - [ ] Background sync without blocking UI
  - [ ] Offline mode support
  - [ ] Error handling and user feedback
- [ ] **Accessibility & Usability:**
  - [x] Keyboard navigation support
  - [x] Optional Vim keybindings mode
  - [ ] Screen reader compatibility
  - [ ] High contrast mode
  - [ ] Tooltips and help system

#### Technical Implementation Notes:
- **Leverage Existing Rust Functions:** Wrap existing `lst_cli` functions with Tauri commands rather than reimplementing
- **Type Safety:** Use Specta for end-to-end type safety between Rust and TypeScript
- **State Management:** Use React/Vue state management for UI state, Rust functions for data persistence
- **Real-time Updates:** Implement file watching to update UI when files change externally
- **Error Handling:** Proper error propagation from Rust to TypeScript with user-friendly messages
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
    *   [x] Add `automerge` (with `rusqlite` feature), `rusqlite`, and `uuid` to `lst-syncd/Cargo.toml`.
    *   [x] Implement `syncd.db` (SQLite) initialization in `lst-syncd` with `documents` table (`doc_id` UUID PK, `file_path` TEXT UNIQUE, `doc_type` TEXT, `last_sync_hash` TEXT, `automerge_state` BLOB) as per `SPEC.md`.
2.  **[Syncd] Implement Local File to Automerge Document Sync Logic:**
    *   [x] Develop logic to read Markdown files, load/create `Automerge` documents from/to `automerge_state` in `syncd.db`.
    *   [x] Implement conversion of file content changes (line-by-line for lists, text diffs for notes) into Automerge transactions.
    *   [x] Save updated `automerge_state` to `syncd.db` and calculate `last_sync_hash`.
    *   [x] Extract Automerge changes (`Vec<u8>`) for network sync using `doc.get_changes_added()`.
3.  **[Syncd] Implement Client-Side Encryption/Decryption for Automerge Data:**
    *   [x] Integrate XChaCha20-Poly1305 (e.g., using `chacha20poly1305` crate or `ring`).
    *   [x] Encrypt Automerge changes/snapshots before sending to `lst-server`.
    *   [x] Decrypt received Automerge changes/snapshots from `lst-server`.
    *   *(Key management is a separate, larger task; for MVP, a configurable/fixed key can be used for testing).*
4.  **[Server] Implement WebSocket Sync Endpoint on `lst-server` for Encrypted Automerge Blobs:**
    *   [ ] Define/update `lst-proto` messages for Automerge sync based on `SPEC.md` (e.g., `Authenticate`, `RequestDocumentList`, `RequestSnapshot`, `PushChanges { doc_id, device_id, changes: Vec<Vec<u8>> }`, `PushSnapshot`, `NewChanges`, `RequestCompaction`).
    *   [ ] Implement WebSocket connection handling, authentication, and relay of these encrypted Automerge blobs (server remains zero-knowledge).
5.  **[Syncd] Network `lst-syncd` with `lst-server` using Automerge Sync Protocol:**
    *   [x] Connect to the `lst-server` WebSocket endpoint (`/api/sync`).
    *   [x] Implement client-side handling for all `SPEC.md` sync protocol messages (sending encrypted changes/snapshots, requesting documents, responding to compaction).
6.  **[Syncd] Implement Remote Automerge Change Application:**
    *   [x] After receiving and decrypting Automerge changes from the server, apply them to the local `Automerge` document loaded from `syncd.db`.
    *   [x] Re-render the Automerge document to Markdown and overwrite the local file.
    *   [x] Update `automerge_state` and `last_sync_hash` in `syncd.db`.
7.  **[Server] Persistence for Encrypted Automerge Data on `lst-server`:**
    *   [x] Implement `content.db` schema on server (`documents` table: `doc_id`, `user_id`, `encrypted_snapshot`; `document_changes` table: `change_id`, `doc_id`, `device_id`, `encrypted_change`) as per `SPEC.md`.
    *   [x] Store received encrypted Automerge changes and snapshots. Handle compaction logic.
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
