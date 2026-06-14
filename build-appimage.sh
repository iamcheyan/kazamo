#!/bin/bash
set -e

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
appimagetool Kazamo.AppDir Kazamo_0.1.0_amd64.AppImage
cd - > /dev/null

echo ""
echo "=== Build complete ==="
echo "AppImage: $APPIMAGE_DIR/Kazamo_0.1.0_amd64.AppImage"
echo "Size: $(du -h $APPIMAGE_DIR/Kazamo_0.1.0_amd64.AppImage | cut -f1)"
