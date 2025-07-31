![lst Banner](Banner_black.png)

# lst - Personal Lists & Notes App

`lst` is a personal lists, notes, and blog posts management application with a focus on plain-text storage, offline-first functionality, and multi-device synchronization.

## Installation

For a step-by-step guide see [docs/INSTALL.md](docs/INSTALL.md).

### CLI Feature Flags

The `lst-cli` supports optional features to reduce compilation time and dependencies:

- **`gui`** (default): Enables desktop app integration and live updates
- **`lists`** (default): Core list functionality  
- **`notes`**: Note management features
- **`posts`**: Blog post features
- **`media`**: Image and media handling

#### Installation Options

```bash
# Full installation (default - includes GUI integration)
cargo install --path crates/lst-cli

# Minimal installation (no GUI dependencies, ~3x faster compilation)
cargo install --path crates/lst-cli --no-default-features --features lists

# Custom feature selection
cargo install --path crates/lst-cli --no-default-features --features "lists,notes"
```

**Compilation Time Comparison:**
- With GUI features: ~19 seconds
- Without GUI features: ~6 seconds (3x faster)

### From Source

```bash
git clone https://github.com/yourusername/lst.git
cd lst

# Install the CLI tool (with GUI integration)
cargo install --path crates/lst-cli

# Install the CLI tool WITHOUT GUI dependencies (faster compilation)
cargo install --path crates/lst-cli --no-default-features --features lists

# Install the MCP server (optional, lightweight)
cargo install --path crates/lst-mcp

# Install the sync daemon (optional)
cargo install --path crates/lst-syncd

# Install the server (optional)
cargo install --path crates/lst-server
```

### MCP Server Setup

The `lst-mcp` provides a Model Context Protocol server that allows AI assistants (like Claude) to manage your lists and notes.

#### Installing the MCP Server

```bash
# Install the MCP server (lightweight, no Tauri dependencies)
cargo install --path crates/lst-mcp

# Run the MCP server
lst-mcp
```

The MCP server provides tools for:
- Listing all available lists
- Adding items to lists
- Marking items as done/undone
- Managing list content through AI assistants

### HTTP API Server Setup

The `lst-server` provides a centralized HTTP API for content synchronization and multi-device access.

#### Building the Server

```bash
# Build the server binary
cargo build --release --bin lst-server

# Or install it system-wide
cargo install --path crates/lst-server
```

#### Configuration

1. Create a configuration file at `~/.config/lst/lst.toml` (see [examples/lst.toml](examples/lst.toml) for reference):

```toml
[server]
host = "127.0.0.1"  # or "0.0.0.0" for all interfaces
port = 5673

[database]
# Directory for server databases (tokens.db, content.db, sync.db)
data_dir = "~/.config/lst/lst_server_data"

[email]
# Optional: SMTP settings for login tokens (if omitted, tokens are logged to stdout)
smtp_host = "smtp.gmail.com"
smtp_user = "your-email@gmail.com"
smtp_pass = "${SMTP_PASSWORD}"  # Environment variable
sender = "noreply@yourdomain.com"
```

2. Set up environment variables (if using email):
```bash
export SMTP_PASSWORD="your-app-password"
```

#### Running the Server

```bash
# Using the installed binary
lst-server

# Or with custom config path
lst-server --config /path/to/your/lst.toml

# Or directly from source
cargo run --bin lst-server
```

The server will:
- Listen on the configured host:port (default: `127.0.0.1:5673`)
- Create SQLite databases in the configured data directory
- Provide REST API endpoints at `/api/*`
- Send login tokens via email (if configured) or log them to stdout

#### API Usage

1. **Request login token**:
```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "host": "your.server.com"}' \
  http://localhost:5673/api/auth/request
```

2. **Verify token and get JWT**:
```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "token": "RECEIVED-TOKEN"}' \
  http://localhost:5673/api/auth/verify
```

3. **Use JWT for authenticated requests**:
```bash
JWT_TOKEN="your-jwt-token"
curl -X POST -H "Content-Type: application/json" -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{"kind": "notes", "path": "example.md", "content": "Hello from API!"}' \
  http://localhost:5673/api/content
```

For complete API documentation, see [SPEC.md](SPEC.md).

#### CLI Authentication Workflow

The CLI provides a streamlined authentication flow that integrates with the server's email-based token system:

1. **Configure server URL** (if not done already):
```bash
lst sync setup --server http://localhost:5673/api
```

2. **Request authentication**:
```bash
lst auth request user@example.com
```
This sends a token to your email address.

3. **Verify and store JWT**:
```bash
lst auth verify user@example.com YOUR-TOKEN-HERE
```
This exchanges the email token for a JWT that's stored locally for future requests.

4. **Use authenticated commands**:
```bash
lst server create notes "test.md" "Hello from authenticated CLI!"
```

#### CLI Authentication Commands

The CLI includes these authentication commands:

```bash
# Request authentication token
lst auth request user@example.com

# Verify token and store JWT
lst auth verify user@example.com RECEIVED-TOKEN

# Check authentication status
lst auth status

# Logout (remove stored JWT)
lst auth logout
```

#### Server Content Commands

Once authenticated, you can interact with server content directly:

```bash
# Create content on the server
lst server create notes "example.md" "Hello from CLI!"

# Get content from the server
lst server get notes "example.md"

# Update content on the server
lst server update notes "example.md" "Updated content"

# Delete content from the server
lst server delete notes "example.md"
```

## Features

- Manage to-do lists from the command line
- Stores data locally as plain Markdown files (CLI); `lst-server` uses SQLite databases for centralized data management via its API.
- Work offline, sync when connected
- Fuzzy matching for item targeting
- Supports multiple document types: lists, notes, and blog posts
- **Directory structure support**: Organize files in nested directories while maintaining fuzzy search by filename
- **Daily workflows**: Automatic organization of daily lists and notes in dedicated subdirectories
- **Edit & reorder**: Change item text or move items within a list
- **Image management**: Attach or paste images into notes and lists
- **Share documents**: Grant read or write access to specific devices
- **Sync daemon control**: `lst sync` commands to configure and monitor background sync
- **Tauri apps**: Optional desktop and mobile front‑ends built with Tauri
- **MCP Integration**: Model Context Protocol server for AI assistant integration
- **Live Updates**: Real-time GUI updates when using CLI commands
- **Theming System**: Comprehensive theme support with base16/base24 color schemes across all applications

## Usage

### Lists

```bash
# List all lists
lst ls

# View a specific list
lst ls <list_name>

# Open a list in your editor
lst open <list_name>

# Add an item to a list (creates the list if it doesn't exist)
lst add <list_name> "<item_text>"

# Mark an item as done (by text, fuzzy matching, or index)
lst done <list_name> "<item_text>"  # Text match
lst done <list_name> "<partial_text>"  # Fuzzy match
lst done <list_name> "#2"  # By index (the second item)

# Remove an item from a list
lst rm <list_name> "<item_text>"

# Read items from stdin
cat items.txt | lst pipe <list_name>

# Directory structure support
lst add groceries/pharmacy "Vitamins"     # Creates groceries/pharmacy.md automatically
lst add pharmacy "Bandages"               # Fuzzy matches to groceries/pharmacy.md

# Share a document with specific devices
lst share <path> --writers <ids> --readers <ids>

# Remove sharing information
lst unshare <path>
```

### Notes

```bash
# Create a new note
lst note new "<title>"

# Append text to a note (creates note if missing)
lst note add "<title>" "<text>"

# Open a note in your editor
lst note open "<title>"

# Remove a note
lst note rm "<title>"

# List all notes
lst note ls

# Directory structure support for notes
lst note new "projects/rust/lst"         # Creates projects/rust/lst.md automatically
lst note open "lst"                      # Fuzzy matches to projects/rust/lst.md
```

### Images

```bash
# Add an image to a document
lst img add path/to/pic.jpg --to notes/travel.md --caption "Mountains"

# Paste an image from the clipboard
lst img paste --to lists/groceries.md

# List images referenced in a document
lst img list notes/travel.md

# Remove an image
lst img rm notes/travel.md <hash>
```

### Daily Commands

`lst` provides special commands for daily workflows that automatically organize files by date:

```bash
# Daily Lists (stored in daily_lists/ subdirectory)
lst dl                           # Show today's daily list
lst dl add "<task>"              # Add task to today's daily list
lst dl done "<task>"             # Mark task as done
lst dl undone "<task>"           # Mark task as undone
lst dl rm "<task>"               # Remove task from today's daily list
lst dl ls                        # List all daily lists with dates

# Daily Notes (stored in daily_notes/ subdirectory)
lst dn                           # Open today's daily note in editor
```

Daily files are automatically named with the current date (e.g., `daily_lists/20250524_daily_list.md`, `daily_notes/20250524_daily_note.md`) and organized in their respective subdirectories.

### Sync Daemon

```bash
# Configure sync settings
lst sync setup --server https://lists.example.com --token <auth>

# Start the background daemon
lst sync start

# Check daemon status
lst sync status

# Stop the daemon
lst sync stop

# View logs
lst sync logs --follow
```

### Themes

`lst` includes a comprehensive theming system that supports base16 and base24 color schemes across all applications (CLI, desktop, and mobile).

```bash
# List available themes
lst theme list

# Apply a theme
lst theme apply <theme_name>

# Show current theme information
lst theme current

# Get detailed theme information
lst theme info <theme_name>

# Validate a theme file
lst theme validate <theme_file>
```

#### Built-in Themes

The system includes several built-in themes:
- **catppuccin-mocha**: Dark theme with warm, muted colors
- **catppuccin-latte**: Light theme with soft, pastel colors
- **gruvbox-dark**: Popular dark theme with earthy tones
- **gruvbox-light**: Light variant of the gruvbox theme
- **nord**: Cool, arctic-inspired color palette
- **solarized-dark**: Classic dark theme with balanced contrast
- **solarized-light**: Light variant of the solarized theme

#### Theme Configuration

Themes can be configured in your `lst.toml` file:

```toml
[theme]
# Set the active theme
name = "catppuccin-mocha"

# Override specific colors
[theme.vars]
primary = "#a6e3a1"
background = "#1e1e2e"
```

#### Desktop and Mobile Apps

Both desktop and mobile applications support real-time theme switching:
- **Desktop**: Theme selector in the sidebar
- **Mobile**: Theme selector in the Settings panel
- **Live Updates**: Theme changes apply immediately without restart
- **Consistent Experience**: Same themes work across all platforms

## Configuration

`lst` uses a unified TOML configuration file located at `~/.config/lst/lst.toml` that is shared across all components (CLI, server, sync daemon). You can override the config file location by setting the `LST_CONFIG` environment variable.

Configuration changes take effect the next time you run a command. If you change the `content_dir` option, existing data will remain in the old location, and you'll need to move it manually to the new location.

### Configuration Options

#### CLI & Server Configuration

```toml
[server]
# URL of the sync server API (CLI) / Server host & port (server only)
url = "https://lists.example.com/api"
auth_token = "your-auth-token"
host = "127.0.0.1"  # server only
port = 3000         # server only
```

#### Sync Daemon Configuration

```toml
[syncd]
# Server URL for remote sync (if None, runs in local-only mode)
url = "https://lists.example.com/api"
auth_token = "your-auth-token"
# device_id is auto-generated on first startup
```

#### User Interface Configuration

```toml
[ui]
# Order in which to try different methods when resolving item targets
# Valid values: "anchor", "exact", "fuzzy", "index", "interactive"
resolution_order = ["anchor", "exact", "fuzzy", "index", "interactive"]

# Enable Vim-like keybindings in the frontend applications
vim_mode = false

# Leader key used for command sequences (defaults to space)
leader_key = " "
```

#### Theme Configuration

```toml
[theme]
# Active theme name (use 'lst theme list' to see available themes)
name = "catppuccin-mocha"

# Override specific theme colors
[theme.vars]
primary = "#a6e3a1"
background = "#1e1e2e"
foreground = "#cdd6f4"
```

#### Fuzzy Matching Configuration

```toml
[fuzzy]
# Similarity threshold for fuzzy matching (0.0 to 1.0)
# Higher values require closer matches
threshold = 0.75

# Maximum number of suggestions to show in interactive mode
max_suggestions = 7
```

#### Path Configuration

```toml
[paths]
# Base directory for all content (lists, notes, posts, media) when used by the CLI directly.
# For `lst-server`, this directory (or the directory containing lst.toml if content_dir is not set,
# which then determines the location of a 'lst_server_data' subdirectory)
# is where SQLite database files (e.g., tokens.db, content.db) are stored.
content_dir = "~/Documents/lst"

# Override the media directory location (relative to content_dir)
# Default: "$content_dir/media"
# Note: Media handling via server API is not yet specified.
media_dir = "~/Documents/lst/media"
```

#### Server-Only Configuration

The `lst-server` uses specific sections from `lst.toml`:

```toml
# Settings for lst-server under its [server] block (host, port)
# are already shown in "CLI & Server Configuration".

# The [paths] block (see above) is crucial for lst-server:
# `content_dir` (or the directory of lst.toml if content_dir is not set)
# determines where the 'lst_server_data' subdirectory is created,
# which in turn stores its SQLite database files (e.g., tokens.db, content.db).

[email]
# SMTP relay settings (optional - if missing, login links logged to stdout)
# Used by lst-server to send login tokens.
smtp_host = "smtp.example.com"
smtp_user = "your-smtp-user"
smtp_pass = "${SMTP_PASSWORD}"  # Can be an environment variable like ${MY_SMTP_PASS}
sender = "noreply@example.com"

# The [content] block (with 'root', 'kinds', 'media_dir') previously used for
# file system layout is no longer directly applicable to how lst-server serves
# content via its API, as content is now stored in an SQLite database.
# 'Kind' is now a dynamic part of the data schema within the database.
```

#### Sync Daemon-Only Configuration

```toml
[sync]
# Sync behavior settings
interval_seconds = 30
max_file_size = 10485760  # 10MB
exclude_patterns = [".*", "*.tmp", "*.swp"]

[storage]
# CRDT storage settings
crdt_dir = "~/.config/lst/crdt"
max_snapshots = 100
```

## Example Configuration

An example unified configuration file is provided in the `examples/lst.toml` file in the repository. You can copy this file to `~/.config/lst/lst.toml` and customize it to your needs. Each component reads only the sections it needs from the same file.

## Storage Format

**CLI and Local Usage:**

When using the `lst` CLI directly for local operations, data is stored as Markdown files in the content directory (which can be configured in `lst.toml` using `paths.content_dir`):

```
content/
├─ lists/                    # per-line anchors, supports nested directories
│   ├─ groceries.md
│   ├─ groceries/
│   │   └─ pharmacy.md
│   └─ daily_lists/         # automatically organized daily lists
│       └─ 20250524_daily_list.md
├─ notes/                    # whole-file merge, supports nested directories
│   ├─ bicycle-ideas.md
│   ├─ projects/
│   │   └─ rust/
│   │       └─ lst.md
│   └─ daily_notes/         # automatically organized daily notes
│       └─ 20250524_daily_note.md
├─ posts/                    # blog, Zola-compatible
│   └─ 2025-04-22-first-ride.md
└─ media/                    # images & binary files
    ├─ 6fc9e6e2b4d3.jpg      # originals
    └─ 6fc9e6e2b4d3@512.webp # thumbnails
```

### File Format Examples

#### Lists

```markdown
---
id: 4a2e00bf-5842-4bff-8487-b9672413f0b6
title: groceries
sharing: []
updated: 2025-04-21T07:35:51.705060Z
---

- [ ] Milk ^XMuD1
- [x] Bread ^lkJzl
- [ ] Eggs ^w5Cdq
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Project Architecture

The `lst` project follows a modular architecture with clear separation of concerns across multiple crates:

### Crate Structure

1. **`lst-core`** - Core functionality
   - Contains shared data structures (`List`, `ListItem`, etc.)
   - Storage layer for markdown files and notes
   - Configuration management
   - Core command implementations
   - Theme system with base16/base24 support
   - No UI dependencies - lightweight and reusable

2. **`lst-cli`** - Command-line interface
   - Depends on `lst-core` with optional Tauri integration
   - Command-line parsing and user interaction
   - Optional desktop app communication (live updates) via `gui` feature
   - Can be installed without GUI dependencies for faster compilation

3. **`lst-mcp`** - MCP (Model Context Protocol) server
   - Depends on `lst-core` (lightweight, no Tauri dependencies)
   - Provides MCP tools for AI assistants (Claude, etc.)
   - Enables AI agents to manage lists and notes
   - Fast compilation without UI dependencies

4. **`lst-server`** - HTTP API server
   - REST API for multi-device synchronization
   - SQLite databases for authentication and content
   - Email-based authentication flow
   - Separate from CLI for deployment flexibility

5. **`lst-syncd`** - Background sync daemon
   - CRDT-based conflict-free synchronization
   - File watching and automatic sync
   - Encrypted client-server communication
   - Multi-device support

6. **`lst-desktop`** - Tauri desktop application
   - Cross-platform GUI built with React + TypeScript
   - Real-time updates from CLI changes
   - Rich text editing with CodeMirror
   - Integrated theme system with live switching

7. **`lst-mobile`** - Tauri mobile application
   - Mobile-optimized interface
   - SQLite storage for offline capability
   - Touch-friendly UI components
   - Mobile-friendly theme selector in Settings

### Architecture Benefits

- **Modular Design**: Each crate has a specific purpose and minimal dependencies
- **Lightweight Core**: `lst-core` can be used without UI dependencies
- **Fast MCP Server**: No Tauri compilation when installing MCP server
- **Reusable Components**: Core functionality shared across all interfaces
- **Flexible Deployment**: Install only the components you need

### Server API Overview

The `lst-server` provides an HTTP API for managing authentication and content.

#### Authentication Flow

1.  **Token Request**: `POST /api/auth/request`
    -   Client sends: `{ "email": "user@example.com", "host": "client.host.name" }`
    -   Server emails a one-time token to `user@example.com`. This token is stored temporarily in its `tokens.db` SQLite database.
2.  **Token Verification & JWT**: `POST /api/auth/verify`
    -   Client sends: `{ "email": "user@example.com", "token": "RECEIVED_TOKEN" }`
    -   Server verifies the token against `tokens.db`. If valid, it's consumed, and a JWT is issued.
3.  **Authenticated Requests**:
    -   The received JWT is used in the `Authorization: Bearer <JWT>` header for all subsequent protected API calls.

#### Content Management API

Content (like notes or lists) is stored in the server's `content.db` SQLite database. Items are identified by a `kind` (e.g., "notes", "lists") and a `path` (e.g., "personal/todos.md"). These are logical identifiers within the database.

-   **Create**: `POST /api/content`
    -   Payload: `{ "kind": "notes", "path": "travel/packing_list.md", "content": "- Passport\n- Tickets" }`
    -   Response: `201 Created` or `409 Conflict` if `kind`/`path` already exists.
-   **Read**: `GET /api/content/{kind}/{path}`
    -   Example: `GET /api/content/notes/travel/packing_list.md`
    -   Response: `200 OK` with content or `404 Not Found`.
-   **Update**: `PUT /api/content/{kind}/{path}`
    -   Payload: `{ "content": "- Passport (checked!)" }`
    -   Response: `200 OK` or `404 Not Found`.
-   **Delete**: `DELETE /api/content/{kind}/{path}`
    -   Response: `200 OK` or `404 Not Found`.

**Example `curl` Usage:**

1.  Request login token:
    ```bash
    curl -X POST -H "Content-Type: application/json" \
      -d '{ "email": "user@example.com", "host": "your.server.com" }' \
      http://your.server.com:3000/api/auth/request
    ```
    (Server sends token to `user@example.com`. Assume token is `ABCD-1234`)

2.  Verify token and get JWT:
    ```bash
    curl -X POST -H "Content-Type: application/json" \
      -d '{ "email": "user@example.com", "token": "ABCD-1234" }' \
      http://your.server.com:3000/api/auth/verify
    ```
    (Returns JSON with `jwt` field, e.g., `{"jwt":"eyJ...", "user":"user@example.com"}`)

3.  Create a note using JWT:
    ```bash
    JWT_TOKEN="eyJ..." # Replace with actual JWT
    curl -X POST -H "Content-Type: application/json" -H "Authorization: Bearer $JWT_TOKEN" \
      -d '{ "kind": "notes", "path": "example.md", "content": "Hello from API!" }' \
      http://your.server.com:3000/api/content
    ```

4.  Read the note:
    ```bash
    curl -X GET -H "Authorization: Bearer $JWT_TOKEN" \
      http://your.server.com:3000/api/content/notes/example.md
    ```

For complete API details, please refer to [SPEC.md](SPEC.md).

### Flow of Control

A typical command flow:

1. User enters a command like `lst done my-list item1`
2. `lst-cli` parses this using `clap` and dispatches to `cli::commands::mark_done`
3. `cli::commands::mark_done` calls `lst_core::commands::mark_done`
4. `lst_core::commands::mark_done` uses `lst_core::storage::markdown::mark_done` to modify the file
5. `lst-cli` sends a notification to the desktop app (if running) for live updates

This architecture provides:

- **Separation of Concerns**: Each crate has a distinct responsibility
- **Testability**: Core logic can be tested without I/O dependencies  
- **Flexibility**: Multiple interfaces (CLI, MCP, server, GUI) can use the same core logic
- **Performance**: MCP server compiles quickly without UI dependencies
- **Live Updates**: GUI automatically refreshes when CLI makes changes

## Performance

The `lst` tools are implemented in Rust, and debug builds can exhibit noticeable startup latency.
For the fastest experience, use optimized release builds:

```bash
# Install CLI with GUI integration (default)
cargo install --path crates/lst-cli

# Install CLI without GUI dependencies (faster compilation)
cargo install --path crates/lst-cli --no-default-features --features lists

# Install MCP server (compiles quickly - no Tauri dependencies)
cargo install --path crates/lst-mcp

# Install other components as needed
cargo install --path crates/lst-server
cargo install --path crates/lst-syncd
```

Release builds start up in just a few milliseconds. The MCP server is particularly fast to compile since it doesn't include any UI dependencies.

If you prefer to build locally without installing:

```bash
# Build and run specific components
cargo build --release -p lst-cli
./target/release/lst ls <list_name>

# Build CLI without GUI dependencies (faster)
cargo build --release -p lst-cli --no-default-features --features lists
./target/release/lst ls <list_name>

# Build MCP server (always lightweight)
cargo build --release -p lst-mcp  
./target/release/lst-mcp
```
