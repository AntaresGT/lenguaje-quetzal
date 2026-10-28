%global debug_package %{nil}

Name:           quetzal
Version:        %{quetzal_version}
Release:        1%{?dist}
Summary:        CLI del Lenguaje Quetzal
License:        LicenseRef-sin-archivo
URL:            https://lenguajequetzal.com
Vendor:         Antares
Packager:       Antares <info@antaresgt.com>
BuildArch:      x86_64

%description
CLI del Lenguaje Quetzal: el binario quetzal.
Autor: Allam López.

%prep

%build

%install
install -D -m 755 %{quetzal_binario} %{buildroot}%{_bindir}/quetzal
install -D -m 644 %{quetzal_desktop} %{buildroot}%{_datadir}/applications/quetzal.desktop
install -D -m 644 %{quetzal_icono} %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/quetzal.png

%files
%{_bindir}/quetzal
%{_datadir}/applications/quetzal.desktop
%{_datadir}/icons/hicolor/256x256/apps/quetzal.png
