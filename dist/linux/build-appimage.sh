#!/bin/bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$DIR/../.." && pwd)"
APP_DIR="$DIR/AppDir"

echo "Building AppImage for USB X..."

if [ ! -f "$ROOT_DIR/target/release/usb-x-app" ]; then
    echo "Compiling release binary..."
    cargo build --release -p usb-x-app
fi

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/usr/bin"
mkdir -p "$APP_DIR/usr/share/applications"

cp "$ROOT_DIR/target/release/usb-x-app" "$APP_DIR/usr/bin/usb-x"
chmod 755 "$APP_DIR/usr/bin/usb-x"

cp "$DIR/usb-x.desktop" "$APP_DIR/usb-x.desktop"
cp "$DIR/usb-x.desktop" "$APP_DIR/usr/share/applications/usb-x.desktop"
cp "$DIR/appimage/AppRun" "$APP_DIR/AppRun"
chmod 755 "$APP_DIR/AppRun"

# Download appimagetool if not present
if ! command -v appimagetool &> /dev/null; then
    if [ ! -f "$DIR/appimagetool" ]; then
        curl -s -L "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage" -o "$DIR/appimagetool"
        chmod +x "$DIR/appimagetool"
    fi
    TOOL="$DIR/appimagetool"
else
    TOOL="appimagetool"
fi

ARCH=x86_64 "$TOOL" "$APP_DIR" "$DIR/USB_X-x86_64.AppImage"
echo "AppImage created successfully: $DIR/USB_X-x86_64.AppImage"

