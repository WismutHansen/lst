# lst - personal lists & notes App – Specification v0.3

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

## 3 · Storage Model

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

---

## 6 · CLI **`lst`**

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

Server is configured via `/opt/lst/server.toml`:

```toml
[server]
host = "127.0.0.1"
port = 3000

[email]
smtp_host = "smtp.mailgun.org"
smtp_user = "postmaster@mg.example.com"
smtp_pass = "${SMTP_PASS}"
sender    = "Lists Bot <no-reply@mg.example.com>"

[content]
root = "content"
kinds = ["lists", "notes", "posts"]
media_dir = "media"
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

- **v0.2** (2025-04-20): Added media support, client configuration, fuzzy item targeting
- **v0.1** (2025-04-20): Initial specification