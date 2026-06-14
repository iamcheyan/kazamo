#!/bin/bash
set -e

# fix-appimage.sh - Fix AppImage bundled libraries INIT pointer corruption
#
# Problem: Tauri's linuxdeploy bundles system libraries (WebkitGTK, GTK, etc.)
# into the AppImage, but these libraries have corrupted ELF DT_INIT pointers
# that cause segfaults during dynamic linker initialization.
#
# Fix: Remove system-provided libraries from the AppDir, keeping only custom
# libraries (libggml, libonnxruntime). The system's properly-compiled versions
# will be loaded instead via the standard library search path.

APPIMAGE_DIR="src-tauri/target/release/bundle/appimage"
APPDIR="$APPIMAGE_DIR/Kazamo.AppDir"
LIB_DIR="$APPDIR/usr/lib"

# Custom libraries that must be kept (not available on the system)
CUSTOM_LIBS=(
    "libggml-base.so"
    "libggml-cpu.so"
    "libggml.so"
    "libonnxruntime.so"
)

if [ ! -d "$APPDIR" ]; then
    echo "Error: AppDir not found at $APPDIR"
    echo "Run 'cargo tauri build' first."
    exit 1
fi

echo "Fixing AppImage bundled libraries..."

# Step 1: Save custom libraries and Kazamo resources
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

for lib in "${CUSTOM_LIBS[@]}"; do
    if [ -f "$LIB_DIR/$lib" ]; then
        cp "$LIB_DIR/$lib" "$TEMP_DIR/"
        echo "  Saved custom library: $lib"
    else
        echo "  Warning: Custom library not found: $lib"
    fi
done

# Save Kazamo resources subdirectory
if [ -d "$LIB_DIR/Kazamo" ]; then
    cp -r "$LIB_DIR/Kazamo" "$TEMP_DIR/"
    echo "  Saved Kazamo resources"
fi

# Step 2: Remove all shared libraries from the AppDir
find "$LIB_DIR" -maxdepth 1 -type f -name "*.so*" -delete
find "$LIB_DIR" -maxdepth 1 -type l -name "*.so*" -delete
find "$LIB_DIR" -maxdepth 1 -type l -name "im-*" -delete 2>/dev/null || true

echo "  Removed system-provided libraries"

# Step 3: Restore custom libraries
for lib in "${CUSTOM_LIBS[@]}"; do
    if [ -f "$TEMP_DIR/$lib" ]; then
        cp "$TEMP_DIR/$lib" "$LIB_DIR/"
    fi
done

# Restore Kazamo resources
if [ -d "$TEMP_DIR/Kazamo" ]; then
    cp -r "$TEMP_DIR/Kazamo" "$LIB_DIR/"
fi

echo "  Restored custom libraries"

# Step 4: Fix the AppRun hook to not force bundled library paths
HOOK_FILE="$APPDIR/apprun-hooks/linuxdeploy-plugin-gtk.sh"
if [ -f "$HOOK_FILE" ]; then
    cat > "$HOOK_FILE" << 'HOOKEOF'
#! /usr/bin/env bash

# Modified hook: use system GTK libraries instead of bundled ones.
# The bundled system libraries had corrupted ELF INIT pointers.

gsettings get org.gnome.desktop.interface gtk-theme 2> /dev/null | grep -qi "dark" && GTK_THEME_VARIANT="dark" || GTK_THEME_VARIANT="light"
APPIMAGE_GTK_THEME="${APPIMAGE_GTK_THEME:-"Adwaita:$GTK_THEME_VARIANT"}"

export APPDIR="${APPDIR:-"$(dirname "$(realpath "$0")")"}"
export GTK_THEME="$APPIMAGE_GTK_THEME"
export XDG_DATA_DIRS="$APPDIR/usr/share:/usr/share:$XDG_DATA_DIRS"
HOOKEOF
    chmod +x "$HOOK_FILE"
    echo "  Fixed AppRun hook"
fi

echo ""
echo "AppImage fix complete."
echo "Remaining libraries in $LIB_DIR:"
ls -la "$LIB_DIR"/*.so* 2>/dev/null || echo "  (none)"
echo ""
echo "To rebuild the fixed AppImage:"
echo "  cd $APPIMAGE_DIR && appimagetool Kazamo.AppDir Kazamo_0.1.0_amd64.AppImage"
