#!/bin/bash
set -e

echo "=== Building Kazamo AppImage ==="

# Step 1: Build frontend
echo "[1/4] Building frontend..."
npm run build

# Step 2: Build Tauri (release + AppImage)
echo "[2/4] Building Tauri release..."
cargo tauri build --bundles appimage

# Step 3: Fix bundled libraries (INIT pointer corruption fix)
echo "[3/4] Fixing AppImage bundled libraries..."
bash fix-appimage.sh

# Step 4: Repackage AppImage
echo "[4/4] Repackaging AppImage..."
APPIMAGE_DIR="src-tauri/target/release/bundle/appimage"
cd "$APPIMAGE_DIR"
appimagetool Kazamo.AppDir Kazamo_0.1.0_amd64.AppImage
cd - > /dev/null

echo ""
echo "=== Build complete ==="
echo "AppImage: $APPIMAGE_DIR/Kazamo_0.1.0_amd64.AppImage"
echo "Size: $(du -h $APPIMAGE_DIR/Kazamo_0.1.0_amd64.AppImage | cut -f1)"
