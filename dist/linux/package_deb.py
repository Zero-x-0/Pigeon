#!/usr/bin/env python3
"""
Pure Python .deb package builder for Pigeon.
Creates a valid Debian package (.deb) without requiring dpkg-deb.
"""
import os
import sys
import tarfile
import io
import time

def build_ar_header(filename, size, mtime=None, mode=0o100644, uid=0, gid=0):
    if mtime is None:
        mtime = int(time.time())
    name_field = f"{filename:<16}"[:16]
    mtime_field = f"{mtime:<12}"[:12]
    uid_field = f"{uid:<6}"[:6]
    gid_field = f"{gid:<6}"[:6]
    mode_field = f"{oct(mode)[2:]:<8}"[:8]
    size_field = f"{size:<10}"[:10]
    magic = "`\n"
    return (name_field + mtime_field + uid_field + gid_field + mode_field + size_field + magic).encode('ascii')

def create_tar_gz(files_dict):
    out = io.BytesIO()
    with tarfile.open(fileobj=out, mode='w:gz') as tar:
        for arcname, data, mode in files_dict:
            ti = tarfile.TarInfo(name=arcname)
            ti.size = len(data)
            ti.mtime = int(time.time())
            ti.mode = mode
            ti.uid = 0
            ti.gid = 0
            ti.uname = "root"
            ti.gname = "root"
            tar.addfile(ti, io.BytesIO(data))
    return out.getvalue()

def build_deb(output_path, control_content, binary_path, desktop_path, icon_path):
    # 1. debian-binary
    debian_binary = b"2.0\n"

    # 2. control.tar.gz
    control_tar_gz = create_tar_gz([
        ("./control", control_content.encode('utf-8'), 0o644)
    ])

    # 3. data.tar.gz
    data_files = []
    if os.path.exists(binary_path):
        with open(binary_path, 'rb') as f:
            bin_data = f.read()
        data_files.append(("./usr/bin/pigeon", bin_data, 0o755))
    
    if os.path.exists(desktop_path):
        with open(desktop_path, 'rb') as f:
            desk_data = f.read()
        data_files.append(("./usr/share/applications/pigeon.desktop", desk_data, 0o644))

    if os.path.exists(icon_path):
        with open(icon_path, 'rb') as f:
            icon_data = f.read()
        data_files.append(("./usr/share/pixmaps/pigeon.png", icon_data, 0o644))
        data_files.append(("./usr/share/icons/hicolor/256x256/apps/pigeon.png", icon_data, 0o644))

    data_tar_gz = create_tar_gz(data_files)

    # Combine into ar archive
    with open(output_path, 'wb') as f:
        f.write(b"!<arch>\n")

        # debian-binary member
        f.write(build_ar_header("debian-binary", len(debian_binary)))
        f.write(debian_binary)
        if len(debian_binary) % 2 != 0:
            f.write(b"\n")

        # control.tar.gz member
        f.write(build_ar_header("control.tar.gz", len(control_tar_gz)))
        f.write(control_tar_gz)
        if len(control_tar_gz) % 2 != 0:
            f.write(b"\n")

        # data.tar.gz member
        f.write(build_ar_header("data.tar.gz", len(data_tar_gz)))
        f.write(data_tar_gz)
        if len(data_tar_gz) % 2 != 0:
            f.write(b"\n")

    print(f"Created Debian package: {output_path} ({os.path.getsize(output_path)} bytes)")

if __name__ == "__main__":
    dir_path = os.path.dirname(os.path.abspath(__file__))
    root_dir = os.path.abspath(os.path.join(dir_path, "../.."))
    control_file = os.path.join(dir_path, "deb/DEBIAN/control")
    desktop_file = os.path.join(dir_path, "pigeon.desktop")
    icon_file = os.path.join(root_dir, "assets/pigeon.png")
    out_deb = os.path.join(dir_path, "pigeon_1.11.0_amd64.deb")

    with open(control_file, 'r', encoding='utf-8') as f:
        control_text = f.read()

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

    build_deb(out_deb, control_text, bin_file, desktop_file, icon_file)
