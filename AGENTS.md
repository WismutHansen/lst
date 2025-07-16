# AGENTS.md - Development Guide for lst

This file provides guidance to humans and AI Agents when working with code in this repository.

## Project: lst - personal lists & notes App

### Build/Test Commands

- **Rust**: `cargo check` (validate), `cargo test` (all tests), `cargo test <name>` (single test)
- **Desktop**: `cd apps/lst-desktop && bun tauri dev` (dev), `bun tauri build` (build), `bun run lint` (lint)
- **Mobile**: `cd apps/lst-mobile && bun tauri dev` (dev), `bun tauri build` (build), `bun run lint` (lint)

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

