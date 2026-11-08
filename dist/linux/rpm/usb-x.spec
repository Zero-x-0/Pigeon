Name:           usb-x
Version:        0.1.0
Release:        1%{?dist}
Summary:        High-performance bootable USB drive creator
License:        MIT OR Apache-2.0
URL:            https://github.com/usb-x/usb-x
BuildArch:      x86_64

%description
Fast and reliable bootable USB formatting and image flasher inspired
by Rufus, supporting Windows 10/11 with autounattend bypass, Linux
ISOHybrid, and raw disk images.

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/share/applications
install -m 755 %{_sourcedir}/usb-x %{buildroot}/usr/bin/usb-x
install -m 644 %{_sourcedir}/usb-x.desktop %{buildroot}/usr/share/applications/usb-x.desktop

%files
/usr/bin/usb-x
/usr/share/applications/usb-x.desktop

%changelog
* Sun Nov 08 2026 USB X Team <support@usb-x.org> - 0.1.0-1
- Initial release of USB X for Linux

