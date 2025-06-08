# lst - personal lists & notes App – Specification v0.4

## 1 · Scope & Guiding Principles

| Principle                       | Manifestation                                                                             |
| ------------------------------- | ----------------------------------------------------------------------------------------- |
| **Plain‑text ownership**        | Everything is Markdown you can open in Neovim.                                            |
| **One core, many surfaces**     | `lst` CLI, slim desktop GUI, mobile apps, Apple Shortcuts, AGNO voice agent, public blog. |
| **Offline‑first / Self‑hosted** | Single Rust server in a Proxmox LXC; sync is opportunistic; you own the data.             |
| **Extensible "document kinds"** | _lists_, _notes_, _posts_ share storage & auth.                                           |

---

## 2 · High‑Level Architecture

```mermaid
graph TD
    subgraph Clients
        CLI["lst (Rust)"]
        GUI["Tauri slim GUI"]
        Mobile["Tauri 2 mobile"]
        Shortcuts["Apple Shortcuts / AppIntents"]
        Voice["AGNO agent"]
    end

    CLI --> API
    GUI --> API
    Mobile --> API
    Shortcuts --> API
    Voice --> API

    subgraph Server (LXC)
        API["Core API (Axum)"]
        Sync["CRDT + Git Store"]
        Mail["SMTP relay"]
        Build["Zola static build"]
    end

    API --|file events| Sync
    Sync --|publish posts| Build
```

---

## Authentication & Login

### Passwordless, Human-Friendly Token Auth

- Users authenticate by requesting a login code to their email address.
    - API call: `POST /auth/request` with their email.
    - Server generates a **human-readable, short-lived token** (e.g. `PLUM-FIRE-BIRD-7182`).
    - Server also generates a **QR code** that encodes a login URL (e.g. `lst-login://host/auth/verify?token=TOKEN&email=EMAIL`).
        - The domain/server is included, enabling true one-step login on mobile.
    - The token, QR (as base64 PNG), and login URL are returned in the API response.
- User enters or scans this code in their client (CLI, GUI, or mobile app).
    - API call: `POST /auth/verify` with email and token, or follows the encoded URL if scanned.
    - If the token is valid and unexpired, the server returns a JWT/session for further API use.
- All one-time tokens are securely stored by the server (e.g., in an SQLite database like `tokens.db`) and are consumed upon successful verification or removed if an attempt fails or they expire.
- No server-stored plaintext passwords; user authentication is ephemeral by design.
- Upon successful verification of the one-time token, the server issues a JSON Web Token (JWT) which is used for subsequent authenticated API calls. The JWT secret is a critical server configuration.

**This login flow is inspired by [Atuin](https://github.com/atuinsh/atuin): QR code onboarding encodes a login URL so users can scan and securely add a new device in a single step. Manual token entry is always supported as fallback.**

---

## 3 · Storage Model

This section primarily describes the conceptual model and the format used by the CLI for local plain-text storage. The `lst-server` component stores this information as records in an SQLite database (`content.db`), using `kind` and `path` (referred to as `item_path` in the database schema) as logical identifiers for content. The actual text content within the database typically resembles Markdown.

```
content/
├─ lists/                    # per‑line anchors
│   └─ groceries.md
├─ notes/                    # whole‑file merge
│   └─ bicycle‑ideas.md
├─ posts/                    # blog, Zola‑compatible
│   └─ 2025‑04‑22‑first‑ride.md
└─ media/                    # images & binary files
    ├─ 6fc9e6e2b4d3.jpg      # originals
    └─ 6fc9e6e2b4d3@512.webp # thumbnails
```

### 3.1 File formats

- **Lists** – bullet lines end with two spaces + `^abc12`; optional YAML front‑matter.
- **Notes** – optional front‑matter (`id`, `title`, `tags`).
- **Posts** – mandatory front‑matter (`id`, `title`, `date`, `draft`, `tags`, `summary`).
- **Media** – binary files named with SHA-256 hash of content; referenced in Markdown via relative paths.

---

## 4 · Sync Logic & Merging

| Kind              | Diff unit | Technique                                   |
| ----------------- | --------- | ------------------------------------------- |
| **lists**         | line      | Automerge CRDT patches                      |
| **notes / posts** | file      | three‑way Git merge; manual fix on conflict |
| **media**         | file      | Git LFS for files up to ~50MB               |

Anchors survive re‑ordering; missing anchors are added automatically (background sync or Neovim Lua autocmd).

---

## 5 · Authentication & Email Delivery

- **Magic‑link flow** – 15 min TTL, single use.
- **SMTP Relay** – default path (Mailgun/Postmark/SES). Configure in `server.toml`:

```toml
[email]
smtp_host = "smtp.mailgun.org"
smtp_user = "postmaster@mg.example.com"
smtp_pass = "${SMTP_PASS}"
sender    = "Lists Bot <no‑reply@mg.example.com>"
```

- Rust crate `lettre` ≥ 0.11 handles async SMTP; if SMTP unset, login link is logged for dev.
- The JWT secret used for signing and verifying JWTs is a critical server configuration item, typically managed securely by the server administrator. It is not exposed to clients.

---

## 6. Content API

The Content API provides CRUD (Create, Read, Update, Delete) operations for managing content records (representing lists, notes, etc.) within the server's SQLite database (`content.db`). All endpoints listed in this section are prefixed with `/api` (e.g., `/api/content`) and **require JWT authentication**.

**Authentication**: Clients must include a valid JWT in the `Authorization` header as a Bearer token:
`Authorization: Bearer <your_jwt_here>`

If authentication fails (missing, invalid, or expired JWT), the server will respond with a `401 Unauthorized` status code.

Content items are identified by a `kind` (string, e.g., "notes", "lists") and a `path` (string, e.g., "recipes/pasta.md", referred to as `item_path` in the database). These form a unique logical key for a content record.

---

### 6.1 Create Content

-   **Method**: `POST`
-   **Path**: `/api/content`
-   **Description**: Creates a new content record in the database.
-   **Request Body (JSON)**:
    ```json
    {
        "kind": "string",
        "path": "string",
        "content": "string"
    }
    ```
    -   `kind`: (string, required) The category or type of content.
    -   `path`: (string, required) The logical path or name for the content within its kind.
    -   `content`: (string, required) The textual content to be stored.
-   **Success Response (201 Created)**:
    ```json
    {
        "message": "Content created successfully.",
        "path": "kind/path" // The logical path of the created content
    }
    ```
-   **Error Responses**:
    -   `400 Bad Request`: Invalid payload (e.g., empty `kind` or `path`, invalid characters).
    -   `401 Unauthorized`: Missing or invalid JWT.
    -   `409 Conflict`: If content with the same `kind` and `path` already exists.
    -   `500 Internal Server Error`: If the record cannot be created in the database.
-   **`curl` Example**:
    ```bash
    JWT="your_jwt_here"
    curl -X POST -H "Content-Type: application/json" \
      -H "Authorization: Bearer $JWT" \
      -d '{ "kind": "notes", "path": "cooking/recipes/pasta.md", "content": "# Pasta Recipe\n..." }' \
      http://localhost:3000/api/content
    ```

---

### 6.2 Read Content

-   **Method**: `GET`
-   **Path**: `/api/content/{kind}/{path}` (Note: `{path}` here can contain slashes, e.g., `topic/sub/file.md`)
-   **Description**: Retrieves the content of a specific record from the database.
-   **Path Parameters**:
    -   `{kind}`: The type/category of content.
    -   `{path}`: The logical path of the content within its kind.
-   **Success Response (200 OK)**:
    -   **Content-Type**: `text/plain; charset=utf-8`
    -   **Body**: The raw textual content of the record.
-   **Error Responses**:
    -   `401 Unauthorized`: Missing or invalid JWT.
    -   `404 Not Found`: If no record matches the given `kind` and `path`.
    -   `500 Internal Server Error`: If the record cannot be read.
-   **`curl` Example**:
    ```bash
    JWT="your_jwt_here"
    curl -X GET -H "Authorization: Bearer $JWT" \
      http://localhost:3000/api/content/notes/cooking/recipes/pasta.md
    ```

---

### 6.3 Update Content

-   **Method**: `PUT`
-   **Path**: `/api/content/{kind}/{path}`
-   **Description**: Updates the content of an existing record in the database.
-   **Path Parameters**:
    -   `{kind}`: The type/category of content.
    -   `{path}`: The logical path of the content.
-   **Request Body (JSON)**:
    ```json
    {
        "content": "string"
    }
    ```
    -   `content`: (string, required) The new textual content for the record.
-   **Success Response (200 OK)**:
    ```json
    {
        "message": "Content updated successfully.",
        "path": "kind/path" // The logical path of the updated content
    }
    ```
-   **Error Responses**:
    -   `400 Bad Request`: Invalid payload.
    -   `401 Unauthorized`: Missing or invalid JWT.
    -   `404 Not Found`: If no record matches the given `kind` and `path`.
    -   `500 Internal Server Error`: If the record cannot be updated.
-   **`curl` Example**:
    ```bash
    JWT="your_jwt_here"
    curl -X PUT -H "Content-Type: application/json" \
      -H "Authorization: Bearer $JWT" \
      -d '{ "content": "# Pasta Recipe\nUpdated ingredients..." }' \
      http://localhost:3000/api/content/notes/cooking/recipes/pasta.md
    ```

---

### 6.4 Delete Content

-   **Method**: `DELETE`
-   **Path**: `/api/content/{kind}/{path}`
-   **Description**: Deletes a specific record from the database.
-   **Path Parameters**:
    -   `{kind}`: The type/category of content.
    -   `{path}`: The logical path of the content.
-   **Success Response (200 OK)**:
    ```json
    {
        "message": "Content deleted successfully.",
        "path": "kind/path" // The logical path of the deleted content
    }
    ```
-   **Error Responses**:
    -   `401 Unauthorized`: Missing or invalid JWT.
    -   `404 Not Found`: If no record matches the given `kind` and `path`.
    -   `500 Internal Server Error`: If the record cannot be deleted.
-   **`curl` Example**:
    ```bash
    JWT="your_jwt_here"
    curl -X DELETE -H "Authorization: Bearer $JWT" \
      http://localhost:3000/api/content/notes/cooking/recipes/pasta.md
    ```

---

## 7 · CLI **`lst`**

```
$ lst help
Usage: lst <command> …

Core – lists
  lst ls                        # list all lists
  lst add   <list> <text>       # add bullet
  lst done  <list> <target>     # mark done (anchor, fuzzy text, or #index)
  lst pipe  <list>              # read items from STDIN

Notes
  lst note new <title>
  lst note add <title> <text>
  lst note open <title>

Posts
  lst post new "<title>"
  lst post list
  lst post publish <slug>

Media
  lst img add <file> --to <doc> # add image to document
  lst img paste --to <doc>      # paste clipboard image
  lst img list <doc>            # list images in document
  lst img rm <doc> <hash>       # remove image reference
```

All commands accept `--json` for automation and return script‑friendly exit codes.

### 6.1 Target Resolution Rules

When using commands like `lst done` that operate on a specific item, the target can be specified in several ways:

1. **Exact anchor** – `^[-A-Za-z0-9]{4,}` matches directly against the anchor ID
2. **Exact text** – Case-insensitive match against the item text
3. **Fuzzy text** – Levenshtein distance ≤2 or contains all words in any order
4. **Numeric index** – `#12` refers to the 12th visible bullet in the list
5. **Interactive picker** – If none of the above resolve uniquely and STDIN is a TTY, presents an interactive selection

Examples:
```bash
lst done groceries oat         # fuzzy → matches "oat milk (x2)"
lst done groceries "#4"        # by index (the 4th unchecked item)
lst done groceries ^d3e1       # explicit anchor (still works)
```

---

## 7 · Client Applications

| Surface              | Highlights                                                                      |
| -------------------- | ------------------------------------------------------------------------------- |
| **Slim GUI (Tauri)** | toggleable, always‑on‑top; Markdown viewer/editor; sync status tray icon.       |
| **Mobile (Tauri 2)** | offline SQLite cache → CRDT; share‑sheet "Add to list"; AppIntents.             |
| **Shortcuts**        | Intents: _AddItem, RemoveItem, GetList, DraftPost_.                             |
| **Voice (AGNO)**     | Whisper transcription → AGNO agent → JSON action (`kind`, `action`, `payload`). |

---

## 8 · Blog Publishing Pipeline

1. `lst post publish <slug>` flips `draft:false`.
2. Server runs `zola build` → `public/`.
3. Reverse proxy serves `/blog/*` static or optionally pushes to GitHub Pages.

---

## 9 · Deployment Recipe (Proxmox LXC)

```bash
# host
pct create 120 debian-12 --cores 2 --memory 1024 --net0 name=eth0,bridge=vmbr0,ip=dhcp
pct start 120

# inside LXC
apt install ca-certificates tzdata
useradd -r -m lst
mkdir /opt/lst && chown lst /opt/lst
# copy single static binary + content/ + server.toml
systemctl enable --now lst.service  # /opt/lst/lst --config /opt/lst/server.toml
```

Proxy with Caddy/Traefik for HTTPS and path routing.

---

## 10 · Configuration

### 10.1 Server Configuration

Server is configured via `/opt/lst/server.toml` (example path):

```toml
[server] # General server settings block
host = "127.0.0.1"
port = 3000
# jwt_secret = "a_very_secret_key_loaded_securely" # This is illustrative; actual secret management varies.

[email] # Email settings for auth token delivery
smtp_host = "smtp.mailgun.org"
smtp_user = "postmaster@mg.example.com"
smtp_pass = "${SMTP_PASS}" # Example: load from environment variable
sender    = "Lists Bot <no-reply@mg.example.com>"

[paths] # Path settings used by the server
# For `lst-server`, `content_dir` (or the directory of lst.toml if content_dir is not set)
# determines where the 'lst_server_data' subdirectory is created.
# This 'lst_server_data' directory stores SQLite database files like 'tokens.db' and 'content.db'.
content_dir = "/srv/lst/data" # Example: a dedicated data directory for the server's databases.

# The [content] block (with `root`, `kinds`, `media_dir`) previously describing
# a file system layout for content is no longer directly used by the server's API
# for content storage, as content is now in an SQLite database (`content.db`).
# `kind` is a field in the database, and media handling via API is not yet specified.
```

### 10.2 Client Configuration

Client is configured via:

- Linux/macOS: `${XDG_CONFIG_HOME:-$HOME/.config}/lst/lst.toml`
- Windows: `%APPDATA%\lst\lst.toml`

```toml
[server]
url = "https://lists.example.com/api"
auth_token = "..." # obtained via magic link flow

[ui]
# default order tried when resolving an item
resolution_order = ["anchor", "exact", "fuzzy", "index", "interactive"]

[fuzzy]
threshold = 0.75          # 0-1 similarity
max_suggestions = 7

[paths]
media_dir = "~/Documents/lst/media"   # override default
```

Environment override: `LST_CONFIG=/path/to/custom.toml`

---

## 11 · Roadmap Snapshot

| Phase                 | Duration | Deliverables                                           |
| --------------------- | -------- | ------------------------------------------------------ |
| **MVP 0.3.1**         | 6 w      | Core server, `lst` CLI (lists), mobile/GUI read‑only   |
| **Offline + CRDT**    | 4 w      | conflict‑free lists across devices                     |
| **Notes & Posts**     | 3 w      | new storage kinds; `lst note` & `lst post`; Zola build |
| **Media Support**     | 2 w      | Image upload, CLI paste, Git LFS backend              |
| **Voice & Shortcuts** | 3 w      | AGNO transcription; App Intents                        |
| **Hardening**         | 2 w      | E2E encryption, invite links, CI, docs                 |

---

## Open Threads

- Set up SMTP provider & DNS (SPF/DKIM).
- Decide if Zola build stays on‑prem or pushes to CDN.
- Future document kinds? (journal, code snippets, etc.)

---

## Version History

+ **v0.4** (2025-04-27): Added `lst note add` command for appending text to notes
+ **v0.3** (2025-04-27): Removed `post` commands and spec entries for posts
+ **v0.2** (2025-04-25): Added notes commands (`lst note new` & `lst note open`)
+ **v0.1** (2025-04-20): Initial specification