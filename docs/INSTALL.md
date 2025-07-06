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

## Building the Mobile App

The mobile Tauri project requires several GTK development libraries when building on Linux so that `cargo check` can succeed. On Ubuntu-based systems install them with:

```bash
sudo apt-get update
sudo apt-get install -y libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev
```

With the dependencies in place you can run the standard commands:

```bash
cargo fmt --manifest-path apps/lst-mobile/src-tauri/Cargo.toml
cargo check -p lst-mobile --message-format=short
```
