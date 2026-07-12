#!/usr/bin/env bash
# Build and install Fang from source on Debian/Ubuntu.
# Usage: sudo ./packaging/install.sh   (run from the repo root)
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
    echo "run as root: sudo $0" >&2
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REAL_USER="${SUDO_USER:-root}"
USER_HOME="$(getent passwd "$REAL_USER" | cut -d: -f6)"

# Run a command as the invoking user with their Rust toolchain on PATH.
# rustup installs cargo under ~/.cargo/bin, which sudo's secure_path drops —
# so a plain `sudo -u user cargo` would fail to find cargo (and the Tauri
# build shells out to cargo as well).
run_user() {
    sudo -u "$REAL_USER" env "PATH=$USER_HOME/.cargo/bin:$PATH" "$@"
}

echo "==> installing build dependencies"
apt-get update
apt-get install -y --no-install-recommends \
    build-essential pkg-config curl \
    libudev-dev \
    libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
    libayatana-appindicator3-dev \
    nodejs npm

if ! run_user sh -c 'command -v cargo >/dev/null'; then
    echo "==> rust toolchain not found for $REAL_USER; install rustup first:" >&2
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
    exit 1
fi

echo "==> building fangd (release)"
cd "$REPO_ROOT"
run_user cargo build --release -p fangd

echo "==> installing fangd + systemd unit"
install -Dm755 target/release/fangd /usr/bin/fangd
install -Dm644 packaging/fangd.service /lib/systemd/system/fangd.service
getent group fang >/dev/null || groupadd --system fang

systemctl daemon-reload
systemctl enable --now fangd
echo "==> fangd running: $(systemctl is-active fangd)"

echo "==> building the Fang app (Tauri)"
cd "$REPO_ROOT/app"
run_user npm install
run_user npm run tauri build
DEB="$(ls -t src-tauri/target/release/bundle/deb/*.deb | head -1)"
echo "==> installing $DEB"
apt-get install -y "$DEB"

if [ "$REAL_USER" != "root" ]; then
    usermod -aG fang "$REAL_USER"
    echo "==> added $REAL_USER to the 'fang' group (log out and back in once)"
fi

echo
echo "Done. Launch 'Fang' from your app menu."
echo "Daemon logs: journalctl -u fangd -f"
