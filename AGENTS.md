# AGENTS.md - Development Guide for lst

## Build/Test Commands
- **Rust**: `cargo check` (validate), `cargo test` (all tests), `cargo test <name>` (single test)
- **Desktop**: `cd apps/lst-desktop && bun dev` (dev), `bun build` (build), `bun run lint` (lint)
- **Mobile**: `cd apps/lst-mobile && bun dev` (dev), `bun build` (build), `bun run lint` (lint)
- **Tauri**: `bun run tauri dev` (desktop dev), `bun run tauri build` (build)

## Code Style
- **Rust**: Use `anyhow::Result` for errors, `thiserror` for custom errors, snake_case naming
- **TypeScript**: Double quotes, semicolons required, React functional components with hooks
- **Imports**: Group by external/internal, use workspace dependencies in Cargo.toml
- **Error Handling**: Rust uses `?` operator, TypeScript uses try/catch with proper error types
- **Naming**: Rust snake_case, TypeScript camelCase, components PascalCase

## Architecture
- **Paradigm**: Everything is text/files, markdown as source of truth (except mobile SQLite)
- **Sync**: CRDT-based encrypted sync to server for multi-device support
- **Structure**: Workspace with CLI, server, desktop/mobile Tauri apps, protocol crates
- **Frontend**: React + TypeScript + Tailwind + Radix UI + CodeMirror for editing

## Testing
- Run `cargo check` after Rust changes to validate compilation
- Use `bun run lint` in app directories to check TypeScript style
- Test individual Rust modules with `cargo test <module_name>`