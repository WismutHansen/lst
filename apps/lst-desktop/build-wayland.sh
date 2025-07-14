#!/bin/bash

# Environment variables for Tauri on Hyprland/Wayland
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export GDK_BACKEND=x11
export DISPLAY=:0

# Additional environment variables that can help
export LIBGL_ALWAYS_SOFTWARE=1
export MESA_GL_VERSION_OVERRIDE=4.6

echo "Starting Tauri with Hyprland-compatible settings..."
exec bun tauri build

