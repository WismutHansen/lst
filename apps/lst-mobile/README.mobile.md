# lst-mobile Development Guide

This document explains how to run, test, and build the Tauri mobile app for both iOS and Android.

## Prerequisites

- Rust toolchain with `cargo`
- [`bun`](https://bun.sh/) for managing the React frontend
- [Tauri CLI](https://tauri.app/) (`cargo install tauri-cli`)
- For Android: Android Studio with the NDK and at least one emulator
- For iOS: Xcode. Building on a real device requires an Apple developer account

## Local Development

1. Install JavaScript dependencies:

```bash
cd apps/lst-mobile
bun install
```

2. Format and check the Rust backend:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check -p lst-mobile --message-format=short
```

3. Start the development server in an emulator (starts the React app and the Tauri backend):

```bash
# Android emulator
cargo tauri android dev

# iOS simulator
cargo tauri ios dev
```

## Building Release Artifacts

- **Android APK**

  ```bash
  cargo tauri android build
  # output: src-tauri/target/android/release/apk/*.apk
  ```

- **iOS App**

  ```bash
  cargo tauri ios build
  # output: src-tauri/target/ios/*.app (use Xcode to archive for TestFlight/IPA)
  ```

## Running on Real Devices

### Android

1. Enable USB debugging on the device and connect it via USB.
2. Run:

   ```bash
   cargo tauri android dev
   ```

   The compiled APK is installed and launched on the connected phone.

### iOS

1. Connect the iPhone via USB and trust the computer.
2. Use `cargo tauri ios dev` to generate the Xcode project and open it:

   ```bash
   cargo tauri ios dev --open
   ```

3. In Xcode choose your physical device and press **Run**. Signing requires a valid developer account.

