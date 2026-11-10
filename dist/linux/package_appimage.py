#!/usr/bin/env python3
"""
Pure Python portable Linux AppImage packager for Pigeon.
Packages the AppDir into a self-extracting executable AppImage that runs on any x86_64 Linux distribution.
"""
import os
import sys
import tarfile
import io

APPIMAGE_STUB = b"""#!/usr/bin/env bash
# Pigeon Portable AppImage Self-Extracting Bundle
set -e

APP_NAME="pigeon"
TMP_DIR=$(mktemp -d -t "appimage_pigeon_XXXXXX")
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

# Find offset of archive payload
PAYLOAD_OFFSET=$(awk '/^__PAYLOAD_BELOW__/ {print NR + 1; exit 0; }' "$0")

# Extract archive
tail -n +"$PAYLOAD_OFFSET" "$0" | tar -xz -C "$TMP_DIR"

# Run AppRun
export APPDIR="$TMP_DIR"
export PATH="$TMP_DIR/usr/bin:$PATH"

if [ -f "$TMP_DIR/AppRun" ]; then
    chmod +x "$TMP_DIR/AppRun"
    exec "$TMP_DIR/AppRun" "$@"
elif [ -f "$TMP_DIR/usr/bin/pigeon" ]; then
    chmod +x "$TMP_DIR/usr/bin/pigeon"
    exec "$TMP_DIR/usr/bin/pigeon" "$@"
else
    echo "Error: Application entry point not found in AppImage bundle."
    exit 1
fi

__PAYLOAD_BELOW__
"""

def build_appimage(output_path, bin_path, desktop_path, apprun_path, icon_path):
    tar_stream = io.BytesIO()
    with tarfile.open(fileobj=tar_stream, mode='w:gz') as tar:
        # 1. AppRun
        if os.path.exists(apprun_path):
            with open(apprun_path, 'rb') as f:
                d = f.read()
        else:
            d = b'#!/bin/sh\nHERE="$(dirname "$(readlink -f "${0}")")"\nexec "${HERE}/usr/bin/pigeon" "$@"\n'
        ti = tarfile.TarInfo(name="AppRun")
        ti.size = len(d)
        ti.mode = 0o755
        tar.addfile(ti, io.BytesIO(d))

        # 2. Desktop
        if os.path.exists(desktop_path):
            with open(desktop_path, 'rb') as f:
                d = f.read()
            ti = tarfile.TarInfo(name="pigeon.desktop")
            ti.size = len(d)
            ti.mode = 0o644
            tar.addfile(ti, io.BytesIO(d))

        # 3. Icon
        if os.path.exists(icon_path):
            with open(icon_path, 'rb') as f:
                d = f.read()
            ti = tarfile.TarInfo(name="pigeon.png")
            ti.size = len(d)
            ti.mode = 0o644
            tar.addfile(ti, io.BytesIO(d))
            ti2 = tarfile.TarInfo(name=".DirIcon")
            ti2.size = len(d)
            ti2.mode = 0o644
            tar.addfile(ti2, io.BytesIO(d))
            ti3 = tarfile.TarInfo(name="usr/share/pixmaps/pigeon.png")
            ti3.size = len(d)
            ti3.mode = 0o644
            tar.addfile(ti3, io.BytesIO(d))
            ti4 = tarfile.TarInfo(name="usr/share/icons/hicolor/256x256/apps/pigeon.png")
            ti4.size = len(d)
            ti4.mode = 0o644
            tar.addfile(ti4, io.BytesIO(d))

        # 4. Binary
        if os.path.exists(bin_path):
            with open(bin_path, 'rb') as f:
                d = f.read()
            ti = tarfile.TarInfo(name="usr/bin/pigeon")
            ti.size = len(d)
            ti.mode = 0o755
            tar.addfile(ti, io.BytesIO(d))

    payload = tar_stream.getvalue()

    with open(output_path, 'wb') as f:
        f.write(APPIMAGE_STUB)
        f.write(payload)

    print(f"Created AppImage: {output_path} ({os.path.getsize(output_path)} bytes)")

if __name__ == "__main__":
    dir_path = os.path.dirname(os.path.abspath(__file__))
    root_dir = os.path.abspath(os.path.join(dir_path, "../.."))
    desktop_file = os.path.join(dir_path, "pigeon.desktop")
    apprun_file = os.path.join(dir_path, "appimage/AppRun")
    icon_file = os.path.join(root_dir, "assets/pigeon.png")
    out_appimage = os.path.join(dir_path, "Pigeon-1.11.0-x86_64.AppImage")

    candidate_bins = [
        os.path.join(root_dir, "target/linux/release/pigeon"),
        os.path.join(root_dir, "target/release/pigeon"),
        os.path.join(root_dir, "target/linux/release/usb-x-app"),
        os.path.join(root_dir, "target/release/usb-x-app"),
    ]
    bin_file = None
    for c in candidate_bins:
        if os.path.exists(c):
            with open(c, 'rb') as f:
                if f.read(4) == b"\x7fELF":
                    bin_file = c
                    break

    if not bin_file:
        raise FileNotFoundError(f"Valid Linux ELF binary (target/linux/release/pigeon) not found! Ensure Linux release build is complete.")

    build_appimage(out_appimage, bin_file, desktop_file, apprun_file, icon_file)
