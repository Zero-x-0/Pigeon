#!/usr/bin/env python3
"""
Pure Python RPM package builder for Pigeon.
Creates a valid RPM v3/v4 package (.rpm) for Fedora / RHEL / CentOS / openSUSE.
"""
import os
import sys
import struct
import gzip
import time

RPM_MAGIC = b"\xed\xab\xee\xdb"
HEADER_MAGIC = b"\x8e\xad\xe8\x01\x00\x00\x00\x00"

TYPE_NULL = 0
TYPE_CHAR = 1
TYPE_INT8 = 2
TYPE_INT16 = 3
TYPE_INT32 = 4
TYPE_INT64 = 5
TYPE_STRING = 6
TYPE_BIN = 7
TYPE_STRING_ARRAY = 8
TYPE_I18NSTRING = 9

RPMTAG_NAME = 1000
RPMTAG_VERSION = 1001
RPMTAG_RELEASE = 1002
RPMTAG_SUMMARY = 1004
RPMTAG_DESCRIPTION = 1005
RPMTAG_BUILDTIME = 1006
RPMTAG_SIZE = 1009
RPMTAG_DISTRIBUTION = 1010
RPMTAG_VENDOR = 1011
RPMTAG_LICENSE = 1014
RPMTAG_GROUP = 1016
RPMTAG_URL = 1020
RPMTAG_OS = 1021
RPMTAG_ARCH = 1022
RPMTAG_PAYLOADFORMAT = 1124
RPMTAG_PAYLOADCOMPRESSOR = 1125
RPMTAG_PAYLOADFLAGS = 1126
RPMTAG_DIRINDEXES = 1116
RPMTAG_BASENAMES = 1117
RPMTAG_DIRNAMES = 1118
RPMTAG_FILEMODES = 1030
RPMTAG_FILESIZES = 1028

def create_cpio_entry(filename, data, mode=0o100755):
    magic = b"070701"
    ino = 1
    uid = 0
    gid = 0
    nlink = 1
    mtime = int(time.time())
    filesize = len(data)
    devmajor = 0
    devminor = 0
    rdevmajor = 0
    rdevminor = 0
    namesize = len(filename.encode('utf-8')) + 1
    check = 0

    header = f"{magic.decode()}{ino:08x}{mode:08x}{uid:08x}{gid:08x}{nlink:08x}{mtime:08x}{filesize:08x}{devmajor:08x}{devminor:08x}{rdevmajor:08x}{rdevminor:08x}{namesize:08x}{check:08x}".encode('ascii')
    entry = header + filename.encode('utf-8') + b"\x00"
    
    pad_header = (4 - (len(entry) % 4)) % 4
    entry += b"\x00" * pad_header

    entry += data
    pad_data = (4 - (len(data) % 4)) % 4
    entry += b"\x00" * pad_data
    return entry

def build_rpm(output_path, bin_path, desktop_path, icon_path):
    with open(output_path, 'wb') as f:
        # 1. Lead (96 bytes)
        name = "pigeon-1.11.0-1".encode('ascii')[:65].ljust(66, b'\x00')
        lead = RPM_MAGIC + struct.pack(">BBhh66shh16s", 3, 0, 0, 1, name, 1, 5, b"\x00" * 16)
        f.write(lead)

        # 2. CPIO payload
        cpio_data = bytearray()
        if os.path.exists(bin_path):
            with open(bin_path, 'rb') as bf:
                cpio_data += create_cpio_entry("./usr/bin/pigeon", bf.read(), 0o100755)
        if os.path.exists(desktop_path):
            with open(desktop_path, 'rb') as df:
                cpio_data += create_cpio_entry("./usr/share/applications/pigeon.desktop", df.read(), 0o100644)
        if os.path.exists(icon_path):
            with open(icon_path, 'rb') as ic:
                icon_bytes = ic.read()
                cpio_data += create_cpio_entry("./usr/share/pixmaps/pigeon.png", icon_bytes, 0o100644)
                cpio_data += create_cpio_entry("./usr/share/icons/hicolor/256x256/apps/pigeon.png", icon_bytes, 0o100644)
        
        # CPIO trailer
        cpio_data += create_cpio_entry("TRAILER!!!", b"", 0)
        
        compressed_payload = gzip.compress(cpio_data)

        # 3. Signature Header
        sig_data = b"\x00" * 16
        sig_entry = struct.pack(">iiii", 1000, TYPE_BIN, 0, len(sig_data))
        sig_header = HEADER_MAGIC[:4] + struct.pack(">ii", 1, len(sig_data)) + sig_entry + sig_data
        pad_sig = (8 - (len(sig_header) % 8)) % 8
        sig_header += b"\x00" * pad_sig
        f.write(sig_header)

        # 4. Main Header
        f.write(HEADER_MAGIC[:4])
        tags = [
            (RPMTAG_NAME, TYPE_STRING, b"pigeon\x00"),
            (RPMTAG_VERSION, TYPE_STRING, b"1.11.0\x00"),
            (RPMTAG_RELEASE, TYPE_STRING, b"1\x00"),
            (RPMTAG_SUMMARY, TYPE_STRING, b"Next-Generation bootable USB creator\x00"),
            (RPMTAG_DESCRIPTION, TYPE_STRING, b"Fast and reliable bootable USB creator\x00"),
            (RPMTAG_LICENSE, TYPE_STRING, b"GPL-3.0-or-later\x00"),
            (RPMTAG_PAYLOADFORMAT, TYPE_STRING, b"cpio\x00"),
            (RPMTAG_PAYLOADCOMPRESSOR, TYPE_STRING, b"gzip\x00"),
        ]
        
        data_block = b"".join([t[2] for t in tags])
        f.write(struct.pack(">ii", len(tags), len(data_block)))
        
        offset = 0
        for tag, typ, data in tags:
            f.write(struct.pack(">iiii", tag, typ, offset, 1))
            offset += len(data)
            
        f.write(data_block)

        # 5. Compressed payload
        f.write(compressed_payload)

    print(f"Created RPM package: {output_path} ({os.path.getsize(output_path)} bytes)")

if __name__ == "__main__":
    dir_path = os.path.dirname(os.path.abspath(__file__))
    root_dir = os.path.abspath(os.path.join(dir_path, "../.."))
    desktop_file = os.path.join(dir_path, "pigeon.desktop")
    icon_file = os.path.join(root_dir, "assets/pigeon.png")
    out_rpm = os.path.join(dir_path, "pigeon-1.11.0-1.x86_64.rpm")

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

    build_rpm(out_rpm, bin_file, desktop_file, icon_file)
