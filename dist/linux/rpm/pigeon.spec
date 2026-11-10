Name:           pigeon
Version:        1.11.0
Release:        1%{?dist}
Summary:        Next-Generation bootable USB drive creator
License:        GPL-3.0-or-later
URL:            https://github.com/pigeon-usb/pigeon
BuildArch:      x86_64

%description
Fast and reliable bootable USB formatting and image flasher,
supporting Windows 10/11 with autounattend bypass, Linux
ISOHybrid, and raw disk images.

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
* Mon Nov 09 2026 Pigeon Team <support@pigeon-usb.org> - 1.0.0-1
- Version 1.0 release of Pigeon

