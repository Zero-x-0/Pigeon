#!/bin/bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$DIR/../.." && pwd)"
BUILD_DIR="$DIR/deb_root"
VERSION="0.1.0"
ARCH="amd64"

echo "Building Debian Package (.deb) for USB X v$VERSION..."

# Ensure target binary exists
if [ ! -f "$ROOT_DIR/target/release/usb-x-app" ]; then
    echo "Compiling release binary..."
    cargo build --release -p usb-x-app
fi

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR/DEBIAN"
mkdir -p "$BUILD_DIR/usr/bin"
mkdir -p "$BUILD_DIR/usr/share/applications"
mkdir -p "$BUILD_DIR/usr/share/icons/hicolor/scalable/apps"

# Copy control file
cp "$DIR/deb/DEBIAN/control" "$BUILD_DIR/DEBIAN/control"

# Copy binary
cp "$ROOT_DIR/target/release/usb-x-app" "$BUILD_DIR/usr/bin/usb-x"
chmod 755 "$BUILD_DIR/usr/bin/usb-x"

# Copy desktop entry
cp "$DIR/usb-x.desktop" "$BUILD_DIR/usr/share/applications/usb-x.desktop"

# Build .deb
DEB_NAME="usb-x_${VERSION}_${ARCH}.deb"
dpkg-deb --build --root-owner-group "$BUILD_DIR" "$DIR/$DEB_NAME"

echo "Debian package created successfully: $DIR/$DEB_NAME"

