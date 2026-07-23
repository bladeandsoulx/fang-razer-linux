#!/usr/bin/env bash
set -euo pipefail

DEB_DIR="${1:?usage: verify.sh DEB_DIRECTORY}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)"
FANGD_UPPER="$(
  sed -n 's/.*"fangd (<< \([^)]*\))".*/\1/p' "$ROOT/app/src-tauri/tauri.conf.json" |
    head -n 1
)"
[[ -n "$VERSION" && -n "$FANGD_UPPER" ]]

mapfile -t packages < <(find "$DEB_DIR" -maxdepth 1 -type f -name '*.deb' -print | sort)
[[ "${#packages[@]}" -eq 2 ]] || {
  printf 'expected exactly two DEBs, found %s\n' "${#packages[@]}" >&2
  exit 1
}

fang="$(readlink -f "$DEB_DIR/Fang_${VERSION}_amd64.deb")"
fangd="$(readlink -f "$DEB_DIR/fangd_${VERSION}-1_amd64.deb")"
[[ -f "$fang" && -f "$fangd" ]] || {
  printf 'expected exact Fang %s DEB filenames\n' "$VERSION" >&2
  exit 1
}

verify_deb() {
  local package=$1
  local expected_name=$2
  local expected_version=$3

  [[ "$(dpkg-deb -f "$package" Package)" == "$expected_name" ]]
  [[ "$(dpkg-deb -f "$package" Version)" == "$expected_version" ]]
  [[ "$(dpkg-deb -f "$package" Architecture)" == amd64 ]]
}

verify_deb "$fang" fang "$VERSION"
verify_deb "$fangd" fangd "${VERSION}-1"
depends="$(dpkg-deb -f "$fang" Depends)"
[[ $depends == *"fangd (>= $VERSION)"* ]]
[[ $depends == *"fangd (<< $FANGD_UPPER)"* ]]

DEBIAN_FRONTEND=noninteractive apt-get install -y "$fangd" "$fang"
[[ "$(dpkg-query -W -f='${Status}' fang)" == 'install ok installed' ]]
[[ "$(dpkg-query -W -f='${Status}' fangd)" == 'install ok installed' ]]
[[ "$(dpkg-query -W -f='${Version}' fang)" == "$VERSION" ]]
[[ "$(dpkg-query -W -f='${Version}' fangd)" == "${VERSION}-1" ]]
getent group fang
test -x /usr/bin/fang
test -x /usr/bin/fangd
test -f /lib/systemd/system/fangd.service
test -f /usr/share/applications/Fang.desktop ||
  test -f /usr/share/applications/fang.desktop

/usr/bin/fangd --version | grep -Fx "fangd $VERSION"
systemd-analyze verify /lib/systemd/system/fangd.service
python3 "$ROOT/packaging/rpm/mock_smoke.py"
if ldd /usr/bin/fang | grep -F 'not found'; then
  printf 'desktop binary has unresolved libraries\n' >&2
  exit 1
fi

set +e
dbus-run-session -- timeout --kill-after=2s 8s xvfb-run -a /usr/bin/fang \
  >"$TMP/fang.out" 2>"$TMP/fang.err"
desktop_status=$?
set -e
case $desktop_status in
  124|137) ;;
  *)
    cat "$TMP/fang.out" "$TMP/fang.err" >&2
    printf 'desktop exited before smoke timeout: %s\n' "$desktop_status" >&2
    exit 1
    ;;
esac

dpkg -V fangd fang
while IFS= read -r owned_path; do
  if [[ -f "$owned_path" || -L "$owned_path" ]]; then
    printf '%s\n' "$owned_path"
  fi
done < <(dpkg-query -L fang fangd) > "$TMP/owned-files"

DEBIAN_FRONTEND=noninteractive apt-get remove -y fang fangd
while IFS= read -r owned_path; do
  [[ ! -e "$owned_path" && ! -L "$owned_path" ]] || {
    printf 'packaged file remains after removal: %s\n' "$owned_path" >&2
    exit 1
  }
done < "$TMP/owned-files"

printf 'DEB verification passed\n'
