#!/bin/bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$DIR/../.." && pwd)"
RPM_BUILD_DIR="$DIR/rpm_build"

echo "Building RPM Package (.rpm) for USB X..."

if [ ! -f "$ROOT_DIR/target/release/usb-x-app" ]; then
    echo "Compiling release binary..."
    cargo build --release -p usb-x-app
fi

mkdir -p "$RPM_BUILD_DIR/SOURCES" "$RPM_BUILD_DIR/SPECS" "$RPM_BUILD_DIR/RPMS" "$RPM_BUILD_DIR/BUILD" "$RPM_BUILD_DIR/SRPMS"

cp "$ROOT_DIR/target/release/usb-x-app" "$RPM_BUILD_DIR/SOURCES/usb-x"
cp "$DIR/usb-x.desktop" "$RPM_BUILD_DIR/SOURCES/usb-x.desktop"
cp "$DIR/rpm/usb-x.spec" "$RPM_BUILD_DIR/SPECS/usb-x.spec"

rpmbuild --define "_topdir $RPM_BUILD_DIR" -bb "$RPM_BUILD_DIR/SPECS/usb-x.spec"

cp "$RPM_BUILD_DIR/RPMS/x86_64/"*.rpm "$DIR/"
echo "RPM package created successfully in $DIR/"

