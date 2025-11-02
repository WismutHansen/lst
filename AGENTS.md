# AGENTS.md - Development Guide for lst

This file provides guidance to humans and AI Agents when working with code in this repository.

## Project: lst - personal lists & notes App

## Issue Tracking with bd (beads)

**IMPORTANT**: This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why bd?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Auto-syncs to JSONL for version control
- Agent-optimized: JSON output, ready work detection, discovered-from links
- Prevents duplicate tracking systems and confusion

### Quick Start

**Check for ready work:**

```bash
bd ready --json
```

**Create new issues:**

```bash
bd create "Issue title" -t bug|feature|task -p 0-4 --json
bd create "Issue title" -p 1 --deps discovered-from:bd-123 --json
```

**Claim and update:**

```bash
bd update bd-42 --status in_progress --json
bd update bd-42 --priority 1 --json
```

**Complete work:**

```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task**: `bd update <id> --status in_progress`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`
6. **Commit together**: Always commit the `.beads/issues.jsonl` file together with the code changes so issue state stays in sync with code state

### Auto-Sync

bd automatically syncs with git:

- Exports to `.beads/issues.jsonl` after changes (5s debounce)
- Imports from JSONL when newer (e.g., after `git pull`)
- No manual export/import needed!

### MCP Server (Recommended)

If using Claude Code, Codex, opencode or MCP-compatible clients, install the beads MCP server:

```bash
pip install beads-mcp
```

Add to MCP config (e.g., `~/.config/claude/config.json`):

```json
{
  "beads": {
    "command": "beads-mcp",
    "args": []
  }
}
```

Then use `mcp__beads__*` functions instead of CLI commands.

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems

For more details, see README.md and QUICKSTART.md.

### Build/Test Commands

- **Rust**: `cargo check` (validate), `cargo test` (all tests), `cargo test <name>` (single test)
- **Desktop**: `cd apps/lst-desktop && bun tauri dev` (dev), `bun tauri build` (build), `bun run lint` (lint)
- **Mobile**: `cd apps/lst-mobile && bun tauri dev` (dev), `bun tauri build` (build), `bun run lint` (lint)
- **Sync Smoke Test**: `./scripts/sync-smoke.sh` (runs local server + daemon smoke test)

## Code Style

- **Rust**: Use `anyhow::Result` for errors, `thiserror` for custom errors, snake_case naming
- **TypeScript**: Double quotes, semicolons required, React functional components with hooks
- **Imports**: Group by external/internal, use workspace dependencies in Cargo.toml
- **Error Handling**: Rust uses `?` operator, TypeScript uses try/catch with proper error types
- **Naming**: Rust snake_case, TypeScript camelCase, components PascalCase

### Architecture

- **Paradigm**: Everything is text/files, markdown as source of truth (except mobile SQLite)
- **Sync**: CRDT-based encrypted sync to server for multi-device support
- **Structure**: Workspace with CLI, server, desktop/mobile Tauri apps, protocol crates
- **Frontend**: React + TypeScript + Tailwind + Radix UI + CodeMirror for editing
- **Theming**: Comprehensive base16/base24 theme system across all applications

### Testing

- Run `cargo check` after Rust changes to validate compilation
- Use `bun run lint` in app directories to check TypeScript style
- Test individual Rust modules with `cargo test <module_name>`

#### End-to-End Type Safety with Tauri Specta

This project uses Tauri Specta for automatic TypeScript generation from Rust types, ensuring type safety between the Rust backend and TypeScript frontend.

##### Requirements for Specta Integration

1. **Dependencies**: All crates that expose types to the frontend must include:

   ```toml
   specta = { version = "2.0.0-rc.22", features = ["uuid", "chrono"] }
   ```

2. **Type Derivation**: All structs/enums exposed to TypeScript must derive `specta::Type`:

   ```rust
   use specta::Type;

   #[derive(Debug, Serialize, Deserialize, Type)]
   pub struct MyStruct {
       // fields
   }
   ```

3. **Command Functions**: Tauri commands must use `#[specta::specta]` annotation:

   ```rust
   #[tauri::command]
   #[specta::specta]
   fn my_command() -> Result<MyStruct, String> {
       // implementation
   }
   ```

4. **Supported Types**: Use types with Specta support:
   - Built-in types: `String`, `i32`, `bool`, `Vec<T>`, `Option<T>`, etc.
   - External types require feature flags: `Uuid` (uuid feature), `DateTime<Utc>` (chrono feature)
   - Custom types must derive `Type`

5. **Code Generation**: TypeScript bindings are automatically generated to `src/bindings.ts` during development builds, providing full type safety for frontend API calls.

### Theme System

The project includes a comprehensive theming system with the following components:

#### Core Theme System (`lst-core/src/theme.rs`)

- **Base16/Base24 support**: Standard color specifications for consistent theming
- **Built-in themes**: Catppuccin, Gruvbox, Nord, Solarized, and more
- **Theme inheritance**: Support for theme variants and overrides
- **CSS generation**: Automatic CSS custom properties generation

#### CLI Theme Commands

```bash
lst theme list          # List available themes
lst theme apply <name>  # Apply a theme
lst theme current       # Show current theme
lst theme info <name>   # Get theme details
lst theme validate <file> # Validate theme file
```

#### CLI User Management Commands (requires lst-server)

```bash
lst user list                    # List all users
lst user create <email>          # Create a new user
lst user delete <email>          # Delete a user (with confirmation)
lst user delete <email> --force  # Delete a user without confirmation
lst user update <email> --name <name> --enabled <true/false>  # Update user info
lst user info <email>            # Show detailed user information
```

#### Frontend Integration

- **Desktop**: Theme selector in sidebar with live switching
- **Mobile**: Theme selector in Settings panel with bottom sheet UI
- **CSS Custom Properties**: Dynamic color injection without restart
- **React Hooks**: `useTheme` hook for theme state management

#### Implementation Notes

- Theme commands must be registered in both `collect_commands!` and `invoke_handler!` macros
- Theme data types must derive `specta::Type` for TypeScript generation
- CSS variables are injected at runtime via `applyThemeToDOM` function
- Hardcoded colors should be replaced with CSS custom properties
