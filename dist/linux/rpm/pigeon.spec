Name:           pigeon
Version:        1.11.26
Release:        1%{?dist}
Summary:        Fast, lightweight bootable USB creator & disk imager
License:        GPL-3.0-or-later
URL:            https://github.com/Zero-x-0/Pigeon
BuildArch:      x86_64

%description
High-performance bootable USB formatting and image flasher supporting
Rufus-style single-pass UEFI:NTFS Windows Setup, Linux ISOHybrid with
persistence, and compressed or raw DD disk streaming.

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/pixmaps
install -m 755 %{_sourcedir}/pigeon %{buildroot}/usr/bin/pigeon
install -m 644 %{_sourcedir}/pigeon.desktop %{buildroot}/usr/share/applications/pigeon.desktop
install -m 644 %{_sourcedir}/pigeon.png %{buildroot}/usr/share/pixmaps/pigeon.png

%files
/usr/bin/pigeon
/usr/share/applications/pigeon.desktop
/usr/share/pixmaps/pigeon.png

%changelog
* Thu Nov 26 2026 Pigeon Team <support@pigeon-usb.org> - 1.11.26-1
- Release 1.11.26 with Rufus-style UEFI:NTFS single-pass extraction, auto-resizing window, and live Light/Dark theme switching.
