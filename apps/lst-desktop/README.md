# lst Desktop App

The desktop application for `lst` - a personal lists and notes management app built with Tauri, React, and TypeScript.

## Features

- **Cross-platform**: Runs on Windows, macOS, and Linux
- **Real-time updates**: Automatically refreshes when CLI commands make changes
- **Rich text editing**: CodeMirror-based editor with syntax highlighting
- **Theme system**: Comprehensive theming with base16/base24 color schemes
- **Vim mode**: Optional Vim-like keybindings for power users
- **Live theme switching**: Change themes instantly without restart

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (v18 or later)
- [Bun](https://bun.sh/) (recommended) or npm

### Setup

```bash
# Install dependencies
bun install

# Start development server
bun tauri dev

# Build for production
bun tauri build
```

### Theme System

The desktop app includes a comprehensive theme system:

- **Built-in themes**: Catppuccin, Gruvbox, Nord, Solarized, and more
- **Live switching**: Theme selector in the sidebar for instant changes
- **CSS custom properties**: Dynamic color injection without restart
- **Base16 compatibility**: Supports standard base16 and base24 color schemes

### Architecture

- **Frontend**: React + TypeScript + Tailwind CSS
- **Backend**: Rust with Tauri for native system integration
- **State management**: React hooks with real-time CLI synchronization
- **Styling**: CSS custom properties with theme-aware components

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
