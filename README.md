# lst - Personal Lists & Notes App

`lst` is a personal lists, notes, and blog posts management application with a focus on plain-text storage, offline-first functionality, and multi-device synchronization.

## Installation

### From Source

```bash
git clone https://github.com/yourusername/lst.git
cd lst
cargo install --path .
```

## Features

- Manage to-do lists from the command line
- Store everything as plain Markdown files
- Work offline, sync when connected
- Fuzzy matching for item targeting
- Supports multiple document types: lists, notes, and blog posts

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

# Read items from stdin
cat items.txt | lst pipe <list_name>
```

### Notes

```bash
# Create a new note
lst note new "<title>"

# Open a note in your editor
lst note open "<title>"
```
  
### Posts (Coming Soon)
```bash
# Create a new blog post
lst post new "<title>"

# Publish a blog post
lst post publish <slug>
```

### Media Support (Coming Soon)

```bash
# Add an image to a document
lst img add <file.jpg> --to <document> [--caption "Optional caption"]

# Paste image from clipboard
lst img paste --to <document> [--caption "Optional caption"]
```

## Configuration

`lst` uses a TOML configuration file located at `~/.config/lst/lst.toml`. You can override the config file location by setting the `LST_CONFIG` environment variable.

Configuration changes take effect the next time you run a command. If you change the `content_dir` option, existing data will remain in the old location, and you'll need to move it manually to the new location.

### Configuration Options

#### Server Configuration

```toml
[server]
# URL of the sync server API
url = "https://lists.example.com/api"

# Authentication token (obtained via magic link flow)
auth_token = "your-auth-token"
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
# Base directory for all content (lists, notes, posts, media)
# Default: current working directory
content_dir = "~/Documents/lst"

# Override the media directory location (relative to content_dir)
# Default: "$content_dir/media"
media_dir = "~/Documents/lst/media"
```

## Example Configuration

An example configuration file is provided in the `examples/lst.toml` file in the repository. You can copy this file to `~/.config/lst/lst.toml` and customize it to your needs.

## Storage Format

All data is stored as Markdown files in the content directory (which can be configured in `lst.toml`):

```
content/
├─ lists/                    # per-line anchors
│   └─ groceries.md
├─ notes/                    # whole-file merge
│   └─ bicycle-ideas.md
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

- [ ] Milk  ^XMuD1
- [x] Bread  ^lkJzl
- [ ] Eggs  ^w5Cdq
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.