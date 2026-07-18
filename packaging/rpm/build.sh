#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT="${1:-$ROOT/target/rpm-dist}"
TOPDIR="$(mktemp -d)"
trap 'rm -rf "$TOPDIR"' EXIT

for command in cargo node npm rpmbuild rpm; do
  command -v "$command" >/dev/null || {
    echo "missing build command: $command" >&2
    exit 1
  }
done

cd "$ROOT"
node app/scripts/version.mjs check
cargo build --release -p fangd
(
  cd app
  npm ci
  npm run tauri build -- --no-bundle
)

mkdir -p "$TOPDIR"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
install -pm0755 target/release/fangd "$TOPDIR/SOURCES/fangd"
install -pm0755 app/src-tauri/target/release/fang "$TOPDIR/SOURCES/fang"
install -pm0644 packaging/fangd.service "$TOPDIR/SOURCES/fangd.service"
install -pm0644 packaging/rpm/fang.sysusers "$TOPDIR/SOURCES/fang.sysusers"
install -pm0644 packaging/rpm/fang.desktop "$TOPDIR/SOURCES/fang.desktop"
install -pm0644 LICENSE "$TOPDIR/SOURCES/LICENSE"
install -pm0644 app/src-tauri/icons/32x32.png "$TOPDIR/SOURCES/fang-32.png"
install -pm0644 app/src-tauri/icons/128x128.png "$TOPDIR/SOURCES/fang-128.png"
install -pm0644 app/src-tauri/icons/128x128@2x.png "$TOPDIR/SOURCES/fang-256.png"
install -pm0644 app/src-tauri/icons/icon.png "$TOPDIR/SOURCES/fang-512.png"

rpmbuild --define "_topdir $TOPDIR" -bb packaging/rpm/fangd.spec
rpmbuild --define "_topdir $TOPDIR" -bb packaging/rpm/fang.spec

mkdir -p "$OUTPUT"
find "$OUTPUT" -maxdepth 1 -type f -name '*.rpm' -delete
mapfile -t built < <(find "$TOPDIR/RPMS" -type f -name '*.rpm' -print | sort)
[[ "${#built[@]}" -eq 2 ]] || {
  printf 'expected two RPMs, found %s\n' "${#built[@]}" >&2
  exit 1
}

declare -A seen=()
for package in "${built[@]}"; do
  name="$(rpm -qp --queryformat '%{NAME}' "$package")"
  [[ "$name" == "fang" || "$name" == "fangd" ]] || {
    echo "unexpected RPM package: $name" >&2
    exit 1
  }
  [[ -z "${seen[$name]:-}" ]] || {
    echo "duplicate RPM package: $name" >&2
    exit 1
  }
  seen[$name]=1
  install -pm0644 "$package" "$OUTPUT/"
done

[[ -n "${seen[fang]:-}" && -n "${seen[fangd]:-}" ]]
printf 'RPM artifacts:\n'
find "$OUTPUT" -maxdepth 1 -type f -name '*.rpm' -printf '%f\n' | sort
