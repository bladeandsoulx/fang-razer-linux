#!/usr/bin/env bash
set -euo pipefail

RPM_DIR="${1:?usage: verify.sh RPM_DIRECTORY}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mapfile -t packages < <(find "$RPM_DIR" -maxdepth 1 -type f -name '*.rpm' -print | sort)
[[ "${#packages[@]}" -eq 2 ]] || {
  printf 'expected two RPMs, found %s\n' "${#packages[@]}" >&2
  exit 1
}

fang=
fangd=
for package in "${packages[@]}"; do
  name="$(rpm -qp --queryformat '%{NAME}' "$package")"
  case "$name" in
    fang) fang="$package" ;;
    fangd) fangd="$package" ;;
    *) echo "unexpected package name: $name" >&2; exit 1 ;;
  esac
done
[[ -n "$fang" && -n "$fangd" ]]

version="$(rpm -qp --queryformat '%{VERSION}' "$fang")"
upper="$(sed -n 's/^%global fangd_upper //p' "$ROOT/packaging/rpm/fang.spec")"
for package in "$fang" "$fangd"; do
  [[ "$(rpm -qp --queryformat '%{VERSION}' "$package")" == "$version" ]]
  [[ "$(rpm -qp --queryformat '%{RELEASE}' "$package")" == "1" ]]
  [[ "$(rpm -qp --queryformat '%{ARCH}' "$package")" == "x86_64" ]]
  [[ "$(rpm -qp --queryformat '%{LICENSE}' "$package")" == "GPL-2.0-only" ]]
done

rpm -qlp "$fangd" | grep -Fx /usr/bin/fangd
rpm -qlp "$fangd" | grep -Fx /usr/lib/systemd/system/fangd.service
rpm -qlp "$fangd" | grep -Fx /usr/lib/sysusers.d/fang.conf
rpm -qlp "$fangd" | grep -Fx /usr/share/licenses/fangd/LICENSE
rpm -qp --queryformat '[%{SYSUSERS}\n]' "$fangd" | grep -F 'g fang - -'
if rpm -qp --scripts "$fangd" | grep -E 'sysusers_create_compat|groupadd'; then
  echo "daemon RPM contains legacy group creation" >&2
  exit 1
fi

rpm -qlp "$fang" | grep -Fx /usr/bin/fang
rpm -qlp "$fang" | grep -Fx /usr/share/applications/fang.desktop
rpm -qlp "$fang" | grep -Fx /usr/share/licenses/fang/LICENSE
for size in 32 128 256 512; do
  rpm -qlp "$fang" | grep -Fx "/usr/share/icons/hicolor/${size}x${size}/apps/fang.png"
done
rpm -qp --requires "$fang" | grep -Fx "fangd >= $version"
rpm -qp --requires "$fang" | grep -Fx "fangd < $upper"
rpm -qp --requires "$fang" | grep -Fx libayatana-appindicator-gtk3

make_dummy_fangd() {
  local dummy_version="$1"
  local top="$TMP/dummy-$dummy_version"
  mkdir -p "$top"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
  {
    printf 'Name: fangd\nVersion: %s\nRelease: 1\n' "$dummy_version"
    printf 'Summary: dependency-bound test double\nLicense: MIT\nBuildArch: noarch\n'
    printf '%%description\nDependency-bound test double.\n'
    printf '%%install\nmkdir -p %%{buildroot}/usr/share/fang-rpm-test\n'
    printf 'echo %s > %%{buildroot}/usr/share/fang-rpm-test/%s\n' "$dummy_version" "$dummy_version"
    printf '%%files\n/usr/share/fang-rpm-test/%s\n' "$dummy_version"
  } > "$top/SPECS/fangd.spec"
  rpmbuild --define "_topdir $top" -bb "$top/SPECS/fangd.spec" >/dev/null
  find "$top/RPMS" -type f -name '*.rpm' -print -quit
}

for incompatible in 0.0.1 "$upper"; do
  dummy="$(make_dummy_fangd "$incompatible")"
  if dnf install -y --setopt=tsflags=test "$fang" "$dummy" >"$TMP/dnf-$incompatible.log" 2>&1; then
    echo "fang accepted incompatible fangd $incompatible" >&2
    cat "$TMP/dnf-$incompatible.log" >&2
    exit 1
  fi
done

dnf install -y "$fangd" "$fang"
[[ "$(rpm -q --queryformat '%{VERSION}' fang)" == "$version" ]]
[[ "$(rpm -q --queryformat '%{VERSION}' fangd)" == "$version" ]]
getent group fang
rpm -V fangd fang
/usr/bin/fangd --version | grep -Fx "fangd $version"
systemd-analyze verify /usr/lib/systemd/system/fangd.service
python3 "$ROOT/packaging/rpm/mock_smoke.py"
desktop-file-validate /usr/share/applications/fang.desktop
if ldd /usr/bin/fang | grep -F 'not found'; then
  echo "desktop binary has unresolved libraries" >&2
  exit 1
fi

set +e
dbus-run-session -- timeout --kill-after=2s 8s xvfb-run -a /usr/bin/fang >"$TMP/fang.out" 2>"$TMP/fang.err"
desktop_status=$?
set -e
case "$desktop_status" in
  124|137) ;;
  *)
    cat "$TMP/fang.out" "$TMP/fang.err" >&2
    echo "desktop exited before smoke timeout: $desktop_status" >&2
    exit 1
    ;;
esac

while IFS= read -r path; do
  if [[ -f "$path" || -L "$path" ]]; then
    printf '%s\n' "$path"
  fi
done < <(rpm -ql fang fangd) > "$TMP/owned-files"

dnf remove -y fang fangd
while IFS= read -r path; do
  [[ ! -e "$path" && ! -L "$path" ]] || {
    echo "packaged file remains after removal: $path" >&2
    exit 1
  }
done < "$TMP/owned-files"

printf 'Fedora RPM verification passed on %s\n' "$(rpm -E '%{fedora}')"
