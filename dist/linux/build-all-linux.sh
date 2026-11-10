#!/bin/bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$DIR/../.." && pwd)"

export PATH="$HOME/.cargo/bin:$PATH"

echo "=========================================="
echo "  Pigeon - Linux Release Build & Packaging "
echo "=========================================="

echo "[1/4] Compiling Rust release binary..."
cd "$ROOT_DIR"
cargo build --release -p pigeon --target-dir target/linux

echo "[2/4] Packaging Debian (.deb)..."
python3 "$DIR/package_deb.py"

echo "[3/4] Packaging Red Hat / Fedora (.rpm)..."
python3 "$DIR/package_rpm.py"

echo "[4/4] Packaging AppImage..."
python3 "$DIR/package_appimage.py"

mkdir -p "$ROOT_DIR/release"
cp -f "$DIR"/*.deb "$DIR"/*.rpm "$DIR"/*.AppImage "$ROOT_DIR/release/"

echo "=========================================="
echo "  All Linux Release Packages Built!       "
echo "=========================================="
ls -lh "$ROOT_DIR/release"
