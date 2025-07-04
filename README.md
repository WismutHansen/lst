![lst Banner](Banner_black.png)

# lst - Personal Lists & Notes App

`lst` is a personal lists, notes, and blog posts management application with a focus on plain-text storage, offline-first functionality, and multi-device synchronization.

## Installation

For a step-by-step guide see [docs/INSTALL.md](docs/INSTALL.md).

### From Source

```bash
git clone https://github.com/yourusername/lst.git
cd lst
cargo install --path .
```

## Features

- Manage to-do lists from the command line
- Stores data locally as plain Markdown files (CLI); `lst-server` uses SQLite databases for centralized data management via its API.
- Work offline, sync when connected
- Fuzzy matching for item targeting
- Supports multiple document types: lists, notes, and blog posts
- **Directory structure support**: Organize files in nested directories while maintaining fuzzy search by filename
- **Daily workflows**: Automatic organization of daily lists and notes in dedicated subdirectories

## Usage

### Lists

```bash
# List all lists
lst ls

# View a specific list
lst ls <list_name>

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

The `lst` project follows a layered architecture with clear separation of concerns:

### Core Architecture Layers

1. **Models Layer** (`models/`)

   - Contains core data structures like `List`, `ListItem`, etc.
   - Defines the domain objects without any I/O operations
   - Provides basic operations on in-memory objects

2. **Storage Layer** (`storage/`)

   - Handles persistence of model objects to disk (markdown files)
   - Provides higher-level operations that combine model operations with file I/O
   - Organized by storage format (markdown.rs, notes.rs)

3. **CLI Layer** (`cli/`)

   - Handles command-line parsing and user interaction
   - Connects user commands to storage operations

4. **Configuration Layer** (`config/`)

   - Manages application settings and paths
   - Provides utility functions for finding content directories

5. **Server Layer** (`server/`)
   - Implements a REST API for accessing the data.
   - Uses SQLite databases (`tokens.db` for authentication tokens, `content.db` for user content) for persistence.
   - These database files are typically stored in a subdirectory (e.g., `lst_server_data`) within the path derived from the server's configuration file location or the `paths.content_dir` setting in `lst.toml`.
   - Separate executable from the CLI.

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
2. `main.rs` parses this using `clap` and dispatches to `cli::commands::mark_done`
3. `cli::commands::mark_done` normalizes the list name and calls `storage::markdown::mark_done`
4. `storage::markdown::mark_done` loads the list from disk, modifies it, and saves it back

This architecture provides:

- **Separation of Concerns**: Each module has a distinct responsibility
- **Testability**: Core logic can be tested without I/O dependencies
- **Flexibility**: Multiple interfaces (CLI, server) can use the same storage and model logic

## Performance

The `lst` CLI is implemented in Rust, and debug builds (e.g., those under `target/debug`) can exhibit noticeable startup latency.
For the fastest experience, use the optimized release build:

```bash
# Install the release binary to your Cargo bin directory
cargo install --path .
```

This builds with release optimizations and should start up in just a few milliseconds.

If you prefer to build locally without installing, you can:

```bash
# Build and run the release binary
cargo build --release
./target/release/lst ls <list_name>
```
