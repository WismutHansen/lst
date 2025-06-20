# RULES.md

This file provides guidance to humans and AI Agents when working with code in this repository.

## Project: lst - personal lists & notes App

### Build/Lint/Test Commands

- Build: `cargo build`
- Test all: `cargo test`
- Test single: `cargo test <test_name>`
- Run: `cargo run`
- Clippy (lint): `cargo clippy -- -D warnings`
- Format: `cargo fmt --all`

### Code Style Guidelines

- **Language**: Rust for CLI and server, TypeScript for Tauri GUI
- **Formatting**: Follow rustfmt conventions, 4 space indentation
- **Naming**: snake_case for variables/functions, CamelCase for types
- **Imports**: Group standard lib first, then external crates, then local modules
- **Error Handling**: Use Result with descriptive error types, avoid unwrap/expect
- **Types**: Prefer strong typing over generic types
- **Documentation**: Document public APIs with triple-slash comments (///)
- **Files**: Store user content in Markdown with defined frontmatter schemas
- **Architecture**: Follow client-server model with CRDT sync capabilities

### End-to-End Type Safety with Tauri Specta

This project uses Tauri Specta for automatic TypeScript generation from Rust types, ensuring type safety between the Rust backend and TypeScript frontend.

#### Requirements for Specta Integration:

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

