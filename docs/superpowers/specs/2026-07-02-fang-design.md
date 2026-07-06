# Fang — Synapse-style control for Razer Blades on Linux

**Date:** 2026-07-02
**Status:** Approved
**Working name:** Fang (binaries: `fang` UI, `fangd` daemon). "Synapse" is a Razer
trademark, so the project ships under its own name.

## Goal

A publishable, open-source Linux app for Razer Blade laptops offering fan control,
performance modes, live monitoring, and a system tray — with a clean
Synapse-4-style dark UI. Primary target hardware: Razer Blade 18 on Ubuntu.

## Decisions (from brainstorming)

| Question | Decision |
|---|---|
| Audience | Publishable open-source project |
| Hardware backend | Own daemon from scratch (no razer-laptop-control/OpenRazer dependency) |
| UI stack | Tauri v2 + Svelte frontend |
| V1 features | Fan control, performance modes, system tray + autostart, live monitoring dashboard |
| Look | Synapse-style dark gamer: charcoal panels, Razer green `#44d62c`, glow on active elements |
| Architecture | Privileged daemon + Unix socket API; unprivileged UI client |
| First supported model | Razer Blade 18 |

## Architecture

```
┌────────────────────────┐   JSON lines over socket   ┌──────────────────────────┐
│ fang (Tauri + Svelte)  │ ◄────────────────────────► │ fangd (Rust, root,       │
│  - Dashboard/Perf/Fan/ │   /run/fangd.sock (Linux)  │        systemd service)  │
│    Settings screens    │   127.0.0.1:7331 (dev/TCP) │  - Razer USB HID control │
│  - System tray         │                            │  - hwmon/NVML telemetry  │
└────────────────────────┘                            │  - state persist/reapply │
                                                      └──────────────────────────┘
```

### Workspace layout

```
fang/
├── crates/
│   ├── fang-protocol/   # shared: Razer HID packet builder + socket API types
│   └── fangd/           # daemon
├── app/
│   ├── src-tauri/       # Tauri v2 shell: socket client, tray, commands
│   └── src/             # Svelte frontend
├── packaging/           # systemd unit, deb bits, install.sh
└── docs/
```

### fang-protocol crate

- **Razer HID packet builder.** 90-byte feature report: status, transaction id,
  remaining packets, protocol type, data size, command class, command id,
  80-byte args, CRC (XOR of bytes 2..88), reserved. Commands implemented and
  byte-verified against the razer-laptop-control-no-dkms source:
  - get/set performance mode (Balanced / Gaming / Creator / Custom)
  - get/set CPU boost & GPU boost (Custom mode)
  - get/set fan mode: auto or manual RPM per fan
- **Socket API types** (serde): requests `get_status`, `set_perf_mode`,
  `set_fan`, `subscribe`; responses `{id, ok, data|error}`; pushed events
  `telemetry` (1 Hz) and `state_changed`.
- Pure, no I/O → unit-testable everywhere, including Windows.

### fangd daemon

- Tokio async; runs as root via systemd unit.
- **Hardware backend trait** with two impls:
  - `RazerHw` (Linux only, cfg-gated): hidraw device discovery for vendor
    `0x1532`, model table keyed by USB PID (Blade 18 first-class; unknown
    Blades → monitor-only mode).
  - `MockHw`: fake temps/fans/state, enabled with `--mock` / `FANGD_MOCK=1`;
    compiles and runs on Windows for development.
- **Telemetry**: CPU temp via `/sys/class/hwmon` (coretemp), fan RPM via hwmon,
  NVIDIA GPU temp via NVML (graceful absence). 1 Hz sample loop.
- **Persistence**: `/var/lib/fangd/state.json`; reapplied on startup and after
  suspend/resume (logind `PrepareForSleep`, Linux only).
- **Server**: Unix socket `/run/fangd.sock`, `0660 root:fang` so the `fang`
  group covers UI access without root. `--tcp 127.0.0.1:7331` for dev.
- **Error handling**: HID write failures retried once, then reported in the
  response; never silently dropped.

### fang UI (Tauri v2 + Svelte)

- Rust side: persistent socket client with auto-reconnect/backoff; Tauri
  commands `get_status`, `set_perf_mode`, `set_fan`, `get/set_ui_settings`;
  telemetry and connection state re-emitted as Tauri events.
- Tray: current mode indicator, quick-switch performance modes, show window,
  quit. Close-to-tray optional. Autostart via `tauri-plugin-autostart`.
- Screens:
  1. **Dashboard** — live CPU/GPU temp gauges, fan RPM, 60 s sparklines,
     current mode hero card.
  2. **Performance** — mode cards (Balanced / Gaming / Creator / Custom);
     Custom reveals CPU/GPU boost sliders; active card glows green.
  3. **Fan** — auto ↔ manual toggle, RPM slider with safe bounds from model
     table.
  4. **Settings** — autostart, close-to-tray, daemon connection info.
- Daemon unreachable → onboarding screen with install/enable one-liners.
- Theme: charcoal `#0f1113` background, `#1a1d20` cards, Razer green `#44d62c`
  accent, subtle green glow on active elements, smooth animated numbers.

## Testing

- `fang-protocol`: unit tests asserting exact packet bytes (CRC, command
  class/id, args) against known-good values from razer-laptop-control.
- `fangd`: integration test over TCP with `MockHw` (request/response +
  subscribe stream).
- Frontend: developed and visually verified in browser with a mock bridge;
  full app demoable on Windows via mock daemon over TCP.
- **Real hardware:** validated against a physical Blade 18 using the
  `HARDWARE_TESTING.md` checklist (daemon logs, mode switching, fan RPM
  readback, temps sanity).

## Packaging (v1)

- `.deb` via cargo-deb for `fangd` (bundles systemd unit, creates `fang`
  group, enables service) + `.deb` for the Tauri app (Tauri bundler).
- `install.sh` from-source path for non-deb users.
- Explicitly v2: AppImage, AUR, Flatpak.

## Out of scope for v1

Keyboard RGB, battery charge limit, fan curves (v1 is auto/manual RPM),
per-game profiles, non-Ubuntu packages.
