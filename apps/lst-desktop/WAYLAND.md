# Wayland Support for lst-desktop

This document explains how to build and run lst-desktop with native Wayland support.

## Configuration

The project includes a separate Wayland-optimized configuration file: `src-tauri/tauri.wayland.json`

### Key differences from the default configuration:
- **Window decorations**: Enabled (`decorations: true`) for proper Wayland window management
- **Transparency**: Disabled (`transparent: false`) for better Wayland compatibility
- **Shadows**: Enabled (`shadow: true`) for native compositor shadows
- **Dependencies**: Additional Wayland libraries included in the Linux bundle

## Building for Wayland

### Prerequisites
Ensure you have the necessary Wayland development libraries installed:

```bash
# Debian/Ubuntu
sudo apt install libwayland-dev libwayland-client0 libwayland-cursor0 libwayland-egl1

# Fedora
sudo dnf install wayland-devel

# Arch
sudo pacman -S wayland
```

### Development
To run the application in development mode with Wayland support:

```bash
# Using npm scripts
bun run dev:wayland

# Or using the helper script
./run-wayland.sh bun run dev:wayland
```

### Building
To build the application with Wayland configuration:

```bash
# Using npm scripts
bun run build:wayland

# Or using the helper script
./run-wayland.sh bun run build:wayland
```

## Environment Variables

The `run-wayland.sh` script sets the following environment variables:
- `GDK_BACKEND=wayland`: Forces GTK to use Wayland backend
- `WAYLAND_DISPLAY=wayland-0`: Specifies the Wayland display
- `XDG_SESSION_TYPE=wayland`: Indicates a Wayland session
- `MOZ_ENABLE_WAYLAND=1`: Enables Wayland support for WebKit
- `TAURI_CONFIG=tauri.wayland.json`: Uses the Wayland-specific Tauri configuration

## Troubleshooting

### Window appears without decorations
Ensure your Wayland compositor supports server-side decorations (SSD). Most modern compositors like GNOME's Mutter and KDE's KWin support this.

### Application falls back to X11
Check that:
1. You're running under a Wayland session (`echo $XDG_SESSION_TYPE` should output "wayland")
2. All required Wayland libraries are installed
3. The `GDK_BACKEND` environment variable is set to "wayland"

### Performance issues
If you experience performance issues, try:
1. Ensuring GPU drivers are properly installed
2. Checking compositor settings for VSync and triple buffering
3. Running with `WAYLAND_DEBUG=1` to see detailed Wayland protocol messages