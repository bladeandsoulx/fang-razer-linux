# Changelog

All notable changes to Fang are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and the project aims to follow
[Semantic Versioning](https://semver.org/).

## [0.6.0] — 2026-07-06 — External-monitor brightness

### Added
- **External-monitor brightness over DDC/CI.** A luminance slider for the
  external display (VCP feature 0x10), scaled to the monitor's own range and
  sent through the daemon (`SetMonitorBrightness`). Shares the "External
  monitor" card with the color presets; hidden when the monitor doesn't report
  the feature.
- **In-app Changelog screen** between Lighting and Settings, so release notes
  are readable without leaving the app.

### Changed
- **Lighting layout** is now two masonry-style columns: the internal
  laptop-panel brightness card sits directly beneath the lid-logo card, instead
  of landing diagonally opposite it in the old auto-flow grid.

### Fixed
- **Creator mode re-enabled on the Blade 18** (2023/2024). Creator is EC power
  mode 2 — a standard Razer mode, hardware-verified on the 2024 Blade 18 — not
  an undefined mode, so the earlier per-model gate wrongly hid it.
- **Honest fan-speed labels.** Razer laptops expose no live tachometer, so the
  fan figure is the EC's target setpoint, not a live measurement. The readout
  and its hint now say so, instead of implying a static number is a live
  reading.

## [0.5.0] — 2026-07-05 — Display color & brightness

### Added
- **External-monitor color control over DDC/CI.** The external monitor's own
  hardware color-temperature presets (Warm / sRGB·D65 6500K / Neutral / Cool /
  Custom) are now switchable from the app. Handled by the daemon (which owns
  i2c access) via `ddcutil` (VCP feature 0x14), with a `SetColorPreset` command
  and the presets a monitor advertises exposed on the status.
- **Internal laptop-panel brightness.** A brightness slider for the built-in
  screen, read from `/sys/class/backlight` and applied through logind's
  `SetBrightness` (no root, works on Wayland; clamped to 5–100 %).

### Changed
- The old "Color profile" feature (colord ICC assignment) was **inert on GNOME
  Wayland** — the standard colorspace profiles carry no VCGT and GNOME never
  applies gamut mapping, so switching did nothing. It has been replaced by the
  DDC/CI path above. True Synapse-style gamut clamp of the internal wide-gamut
  panel has no app-reachable mechanism on GNOME Wayland; this is documented in
  the UI.
- Panel brightness and monitor color moved onto the **Lighting** screen,
  alongside keyboard and logo lighting.

## [0.4.0] — 2026-07-05 — Refresh-rate switching on GNOME

### Added
- **GNOME Mutter refresh-rate backend** (`org.gnome.Mutter.DisplayConfig`),
  tried ahead of `kscreen-doctor` and `xrandr`. It drives the primary monitor —
  external displays included — reconstructing the full logical-monitor layout
  and swapping a single monitor's mode.

### Fixed
- Refresh-rate switching no longer reports "no supported tool" on GNOME Wayland,
  where `xrandr` runs under XWayland and never sees the real outputs.

### Credits
- Attribution added (README + in-app About) for
  [Razer-Control](https://github.com/Rintastic247/Razer-Control) by Rintastic247
  (GPL-2.0), the source of much of Fang's hardware knowledge, with a link to the
  author's donation page.

## [0.3.0] — 2026-07-04 — Lighting & power telemetry

### Added
- **Lighting control** (EC class 0x03): keyboard backlight brightness, hardware
  effects (Off / Static RGB / Spectrum / Wave), and the lid logo LED
  (Off / Static / Breathing), gated on the per-model "logo" feature. New
  Lighting screen; state persists and re-applies on boot and resume.
- **Power-draw telemetry** on the dashboard: CPU package watts via RAPL and GPU
  watts via NVML (behind the runtime-PM gate), shown under the temperature
  gauges.

## [0.2.0] — 2026-07-04 — Hardware support & battery

### Added
- **Battery Health Optimizer** — Synapse-style charge limiter (50–80 %) over the
  EC (class 0x07), gated on the per-model "bho" feature and re-applied after
  reboot/resume. Exposed as a Battery card in Settings.
- **48-model device table** imported from Razer-Control's `laptops.json`
  (GPL-2.0): per-model fan limits and feature flags (CPU overclock boost,
  battery limiter, Creator mode) for Blades from 2015–2025.
- **Verified profile for the Razer Blade 18 2024** (USB `1532:02b8`), unlocking
  the CPU overclock boost level and dropping the "unverified" badge.

### Fixed
- **Creator mode** (EC power mode 2) is now gated on a per-model flag — it's
  defined on only six 2019–2020 models. On everything else it was an undefined
  EC mode (the same failsafe trap as Silent), so the daemon rejects it and the
  UI hides the card.

## [0.1.1] — 2026-07-04 — First-hardware fixes

### Fixed
- **"Daemon offline" on every launch.** The Tauri v2 app shipped with no
  capability file, so the UI was denied all core APIs and its connection-event
  listener was rejected before init finished. Added a `core:default` capability.
- **Daemon hang shown as a stuck "searching…".** The telemetry loop cycled
  `nvmlInit`/`nvmlShutdown` every second, which could livelock the NVIDIA driver
  and wedge the core loop. NVML now holds one session for the daemon's lifetime,
  and each GPU query is gated on the card's sysfs runtime-PM state — so sampling
  never wakes a sleeping dGPU or blocks RTD3.
- **Idle dashboard pinned the GPU** (~30 % on the iGPU). The fan rotor ran an
  unthrottled `requestAnimationFrame` loop and two infinite CSS `box-shadow`
  animations forced constant re-rasterization. Throttled to ~30 fps (paused when
  hidden) and made the glows static — measured render-engine use dropped from
  5.7 % to 0.8 %.
- **Silent mode was the loudest mode.** It was mapped to EC power mode 3, which
  the Razer EC doesn't define; the pid `02b8` EC answered it with a max-fan
  failsafe. Silent now rides on the EC's Custom mode with both boosts pinned to
  Low.

## [0.1.0] — Initial release

- Performance modes (Silent / Balanced / Creator / Gaming / Custom) and CPU/GPU
  boost over the Razer EC.
- Fan control: automatic EC curve or manual RPM, clamped to per-model limits.
- Live dashboard: CPU/GPU temperature gauges, fan RPM, sparkline history.
- GPU mode switching (`prime-select` / `envycontrol`) and system tray + autostart.
- Privileged `fangd` daemon + unprivileged Tauri/Svelte app over a Unix socket;
  settings persist and re-apply after reboot and suspend/resume.

[0.6.0]: https://github.com/solomonmorse/fang/releases/tag/v0.6.0
[0.5.0]: https://github.com/solomonmorse/fang/releases/tag/v0.5.0
[0.4.0]: https://github.com/solomonmorse/fang/releases/tag/v0.4.0
[0.3.0]: https://github.com/solomonmorse/fang/releases/tag/v0.3.0
[0.2.0]: https://github.com/solomonmorse/fang/releases/tag/v0.2.0
[0.1.1]: https://github.com/solomonmorse/fang/releases/tag/v0.1.1
[0.1.0]: https://github.com/solomonmorse/fang/releases/tag/v0.1.0
