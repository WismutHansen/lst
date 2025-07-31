# TODO List for lst Project

## 🎨 Tinted Theming Implementation (NEW)

### Phase 1: Core Infrastructure
- [x] Extend `lst-core` config parsing to handle `[theme]` section
- [x] Create theme data structures (Theme, Palette, SemanticMapping)
- [x] Implement theme validation and error handling
- [x] Add theme inheritance system (base themes + overrides)
- [x] Create built-in theme templates (base16-default-dark, base16-default-light, etc.)
- [x] Implement theme file discovery (built-in + user themes)
- [x] Create theme loader with inheritance support
- [ ] Add theme caching for performance
- [ ] Implement theme file watching for hot reloading
- [ ] Add theme validation and error reporting

### Phase 2: CLI Commands
- [x] Add `lst themes list` command
- [x] Add `lst themes current` command  
- [x] Add `lst themes apply <theme-name>` command
- [x] Add `lst themes validate <theme-file>` command
- [x] Add `lst themes info <theme-name>` command

### Phase 3: Frontend Integration
- [ ] Create CSS custom properties generator from theme data
- [ ] Implement semantic color mapping (background, foreground, primary, etc.)
- [ ] Add CSS variable injection system for desktop app
- [ ] Update index.css to use CSS custom properties (desktop)
- [ ] Update index.css to use CSS custom properties (mobile)
- [ ] Replace hardcoded hex colors with CSS variables (desktop App.tsx)
- [ ] Replace hardcoded hex colors with CSS variables (mobile App.tsx)
- [ ] Implement theme provider in React context
- [ ] Add theme switching UI components
- [ ] Update Tailwind config to use theme colors

### Phase 4: Mobile-Specific Features
- [ ] Create SQLite theme storage schema
- [ ] Implement theme CRUD operations in mobile database
- [ ] Add theme sync capability with server
- [ ] Create React Native theme context provider
- [ ] Add system theme detection (light/dark mode)

### Phase 5: Advanced Features
- [ ] Create Tinty template repository for lst
- [ ] Add Tinty hook support in configuration
- [ ] Implement theme file generation for Tinty
- [ ] Add environment variable support for theme data
- [ ] Test Tinty integration workflow
- [ ] Add theme import/export functionality
- [ ] Create custom theme creation wizard
- [ ] Add system theme detection (macOS/Linux)
- [ ] Implement automatic light/dark mode switching

**Progress**: 12/29 tasks complete (41%)

---

## Core Infrastructure

- [x] Set up Rust project structure with Cargo
- [x] Create basic command-line interface structure
- [x] Implement core storage model for content directories
- [x] Create file format parsers for lists
- [x] Implement anchor generation and tracking
- [x] Convert project to a Cargo workspace
- [x] Create `lst-proto` crate for shared wire types
- [ ] Create file format parsers for notes and posts
- [ ] Define and implement sync strategy for notes/posts (e.g., Git-based 3-way merge or simpler CRDT, also client-side encrypted)

## Secure Multi-Device Sync

- [ ] **`lst-syncd` (Client-side Sync Daemon):**
  - [x] Scaffold `lst-syncd` daemon with file watching.
  - [x] **Integrate `automerge` crate for CRDT-based list and note synchronization:**
    - [x] Add `automerge` (with `rusqlite` feature), `rusqlite` (for `syncd.db`), and `uuid` dependencies to `lst-syncd/Cargo.toml`.
    - [x] **Implement `syncd.db` (SQLite) for local Automerge state management (`lst-syncd/src/database.rs` or similar):**
      - [x] Define `documents` table schema: `doc_id` (UUID PK), `file_path` (TEXT UNIQUE), `doc_type` (TEXT, e.g., 'list', 'note'), `last_sync_hash` (TEXT), `automerge_state` (BLOB for the full Automerge document), `owner` (TEXT), `writers` (TEXT), `readers` (TEXT).
      - [x] Implement function to initialize the database and table.
    - [x] **Develop logic for processing local file changes into Automerge documents (`lst-syncd/src/sync.rs` or similar):**
      - [x] On file change, read content and compare its hash with `last_sync_hash` from `syncd.db`.
      - [x] If different, load `automerge_state` for the file. If no state, create a new `Automerge` document.
      - [x] Generate Automerge changes (line-by-line for lists, text diffs for notes).
      - [x] Save the updated full `automerge_state` back to `syncd.db` and update `last_sync_hash`.
      - [x] Extract compact Automerge changes/diffs (`Vec<u8>`) for network transmission.
    - [x] **Develop logic for applying remote Automerge changes to local files:**
      - [x] After receiving an encrypted Automerge change set from `lst-server` and decrypting it:
      - [x] Load the corresponding `automerge_state` from `syncd.db`.
      - [x] Apply the decrypted Automerge changes to the document (`doc.apply_changes()`).
      - [x] Re-render the full Automerge document back into Markdown format (preserving frontmatter if possible).
      - [x] Overwrite the local Markdown file with the new content.
      - [x] Save the updated `automerge_state` to `syncd.db`.
  - [x] **Implement client-side encryption (XChaCha20-Poly1305) for Automerge data.**
  - [ ] **Implement asymmetric cryptography for secure device pairing:**
    - [ ] Integrate a crate for "Sealed Box" style encryption (e.g., `sodiumoxide`).
    - [ ] Implement device-specific public/private keypair generation.
    - [ ] Implement logic to encrypt the master key with a new device's public key.
  - [x] **Implement WebSocket networking to communicate with `lst-server` for Automerge sync.**
  - [ ] Implement robust file event handling (debouncing, better temp/hidden file filtering).
  - [ ] Implement proper daemonization (beyond `--foreground` flag).

- [ ] **`lst-server` (Encrypted Relay):**
  - [x] Build Axum API server with passwordless auth.
  - [ ] **Implement WebSocket endpoint for real-time sync message relay.**
  - [x] **Implement server-side persistence for encrypted blobs** (documents and changes).
  - [ ] **Implement Device Provisioning API (`/api/provision`)** to facilitate the secure device pairing handshake.
  - [ ] Make JWT secret configurable (env var or config file).
  - [ ] Set up SMTP email delivery with `lettre`.

- [ ] **`lst` (CLI):**
  - [ ] **Rework `lst sync setup`** to handle both the initial creation of a master key (first device) and the secure onboarding flow (new devices).
  - [ ] **Implement `lst sync add-device` command** to authorize a new device by scanning a QR code and sending the encrypted master key.

## CLI Implementation (`lst`)

- [x] Implement `lst ls` command
- [x] Implement `lst add <list> <text>` command
- [x] Implement `lst done <list> <target>` command with basic fuzzy matching
- [x] Implement `lst pipe <list>` command
- [x] Add `--json` output option for all commands
- [x] Implement note commands (`note new`, `note add`, `note open`, `note rm`, `note ls`)
- [x] Support directory structures with fuzzy search by name.
- [x] Implement daily list commands (`dl`)
- [x] Implement daily note command (`dn`)
- [x] Add `share` and `unshare` commands to manage document members.
- [ ] Implement post commands (`post new`, `post list`, `post publish`).
- [ ] Implement image commands (`img add`, `img paste`, `img list`, `img rm`).

## Client Applications

### Desktop App (lst-desktop) - Tauri Implementation

#### Phase 1: Core List Management

- [x] Basic Tauri project setup with TypeScript bindings
- [x] Implement `get_lists()` command to list available lists
- [x] Implement `get_list(name)` command to load specific list
- [x] Implement `get_notes()` command to list available notes
- [x] **Core List Operations (leveraging lst-cli functions):**
  - [x] Add Tauri command for `create_list(title: String)`
  - [x] Add Tauri command for `add_item(list: String, text: String)`
  - [x] Add Tauri command for `toggle_item(list: String, target: String)`
  - [x] Add Tauri command for `remove_item(list: String, target: String)`
  - [x] Add Tauri command for `save_list(list: List)`

#### Phase 2: User Interface

- [x] **List Management UI:**
  - [x] Create list browser/sidebar showing all available lists
  - [x] Implement list view with checkboxes for todo items
  - [x] Add new list creation dialog
  - [x] Add item input field with quick-add functionality
  - [x] Support for directory structure visualization (nested lists)
- [x] **Item Management UI:**
  - [x] Checkbox interactions for marking items done/undone
  - [x] Inline editing for item text
  - [x] Delete confirmation for items
  - [x] Drag-and-drop reordering
  - [x] Multi-select for bulk operations

#### Phase 3: Search and Navigation

- [ ] **Search Integration (leveraging lst-cli fuzzy matching):**
  - [ ] Add Tauri command for `search_lists(query: String)` -> expose fuzzy search
  - [ ] Add Tauri command for `search_items(query: String)` -> expose item search across all lists
  - [x] Implement global search bar with real-time results
  - [x] Add keyboard shortcuts for quick navigation (Cmd+P style)
- [x] **Navigation Features:**
  - [x] Breadcrumb navigation for nested directories
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

#### Technical Implementation Notes

- **Leverage Existing Rust Functions:** Wrap existing `lst_cli` functions with Tauri commands rather than reimplementing
- **Type Safety:** Use Specta for end-to-end type safety between Rust and TypeScript
- **State Management:** Use React/Vue state management for UI state, Rust functions for data persistence
- **Real-time Updates:** Implement file watching to update UI when files change externally
- **Error Handling:** Proper error propagation from Rust to TypeScript with user-friendly messages

## Testing

- [ ] Add unit tests for `storage/markdown.rs`
- [ ] Add unit tests for `models/list.rs`
- [ ] **Add unit tests for the device pairing crypto logic.**
- [ ] Expand server integration tests for auth, **device provisioning**, and WebSocket sync endpoints.
- [ ] Add integration tests for `lst-syncd`, including the full device pairing flow.
- [ ] Set up CI pipeline to run tests automatically.
</file>
