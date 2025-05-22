# Installation Guide

This guide explains how to install **lst**, the personal lists & notes application, from source.

## Prerequisites

- **Rust toolchain**: Install the latest stable Rust from [rust-lang.org](https://www.rust-lang.org/tools/install).
- **Git**: Required for cloning the repository.

## Building the CLI

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/lst.git
   cd lst
   ```

2. Build and install the `lst` CLI:

   ```bash
   cargo install --path .
   ```

   This compiles an optimized release build and places the binary in your Cargo bin directory (usually `~/.cargo/bin`).

3. Verify the installation:

   ```bash
   lst --help
   ```

## Running the Server

The repository also contains a small API server. To build and run it:

```bash
cargo run --bin lst-server
```

## Configuration

Copy the example configuration to your config directory and adjust paths as needed:

```bash
mkdir -p ~/.config/lst
cp examples/lst.toml ~/.config/lst/lst.toml
```

Edit `~/.config/lst/lst.toml` to customize content directories or server settings.

For more details on configuration options see the [README](README.md).
