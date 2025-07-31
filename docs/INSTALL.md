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

4. Explore the theme system:

   ```bash
   # List available themes
   lst theme list
   
   # Apply a theme
   lst theme apply catppuccin-mocha
   
   # View current theme
   lst theme current
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

Edit `~/.config/lst/lst.toml` to customize content directories, server settings, and themes:

```toml
[theme]
# Set your preferred theme
name = "catppuccin-mocha"

# Override specific colors if desired
[theme.vars]
primary = "#a6e3a1"
```

For more details on configuration options see the [README](../README.md) and [THEMES.md](../THEMES.md).

## Building the Desktop and Mobile Apps

Both desktop and mobile Tauri applications include full theme support with live switching capabilities.

### Desktop App

```bash
cd apps/lst-desktop
bun install
bun tauri dev  # Development mode
bun tauri build  # Production build
```

### Mobile App

The mobile Tauri project requires several GTK development libraries when building on Linux so that `cargo check` can succeed. On Ubuntu-based systems install them with:

```bash
sudo apt-get update
sudo apt-get install -y libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev
```

With the dependencies in place you can run the standard commands:

```bash
cd apps/lst-mobile
bun install
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check -p lst-mobile --message-format=short

# For mobile development
cargo tauri android dev  # Android
cargo tauri ios dev      # iOS
```

Both applications feature:
- **Complete theme integration** with the same themes as the CLI
- **Live theme switching** without restart
- **Mobile-optimized theme selector** (mobile app)
- **Sidebar theme selector** (desktop app)
