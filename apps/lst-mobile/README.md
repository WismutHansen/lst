# lst Mobile App

The mobile application for `lst` - a personal lists and notes management app built with Tauri, React, and TypeScript for iOS and Android.

## Features

- **Cross-platform mobile**: Runs on iOS and Android
- **Offline-first**: SQLite storage for full offline capability
- **Sync integration**: Real-time synchronization with lst-server
- **Touch-optimized UI**: Mobile-friendly interface with gesture support
- **Theme system**: Full theming support with mobile-optimized theme selector
- **Settings panel**: Comprehensive configuration including sync and themes

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (v18 or later)
- [Bun](https://bun.sh/) (recommended) or npm
- Platform-specific tools:
  - **iOS**: Xcode and iOS SDK
  - **Android**: Android Studio and Android SDK

### Setup

```bash
# Install dependencies
bun install

# Start development server (mobile simulator)
bun tauri dev

# Build for production
bun tauri build
```

### Mobile-Specific Features

#### Theme System
- **Mobile-friendly selector**: Bottom sheet interface for theme selection
- **Settings integration**: Theme options accessible through Settings panel
- **Touch-optimized**: Large buttons and touch targets for mobile interaction
- **Consistent theming**: Same themes as desktop with mobile-optimized UI

#### Sync Integration
- **Background sync**: Automatic synchronization with lst-server
- **Offline capability**: Full functionality without network connection
- **Conflict resolution**: CRDT-based merge for multi-device editing
- **Authentication flow**: Email-based token authentication

#### Storage
- **SQLite database**: Local storage for lists, notes, and sync state
- **Encrypted sync**: End-to-end encryption for server synchronization
- **Efficient queries**: Optimized database schema for mobile performance

### Architecture

- **Frontend**: React + TypeScript + Tailwind CSS
- **Backend**: Rust with Tauri for native mobile integration
- **Database**: SQLite for local storage and sync state
- **Sync**: CRDT-based synchronization with lst-server
- **UI**: Mobile-optimized components with touch gestures

## Platform-Specific Setup

### iOS Development

```bash
# Add iOS target
rustup target add aarch64-apple-ios

# Generate iOS project
bun tauri ios init

# Run on iOS simulator
bun tauri ios dev
```

### Android Development

```bash
# Add Android targets
rustup target add aarch64-linux-android armv7-linux-androideabi

# Generate Android project
bun tauri android init

# Run on Android emulator
bun tauri android dev
```

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
