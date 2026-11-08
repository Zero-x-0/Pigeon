#!/bin/bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$DIR/../.." && pwd)"

echo "=========================================="
echo "  USB X - Linux Release Build & Packaging "
echo "=========================================="

echo "[1/4] Compiling Rust release binary..."
cd "$ROOT_DIR"
cargo build --release -p usb-x-app

echo "[2/4] Packaging Debian (.deb)..."
bash "$DIR/build-deb.sh"

echo "[3/4] Packaging Red Hat / Fedora (.rpm)..."
if command -v rpmbuild &> /dev/null; then
    bash "$DIR/build-rpm.sh"
else
    echo "Warning: rpmbuild not installed, skipping .rpm build."
fi

echo "[4/4] Packaging AppImage..."
bash "$DIR/build-appimage.sh"

echo "=========================================="
echo "  All Linux Release Packages Built!       "
echo "=========================================="
ls -lh "$DIR"/*.deb "$DIR"/*.rpm "$DIR"/*.AppImage 2>/dev/null || true

