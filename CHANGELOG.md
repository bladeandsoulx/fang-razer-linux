# Changelog

All notable changes to Fang are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and the project aims to follow
[Semantic Versioning](https://semver.org/).

## [0.9.2] — 2026-07-17 — Support Fang

### Added
- **Dedicated Support screen.** A new sidebar destination explains how
  contributions help fund Fang's development, hardware testing and future
  features while keeping the project independent and open source.
- **Fang creator wallets.** BTC, USDT and Solana addresses include one-click
  copying, responsible-donation guidance, and clear address, network and
  irreversible-transfer warnings.
- **Community-funded roadmap.** A planned Peripherals area covers external
  Razer mice, standalone keyboards, headsets, microphones, mouse docks,
  charging stations, RGB mats and controllers. The screen also highlights
  broader laptop support, possible native Fedora/RHEL packages and a Windows 11
  edition that works fully offline without required accounts, cloud services,
  ads or bundled bloatware. These are future directions, not promises.

## [0.9.1] — 2026-07-17 — Safety, reliability & dependency hardening

### Added
- **Failure-injection safety coverage.** New tests exercise every EC state
  application failure boundary, failed rollback, partial two-fan updates,
  startup recovery, and shutdown restoration without touching real hardware.
- **Protocol and process regression coverage.** Tests now reject malformed,
  stale, truncated and oversized HID replies; prove single-daemon socket
  behavior; and cover helper-process timeouts.
- **Desktop and power fixtures.** Multi-monitor KDE/X11 fixtures verify primary
  output selection, power-supply fixtures cover multiple barrel and USB-C/PD
  adapters, and Rust/JavaScript tests cover transactional autostart failures.
- **Unknown-model bring-up documentation.** README and hardware-testing docs
  explain the exact-PID opt-in, conservative limits, verification workflow and
  how to remove the systemd override after adding a proper model profile.

### Changed
- **Strict EC response validation.** HID replies must now have the exact report
  length, valid framing and checksum, and match the request's transaction,
  command and data size. Malformed or stale replies are retried once and never
  accepted as successful hardware writes.
- **One hardware controller at a time.** A process-wide lock is acquired before
  opening HID or applying state, including for alternate socket paths and the
  automatic-fan restore helper. Live sockets and unrelated filesystem entries
  are never removed during startup or shutdown.
- **Unknown hardware is monitor-only by default.** An unlisted Razer PID is no
  longer selected just because it exposes a vendor usage page. Bring-up now
  requires an exact `FANGD_ALLOW_UNVERIFIED_PID` opt-in; known model PIDs are
  unaffected.
- **Active primary display selection.** KDE and X11 now target the primary
  enabled output instead of hardcoding an `eDP` panel, with active-output
  fallback for layouts without a declared primary. Display helpers have hard
  timeouts and run off the Tauri UI thread.
- **Patched frontend toolchain.** Svelte, Vite, and the Svelte Vite plugin were
  upgraded together to 5.56.5, 6.4.3, and 5.1.1 respectively. Ambiguous
  self-closing HTML markup was updated for a warning-free Svelte 5 build.
- **Complete 0.9.1 package synchronization.** Workspace crates, the Tauri app,
  npm metadata, both lockfiles and the desktop package dependency bounds now
  agree on the 0.9.1 release line.

### Fixed
- **Svelte 5 desktop startup.** The frontend now mounts its root component with
  Svelte 5's `mount()` API instead of the removed class-constructor API, fixing
  an empty black desktop window after the dependency upgrade.
- **Correct Razer HID checksum boundaries.** The checksum now covers the full
  Razer payload after the HID, status and transaction headers, including the
  final argument byte. This matches the Blade EC's real wire responses.
- **Transactional fan recovery.** Failed state changes restore the previous
  complete state; if that also fails, both fan zones are returned to EC Auto.
  Partial two-zone RPM updates are recovered at the model's safe maximum and
  target-only writes stay disabled until a complete state application succeeds.
- **Multiple power adapters.** AC detection now checks all barrel, USB,
  USB-C/PD, wireless, and compatible external supplies. An offline adapter can
  no longer hide a later online one.
- **Transactional autostart settings.** The OS entry is changed before an
  atomic settings-file replacement, failed writes roll the OS state back, and
  the frontend publishes only backend-confirmed settings. Startup also
  reconciles legacy saved state with the actual OS entry; failures are shown in
  Settings and the toggle returns to its last confirmed value.

### Security
- Enabled a restrictive production and development Tauri CSP, allowing only
  bundled assets, Tauri IPC, and Fang's GitHub release-check API request.
- Updated the Tauri `plist` chain to 1.10.0 and `quick-xml` to 0.41.0, resolving
  RUSTSEC-2026-0194 and RUSTSEC-2026-0195. Full and production-only npm audits
  now report zero advisories.

## [0.9.0] — 2026-07-15 — In-app update checker

### Added
- **Check for updates button.** Settings now compares the installed Fang
  version with the latest stable release published on GitHub.
- When a newer version is available, Fang links directly to its release page
  so the matching app and daemon packages can be downloaded together.

## [0.8.2] — 2026-07-13 — Single-instance desktop app

### Added
- **Single-instance protection.** Launching Fang while it is already running
  now restores and focuses the existing window instead of opening a second app
  instance.
- A native information dialog clearly explains that Fang is already open when
  a second launch is attempted.

## [0.8.1] — 2026-07-13 — External-monitor recovery

### Added
- **One-click DDC/CI rescan.** The Lighting screen can immediately search again
  for a connected external monitor without restarting `fangd`.

### Changed
- While no DDC/CI monitor is available, the daemon retries discovery every 15
  seconds. It does no periodic helper work after a monitor has been found.
- The app/daemon socket API is now version 2, adding the explicit DDC rescan
  command. The 0.8.1 desktop package requires `fangd` 0.8.1 or newer.

### Fixed
- DDC/CI discovery no longer caches an early-boot failure until the daemon is
  restarted, so monitors connected after startup recover automatically.
- Failed monitor brightness or color writes now invalidate stale monitor state
  and allow automatic rediscovery to begin.

## [0.8.0] — 2026-07-12 — Fan curves & safety hardening

### Added
- **Custom fan curves.** Define 2–8 temperature/RPM points; the daemon uses the
  hotter CPU/GPU sensor and linearly interpolates the target between points.
- **Mandatory thermal override.** Manual and Curve modes are forced to the
  model's maximum fan target at CPU ≥95 °C or GPU ≥87 °C. The guard cannot be
  disabled and uses hysteresis before releasing.
- **Sensor-loss watchdog.** Manual and Curve start at maximum RPM and only
  relax after a fresh CPU reading. A stale, missing or implausible CPU sensor
  forces maximum fans, with automatic hwmon rediscovery.
- **App/daemon API handshake.** Read-only status remains available across
  versions, but hardware writes are rejected unless both packages use the same
  socket API.

### Changed
- TCP is now strictly a development transport: it requires mock mode and a
  numeric loopback address.
- DDC and GPU helper programs now run outside the thermal-control lock with
  hard timeouts, so a slow helper cannot delay the 1 Hz fan guard.
- Stopping or crashing the service restores EC automatic fan control. systemd
  runs a second restore helper after shutdown as a fallback.

### Fixed
- All Cargo, npm, Tauri and lockfile versions now agree on 0.8.0. The desktop
  package requires a compatible 0.8.x fangd package, and CI checks version
  synchronization.

### Removed
- **Creator mode** has been removed from the protocol, daemon, tray and UI.

## [0.7.0] — 2026-07-07 — Power-source automation

### Added
- **Power-source automation** on the Performance screen: automatically apply a
  performance profile when AC power is connected or removed. Each source
  (AC / battery) maps to a profile — Silent / Balanced / Creator / Gaming — plus
  an independent fan choice: follow the mode's own curve, or pin the fans quiet.
  The daemon reads the AC adapter from `/sys/class/power_supply` (matching the
  `Mains` supply type, so USB-C PD and the battery are ignored) and applies the
  mapped profile on each transition, including at startup. The live source is
  shown with a "now" badge. Off by default.

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
  mode 2 — a standard Razer mode, not an undefined one — so the earlier
  per-model gate wrongly hid it.
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
- **Verified profile for the Razer Blade 18 2024**, unlocking
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
  the Razer EC doesn't define; the EC answered it with a max-fan
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

[0.9.2]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.9.2
[0.9.1]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.9.1
[0.9.0]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.9.0
[0.8.2]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.8.2
[0.8.1]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.8.1
[0.8.0]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.8.0
[0.7.0]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.7.0
[0.6.0]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.6.0
[0.5.0]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.5.0
[0.4.0]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.4.0
[0.3.0]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.3.0
[0.2.0]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.2.0
[0.1.1]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.1.1
[0.1.0]: https://github.com/bladeandsoulx/fang-razer-linux/releases/tag/v0.1.0
