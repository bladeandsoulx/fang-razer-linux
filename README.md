# Fang — Razer Blade Control Center for Linux

[![CI](https://github.com/bladeandsoulx/fang-razer-linux/actions/workflows/ci.yml/badge.svg)](https://github.com/bladeandsoulx/fang-razer-linux/actions/workflows/ci.yml)
[![License: GPL-2.0](https://img.shields.io/badge/license-GPL--2.0-blue.svg)](LICENSE)

**An open-source Razer Synapse alternative for Razer Blade laptops on Linux.**
Control performance modes, safe custom fan curves, battery charging, keyboard
RGB, GPU mode, displays and live thermals — no Windows required.

## A Razer Synapse alternative for Linux laptops

[Razer Synapse 4](https://www.razer.com/gb-en/synapse-4) is Razer's official
device-control platform for customization, performance settings, profiles and
lighting. Fang brings the laptop-focused controls Razer Blade Linux users need
into a native, safety-first desktop app.

| Razer Blade control | Fang on Linux |
|---|---|
| Laptop performance modes and power levels | ✅ |
| Automatic, manual and custom fan curves | ✅ thermal safety override always active |
| Battery Health Optimizer / charge limit | ✅ on supported models |
| Laptop keyboard RGB and lid logo lighting | ✅ on supported models |
| GPU mode, refresh rate and display controls | ✅ |
| Live temperatures, power and fan RPM | ✅ |
| Mouse/keyboard remapping and macros | ❌ not currently supported |

Fang is focused on **Razer Blade laptops**, rather than claiming full Synapse
parity across every Razer peripheral. It currently recognizes 48 Blade models
from 2015–2025.

## Screenshots

![Fang live thermal dashboard for a Razer Blade laptop on Linux](docs/screenshots/dashboard.png)

| Custom fan curves with thermal protection | Performance modes and power automation |
|---|---|
| ![Fang custom Razer Blade fan curve editor on Linux](docs/screenshots/fan-curve.png) | ![Fang Razer Blade performance modes on Linux](docs/screenshots/performance.png) |
| Keyboard, logo and display lighting | GPU mode and refresh-rate controls |
| ![Fang Razer Blade RGB and display controls on Linux](docs/screenshots/lighting.png) | ![Fang Razer Blade GPU switching and refresh-rate controls on Linux](docs/screenshots/gpu-display.png) |

_Screenshots use Fang's built-in hardware simulator; the desktop UI is the
same when connected to `fangd` on a supported Razer Blade._

## What Fang controls

- 🎛 **Performance modes** — Silent / Balanced / Gaming, plus Custom
  with per-CPU/GPU power levels (including CPU overclock boost on supported
  models)
- 🔌 **Power automation** — auto-switch to a chosen profile when you plug in or
  unplug, with an independent fan choice (mode curve or pinned-quiet) per source
- 🌀 **Fan control** — automatic EC curve, manual RPM, or an editable software
  curve, clamped to per-model limits with a non-disableable thermal override
- 🔋 **Battery Health Optimizer** — Synapse-style charge limiter (50–80 %) to
  slow battery wear, on models that support it
- 🎹 **Lighting** — keyboard backlight brightness, hardware effects (Static RGB
  / Spectrum / Wave) and the lid logo LED (Static / Breathing)
- 🖼 **GPU mode** — Integrated / Hybrid / dGPU switching via `prime-select`
  or `envycontrol` (the Linux equivalent of Synapse's GPU mode; applies at
  next logout/reboot)
- ⚡ **Refresh rate** — switch the active display's Hz instantly, on GNOME
  (Wayland or Xorg, via Mutter), KDE (kscreen-doctor) or bare X11 (xrandr)
- 🎨 **Display color & brightness** — color-temperature presets and a brightness
  slider for a DDC/CI external monitor (VCP 0x14 / 0x10), plus internal-panel
  backlight brightness (the wide-gamut laptop panel itself has no color-managed
  gamut clamp on Linux)
- 📊 **Live dashboard** — CPU/GPU temperatures and power draw, fan RPM,
  90-second history
- 🖥 **Tray + autostart** — quick mode switching from the system tray
- 🔁 Settings persist and are re-applied after reboot and suspend/resume

Fang talks to the Blade's embedded controller directly over USB HID — the
same protocol Razer Synapse uses, byte-verified against the
[razer-laptop-control](https://github.com/Razer-Linux/razer-laptop-control-no-dkms)
project. No kernel driver (DKMS) needed.

## Architecture

```
┌────────────────────────┐   JSON lines over socket   ┌──────────────────────────┐
│ fang (Tauri + Svelte)  │ ◄────────────────────────► │ fangd (Rust, root,       │
│  dashboard · modes ·   │   /run/fangd.sock          │        systemd service)  │
│  fan · tray            │                            │  Razer EC HID · hwmon ·  │
└────────────────────────┘                            │  NVML · state persist    │
                                                      └──────────────────────────┘
```

The privileged daemon (`fangd`) owns the hardware; the desktop app is an
unprivileged client (socket access via the `fang` group).

## Install (Ubuntu / Debian)

From source:

```sh
git clone https://github.com/bladeandsoulx/fang-razer-linux && cd fang-razer-linux
sudo ./packaging/install.sh
```

The script installs build dependencies, builds and installs both the `fangd`
daemon and the app as `.deb` packages (so `sudo apt remove fangd fang` cleanly
uninstalls), enables the service, and adds you to the `fang` group (log out and
back in once for group membership to apply).

### Prebuilt packages

Each [release](https://github.com/bladeandsoulx/fang-razer-linux/releases) attaches two
`.deb`s — the `fangd` daemon and the app. After installing both, add yourself
to the `fang` group so the app can reach the daemon socket, then log out and
back in:

```sh
sudo apt install ./fangd_*.deb ./Fang_*.deb
sudo usermod -aG fang "$USER"   # then log out and back in
```

Without the group step the app just shows "daemon offline" — the socket is
`root:fang` and only the daemon runs as root.

## Install (Fedora 43 / 44)

GitHub releases include x86_64 RPMs for the daemon and desktop app. Download
both files from the same release, then install and activate them:

```sh
sudo dnf install ./fangd-*.rpm ./fang-*.rpm
sudo systemctl enable --now fangd
sudo usermod -aG fang "$USER"
```

Log out and back in once after `usermod`; the desktop app needs the new group
membership to access `/run/fangd.sock`. Diagnose the daemon with
`systemctl status fangd` and `journalctl -u fangd`. Remove both packages with:

```sh
sudo dnf remove fang fangd
```

These first RPMs are unsigned direct downloads rather than a configured DNF
repository. Fedora package installation is tested in Fedora 43/44 containers;
physical Razer hardware and SELinux-enforcing behavior depend on community
validation through `HARDWARE_TESTING.md`.

## Supported hardware

Fang recognizes **48 Blade models** (2015–2025) with per-model fan limits and
feature flags (CPU overclock boost, battery charge limiter, lid logo LED)
imported from [Razer-Control](https://github.com/Rintastic247/Razer-Control)'s
device table (GPL-2.0). See
[`crates/fang-protocol/src/models.rs`](crates/fang-protocol/src/models.rs)
for the full list.

| Profile source | Status |
|---|---|
| Blade 18 2023 (`02a0`), Blade 18 2024 (`02b8`) | ✅ complete profile — fan limits + all feature flags |
| 46 further models | ✅ limits from Razer-Control's table (field-tested by that project) |
| Unknown `1532:*` PIDs | 🔒 monitor-only unless that exact PID is explicitly approved |

Adding a model is a one-line entry in
[`crates/fang-protocol/src/models.rs`](crates/fang-protocol/src/models.rs) —
PRs welcome. First time on real hardware? Follow
[HARDWARE_TESTING.md](HARDWARE_TESTING.md).

For bring-up of an unknown Blade, approve only its exact hexadecimal PID with
`FANGD_ALLOW_UNVERIFIED_PID` (for example `02c1`). The daemon still requires a
vendor-defined HID interface and uses conservative limits. With systemd, add
the variable through `sudo systemctl edit fangd`, then restart the service:

```ini
[Service]
Environment=FANGD_ALLOW_UNVERIFIED_PID=02c1
```

## Development (any OS, no Razer hardware needed)

```sh
# terminal 1 — daemon with simulated hardware on TCP
cargo run -p fangd -- --mock --tcp 127.0.0.1:7331

# terminal 2 — the app (connects via FANGD_ADDR, defaults to tcp on non-Linux)
cd app && npm install && npm run tauri dev
```

TCP is accepted only with `--mock` and a numeric loopback address. Real
hardware control is always restricted to the group-protected Unix socket.

Or UI-only in a plain browser (built-in simulator, no daemon at all):

```sh
cd app && npm run dev    # http://localhost:1420
```

Run the tests with `cargo test --workspace`. Release versions are kept in sync
with `node app/scripts/version.mjs check`; use the same tool with
`set MAJOR.MINOR.PATCH` when preparing a release.

## Safety notes

- Manual RPM and custom curves are clamped to the model profile's limits. A
  daemon guard that cannot be disabled forces maximum fans at CPU ≥95 °C or
  GPU ≥87 °C, in addition to the EC's own thermal failsafes. Missing or stale
  CPU telemetry also forces maximum fans.
- Stopping the daemon (`systemctl stop fangd`) restores the EC's automatic fan
  policy. systemd repeats that restore after the process exits as a fallback.
- App and daemon packages negotiate socket API version 2. Read-only status
  remains available on a mismatch, while hardware-changing commands are blocked.
- Custom CPU "Boost" raises power limits — expect heat and fan noise.

## Credits & license

GPL-2.0. Much of Fang's hardware knowledge — the EC packet layouts, the
48-model device table, and the battery-limiter and lighting commands — was
derived from **[Razer-Control](https://github.com/Rintastic247/Razer-Control)**
by **Rintastic247** (GPL-2.0), the maintained continuation of
[razer-laptop-control-no-dkms](https://github.com/Razer-Linux/razer-laptop-control-no-dkms),
with additional reference from [OpenRazer](https://openrazer.github.io/).
If Fang is useful to you, please consider
[supporting Razer-Control's author](https://www.paypal.com/donate/?hosted_button_id=H4SCC24R8KS4A).

Assisted-by: OpenAI GPT-5.6 via Codex and Claude Fable 5 via CLI.

Fang is not affiliated with or endorsed by Razer Inc. "Razer" and "Synapse"
are trademarks of Razer Inc.
