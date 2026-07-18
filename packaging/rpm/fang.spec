%global debug_package %{nil}
%global fangd_upper 0.10.0

Name: fang
Version: 0.9.3
Release: 1
Summary: Razer Blade control center for Linux
License: GPL-2.0-only
URL: https://github.com/bladeandsoulx/fang-razer-linux
Source0: fang
Source1: fang.desktop
Source2: LICENSE
Source3: fang-32.png
Source4: fang-128.png
Source5: fang-256.png
Source6: fang-512.png
BuildRequires: desktop-file-utils
Requires: fangd >= %{version}
Requires: fangd < %{fangd_upper}
Requires: libayatana-appindicator-gtk3

%description
Native desktop control center for performance modes, fan curves, lighting,
power, displays, and live telemetry on supported Razer Blade laptops.

%prep

%build

%install
install -Dpm0755 %{SOURCE0} %{buildroot}%{_bindir}/fang
install -Dpm0644 %{SOURCE1} %{buildroot}%{_datadir}/applications/fang.desktop
install -Dpm0644 %{SOURCE2} %{buildroot}%{_licensedir}/%{name}/LICENSE
install -Dpm0644 %{SOURCE3} %{buildroot}%{_datadir}/icons/hicolor/32x32/apps/fang.png
install -Dpm0644 %{SOURCE4} %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/fang.png
install -Dpm0644 %{SOURCE5} %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/fang.png
install -Dpm0644 %{SOURCE6} %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/fang.png

%check
desktop-file-validate %{SOURCE1}

%files
%license %{_licensedir}/%{name}/LICENSE
%{_bindir}/fang
%{_datadir}/applications/fang.desktop
%{_datadir}/icons/hicolor/32x32/apps/fang.png
%{_datadir}/icons/hicolor/128x128/apps/fang.png
%{_datadir}/icons/hicolor/256x256/apps/fang.png
%{_datadir}/icons/hicolor/512x512/apps/fang.png
