%global debug_package %{nil}

Name: fangd
Version: 0.9.2
Release: 1
Summary: Privileged hardware-control daemon for Fang
License: GPL-2.0-only
URL: https://github.com/bladeandsoulx/fang-razer-linux
Source0: fangd
Source1: fangd.service
Source2: fang.sysusers
Source3: LICENSE
BuildRequires: systemd-rpm-macros
Requires: systemd

%description
Privileged daemon exposing performance, fan, lighting, power, and telemetry
controls for supported Razer Blade laptops over a local Unix socket.

%prep

%build

%install
install -Dpm0755 %{SOURCE0} %{buildroot}%{_bindir}/fangd
install -Dpm0644 %{SOURCE1} %{buildroot}%{_unitdir}/fangd.service
install -Dpm0644 %{SOURCE2} %{buildroot}%{_sysusersdir}/fang.conf
install -Dpm0644 %{SOURCE3} %{buildroot}%{_licensedir}/%{name}/LICENSE

%post
%systemd_post fangd.service

%preun
%systemd_preun fangd.service

%postun
%systemd_postun_with_restart fangd.service

%files
%license %{_licensedir}/%{name}/LICENSE
%{_bindir}/fangd
%{_unitdir}/fangd.service
%{_sysusersdir}/fang.conf
