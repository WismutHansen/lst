# OpenCode.md – Agent Coding Guide

## Build, Lint, and Test

- **Build**: `cargo build`
- **Test all**: `cargo test`
- **Test single**: `cargo test <test_name>`
- **Run**: `cargo run`
- **Lint**: `cargo clippy -- -D warnings`
- **Format**: `cargo fmt --all`

## Code Style Guidelines

- **Language**: Rust for CLI, TypeScript for GUI
- **Formatting**: Use rustfmt, 4-space indent
- **Naming**: snake_case for variables/functions; CamelCase for types
- **Imports**: Order: std lib, external crates, then local modules
- **Types**: Prefer strong typing over generics
- **Error Handling**: Use `Result` and error types, avoid `unwrap`/`expect` in non-test code
- **Documentation**: Use `///` for public items; keep docs clear and reason-focused
- **Test Coverage**: Place tests in-module using Rust's `#[cfg(test)]` when possible
- **Project Structure**: Store user content in Markdown with defined frontmatter, under `content/`
- **Architecture**: Offline-first, plain Markdown storage, supports CRDT sync
- **Do not add license or copyright headers** unless requested

**For agent-based changes:** Focus on root-cause fixes and keep edits minimal and style-consistent.
