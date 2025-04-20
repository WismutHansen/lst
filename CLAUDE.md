# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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