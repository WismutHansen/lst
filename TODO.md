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
  - [ ] Implement CRDT logic for **lists** (e.g., Automerge or similar)
  - [ ] Implement local persistence of CRDT state for lists (e.g., in `storage.crdt_dir`)
  - [ ] **Implement client-side encryption (e.g., XChaCha20) for CRDT data before sending to server**
  - [ ] Implement networking to connect to `lst-server`'s WebSocket sync endpoint (sending/receiving encrypted blobs)
  - [ ] Implement robust file event handling (debouncing, better temp/hidden file filtering)
  - [ ] Implement proper daemonization (beyond `--foreground` flag)
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

## Next Immediate Tasks (Focus: Encrypted List Sync MVP)

1. **[Syncd] Implement Client-Side Encryption/Decryption for List CRDT Data:**
   - [ ] Choose an encryption library (e.g., `ring` for XChaCha20-Poly1305).
   - [ ] Define key derivation/management (e.g., from a user master key or passphrase, not yet implemented). For MVP, could use a fixed key for testing.
   - [ ] Encrypt CRDT patches/state before sending; decrypt upon receiving.
2. **[Server] Implement WebSocket Sync Endpoint on `lst-server` for Encrypted Blobs:**
   - [ ] Define `lst-proto` messages for sync (e.g., `EncryptedCrdtUpdate { device_id: String, blob: Vec<u8> }`).
   - [ ] Basic WebSocket connection handling, receiving and broadcasting these encrypted blobs. The server does not need to decrypt.
3. **[Syncd] Implement Basic List CRDT Logic in `lst-syncd`:**
   - [ ] Choose and integrate a CRDT library (or implement simple list CRDT).
   - [ ] Convert file changes from `FileWatcher` into CRDT operations for lists.
   - [ ] Apply CRDT patches (after decryption) received from the server to local list files.
4. **[Syncd] Network `lst-syncd` with `lst-server` (Encrypted):**
   - [ ] Connect to the `lst-server` WebSocket endpoint using the configured URL and JWT.
   - [ ] Send local encrypted CRDT changes; receive and process remote encrypted changes.
5. **[Server & Syncd] Basic Encrypted CRDT State Persistence:**
   - [ ] `lst-syncd`: Persist its local view of the CRDT state for lists (can be unencrypted locally if filesystem is trusted, or encrypted).
   - [ ] `lst-server`: Persist the **encrypted blobs** for lists.
6. **[Testing] Initial Encrypted Sync Tests:**
   - [ ] Manual E2E tests: modify a list on one client, see it update on another via `lst-syncd` and `lst-server` (data should be unreadable on the server).
   - [ ] Basic unit tests for new CRDT logic, encryption/decryption, and server WebSocket handlers.
7. **[CLI] Polish & Error Handling:**
   - [ ] Address critical `unwrap()`/`expect()` calls in existing CLI commands.
   - [ ] Improve `fuzzy_find` for `lst done`.

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
