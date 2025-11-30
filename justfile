set positional-arguments

# Display help
help:
    just -l

# format code
fmt:
    cargo fmt -- --config imports_granularity=Item

fix *args:
    cargo clippy --fix --all-features --tests --allow-dirty "$@"

clippy:
    cargo clippy --all-features --tests "$@"

# Fetch dependencies (run before install tasks)
fetch:
    rustup show active-toolchain
    cargo fetch

# Install all components (CLI, syncd, MCP, server, desktop)
install-all: fetch
    cargo install --path crates/lst-cli
    cargo install --path crates/lst-syncd
    cargo install --path crates/lst-mcp
    cargo install --path crates/lst-server
    cd apps/lst-desktop && bun install && bun tauri build

# Install client components (CLI, syncd, MCP, desktop)
install-client: fetch
    cargo install --path crates/lst-cli
    cargo install --path crates/lst-syncd
    cargo install --path crates/lst-mcp
    cd apps/lst-desktop && bun install && bun tauri build

# Install minimal CLI only (no Tauri/GUI features)
install-minimal: fetch
    cargo install --path crates/lst-cli --no-default-features --features lists

# Run `cargo nextest` since it's faster than `cargo test`, though including
# --no-fail-fast is important to ensure all tests are run.
#
# Run `cargo install cargo-nextest` if you don't have it installed.
test:
    cargo nextest run --no-fail-fast
