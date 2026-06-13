#!/bin/bash
set -e

echo "Stopping existing kazamo processes..."
pkill -f target/debug/kazamo || true
sleep 1

echo "Building frontend..."
npm run build

echo "Building Tauri backend..."
cargo build --manifest-path src-tauri/Cargo.toml

echo "Starting kazamo..."
./src-tauri/target/debug/kazamo &
echo "Kazamo started in background."
