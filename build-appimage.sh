#!/bin/bash
set -e

# Disable stripping to fix modern ELF relocation format (.relr.dyn) errors on modern Linux distros (Fedora 44)
export NO_STRIP=true

echo "=== Building Kazamo AppImage ==="

# Step 1: Build Tauri (frontend + release + AppImage)
# Note: tauri build automatically runs beforeBuildCommand (npm run build)
echo "[1/3] Building Tauri release + AppImage..."
npx tauri build --bundles appimage

# Step 2: Fix bundled libraries (INIT pointer corruption fix)
echo "[2/3] Fixing AppImage bundled libraries..."
bash fix-appimage.sh

# Step 3: Repackage AppImage
echo "[3/3] Repackaging AppImage..."
APPIMAGE_DIR="src-tauri/target/release/bundle/appimage"
cd "$APPIMAGE_DIR"

ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
    ARCH_SUFFIX="amd64"
else
    ARCH_SUFFIX="$ARCH"
fi
APPIMAGE_NAME="Kazamo_0.1.0_${ARCH_SUFFIX}.AppImage"

ARCH=$ARCH appimagetool Kazamo.AppDir "$APPIMAGE_NAME"
cd - > /dev/null

echo ""
echo "=== Build complete ==="
echo "AppImage: $APPIMAGE_DIR/$APPIMAGE_NAME"
echo "Size: $(du -h $APPIMAGE_DIR/$APPIMAGE_NAME | cut -f1)"
