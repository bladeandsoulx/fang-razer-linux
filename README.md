# Fang

**Synapse-style control center for Razer Blade laptops on Linux.**
Performance modes, fan control and live thermals — no Windows required.

- 🎛 **Performance modes** — Silent / Balanced / Creator / Gaming, plus Custom
  with per-CPU/GPU power levels (including CPU overclock boost on supported
  models)
- 🌀 **Fan control** — automatic EC curve or manual RPM, clamped to per-model
  safe limits
- 🖼 **GPU mode** — Integrated / Hybrid / dGPU switching via `prime-select`
  or `envycontrol` (the Linux equivalent of Synapse's GPU mode; applies at
  next logout/reboot)
- ⚡ **Refresh rate** — switch the internal panel's Hz instantly
  (kscreen-doctor or xrandr; GNOME Wayland users: Settings → Displays)
- 🎨 **Color profiles** — Native / sRGB / Adobe RGB / Rec. 709 via colord's
  standard ICC profiles (applied by GNOME/KDE color management)
- 📊 **Live dashboard** — CPU/GPU temperatures, fan RPM, 90-second history
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
git clone https://github.com/solomonmorse/fang && cd fang
sudo ./packaging/install.sh
```

The script installs build dependencies, builds and enables the `fangd`
service, builds the app `.deb`, and adds you to the `fang` group
(log out and back in once for group membership to apply).

## Supported hardware

| Model | USB PID | Status |
|---|---|---|
| Razer Blade 18 (2023) | `02a0` | ✅ verified profile |
| Razer Blade 18 (2024) | `02b8` | ✅ verified profile |
| Other Razer Blades | any `1532:*` | ⚠️ works with conservative fan limits ("unverified" badge) |

Adding a model is a one-line entry in
[`crates/fang-protocol/src/models.rs`](crates/fang-protocol/src/models.rs) —
PRs welcome. First time on real hardware? Follow
[HARDWARE_TESTING.md](HARDWARE_TESTING.md).

## Development (any OS, no Razer hardware needed)

```sh
# terminal 1 — daemon with simulated hardware on TCP
cargo run -p fangd -- --mock --tcp 127.0.0.1:7331

# terminal 2 — the app (connects via FANGD_ADDR, defaults to tcp on non-Linux)
cd app && npm install && npm run tauri dev
```

Or UI-only in a plain browser (built-in simulator, no daemon at all):

```sh
cd app && npm run dev    # http://localhost:1420
```

Run the tests with `cargo test --workspace`.

## Safety notes

- Manual fan RPM is clamped to the model profile's limits; the EC keeps its
  own thermal failsafes.
- Stopping the daemon (`systemctl stop fangd`) leaves the EC in its last
  state; it returns to defaults on reboot.
- Custom CPU "Boost" raises power limits — expect heat and fan noise.

## Credits & license

GPL-2.0. EC protocol knowledge derived from
[razer-laptop-control-no-dkms](https://github.com/Razer-Linux/razer-laptop-control-no-dkms)
(GPL-2.0) and [OpenRazer](https://openrazer.github.io/).
Fang is not affiliated with or endorsed by Razer Inc. "Razer" and "Synapse"
are trademarks of Razer Inc.
