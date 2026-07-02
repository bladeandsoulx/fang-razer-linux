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

echo "==> installing build dependencies"
apt-get update
apt-get install -y --no-install-recommends \
    build-essential pkg-config curl \
    libudev-dev \
    libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
    libayatana-appindicator3-dev \
    nodejs npm

if ! command -v cargo >/dev/null; then
    echo "==> rust toolchain not found; install rustup first:" >&2
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" >&2
    exit 1
fi

echo "==> building fangd (release)"
cd "$REPO_ROOT"
sudo -u "$REAL_USER" cargo build --release -p fangd

echo "==> installing fangd + systemd unit"
install -Dm755 target/release/fangd /usr/bin/fangd
install -Dm644 packaging/fangd.service /lib/systemd/system/fangd.service
getent group fang >/dev/null || groupadd --system fang

systemctl daemon-reload
systemctl enable --now fangd
echo "==> fangd running: $(systemctl is-active fangd)"

echo "==> building the Fang app (Tauri)"
cd "$REPO_ROOT/app"
sudo -u "$REAL_USER" npm install
sudo -u "$REAL_USER" npm run tauri build
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
