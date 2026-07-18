# First run on real hardware (Razer Blade, Ubuntu)

Fang's EC packet layer is byte-verified against razer-laptop-control, but
this checklist is for the first boot on a physical Blade. Work through it in
order; each step has a rollback.

## 0. Baseline

```sh
lsusb -d 1532:            # note the product id (e.g. 1532:02a0)
sensors | head -30        # confirm coretemp is visible
```

Fang recognizes the PIDs in `crates/fang-protocol/src/models.rs`. An unknown
Razer PID is monitor-only by default, even when it exposes a vendor HID usage
page. Add the PID and verified limits to the model table before treating it as
supported. For controlled bring-up only, explicitly approve that exact PID:

```sh
sudo systemctl edit fangd
# Add these two lines, substituting the PID reported by lsusb:
# [Service]
# Environment=FANGD_ALLOW_UNVERIFIED_PID=02c1
sudo systemctl restart fangd
```

The opt-in applies conservative fan limits and the UI shows an “unverified”
warning. Remove the override after adding a verified model profile.

## 1. Install and check the daemon

```sh
sudo ./packaging/install.sh
journalctl -u fangd -b --no-pager | tail -20
```

Expect: `found Razer Blade 18 (2023)` (or your model) and
`listening on /run/fangd.sock`. If you see `no Razer laptop device`, the
daemon is monitor-only — check `lsusb` and that fangd runs as root.

## 2. Socket smoke test (no UI)

```sh
echo '{"id":1,"cmd":"get_status"}' | sudo socat - UNIX-CONNECT:/run/fangd.sock
```

Expect `"ok":true` with your model name, `"device_present":true`.

## 3. Telemetry sanity

```sh
printf '%s\n%s\n' '{"id":1,"cmd":"subscribe"}' '' \
  | sudo socat -t 5 - UNIX-CONNECT:/run/fangd.sock
```

Watch ~5 seconds of `telemetry` events: `cpu_temp_c` should match `sensors`
within a couple of °C; `fan_rpm` should be plausible (0 when fans are
parked at idle is normal).

## 4. Performance mode switch

In the Fang app (or via socket): switch Balanced → Gaming. Under a CPU load
(`stress-ng --cpu 8` for a minute), Gaming should hold noticeably higher
package power / clocks than Silent. Check `journalctl -u fangd` for EC
errors after each switch — there should be none.

## 5. Manual fan

Set fan to Manual at 3000 RPM. Within ~10 s the fans should be audible and
telemetry `fan_rpm` should report the EC's 3000 RPM target. Then switch back to
Auto and confirm the EC curve's target changes. Razer exposes a setpoint, not a
live tachometer, so this validates command/readback rather than measured speed.

Switch to Curve, adjust one point, and apply it. The active target should move
as the hotter CPU/GPU temperature crosses curve points. Manual and Curve modes
always force the model's maximum target at CPU ≥95 °C or GPU ≥87 °C; this guard
cannot be disabled. Do not deliberately overheat the laptop to test it.

## 6. Custom mode boosts

Custom + CPU High / GPU High, run a combined load, watch temps on the
dashboard. On the Blade 18, CPU "Boost" (overclock) is available — expect
temps near the high 90s °C under all-core load; that's Razer's intended
envelope, but back off if you're uncomfortable.

## 6b. GPU mode & refresh rate

- **GPU mode** (GPU & Display screen) needs `prime-select` (comes with
  Ubuntu's NVIDIA driver) or `envycontrol`. Switch Hybrid → Integrated, check
  `prime-select query` reflects it, reboot, confirm `nvidia-smi` fails (dGPU
  off) and battery drain drops. Switch back to Hybrid the same way. The UI
  shows an amber "staged" banner until the reboot.
- **Refresh rate** applies instantly to the active display. Switch 240 → 60 Hz
  and back; cursor motion makes the change obvious. Works on GNOME (Wayland or
  Xorg, via the Mutter DisplayConfig API), KDE (`kscreen-doctor`) and bare X11
  (`xrandr`) — the app picks whichever is present.

## 6c. Display color & brightness

These live on the **Lighting** screen and need `ddcutil` plus an external
monitor with DDC/CI enabled in its on-screen menu (laptop eDP panels don't
speak DDC/CI):

- **External-monitor color temperature** — switch between the presets the
  monitor advertises (Warm / sRGB·D65 / Neutral / Cool / Custom); the screen
  should visibly warm or cool. `ddcutil getvcp 14` reflects the change.
- **External-monitor brightness** — the luminance slider should dim/brighten
  the panel; `ddcutil getvcp 10` tracks it.
- **DDC/CI recovery** — start `fangd` with the monitor disconnected, then
  connect and wake it. The controls should appear automatically within 15–30
  seconds without restarting the daemon. The **Rescan** action should trigger
  the same recovery immediately.
- **Internal-panel brightness** — the laptop-panel slider changes the built-in
  screen's backlight instantly (through logind, no root); clamped to 5–100 %.

The Blade's own wide-gamut panel has no color-managed gamut clamp on Linux, so
there's no internal "sRGB profile" to test — the UI says as much.

## 6d. Battery Health Optimizer

On models with the "bho" feature (Settings shows a Battery card): enable
the optimizer at 80%. With AC plugged and the battery above the cap,
`cat /sys/class/power_supply/BAT*/status` should read `Not charging`
within a couple of minutes; `journalctl -u fangd` shows no EC errors.
Disable to resume normal charging to 100%.

## 7. Persistence

- `sudo systemctl restart fangd` → previous mode/fan settings re-applied
  (journal: applying state line, UI reflects it).
- Put the fan in Manual mode, then `sudo systemctl stop fangd`. The journal
  should report `restored EC automatic fan control`; starting the service
  again safely reapplies the saved Manual preference.
- Suspend, wait 30 s, resume → journal shows
  `wall clock jump detected (resume from suspend); reapplying state`.

## 8. Rollback

```sh
sudo systemctl disable --now fangd     # stop controlling the EC
```

Stopping now restores EC automatic fan control; reboot still returns all EC
settings to firmware defaults. To remove everything:
`sudo apt remove fang fangd` (deb installs) or delete `/usr/bin/fangd`,
`/lib/systemd/system/fangd.service`, `/var/lib/fangd`.

## Reporting results

Open an issue with: model + year, `lsusb -d 1532:` output, `journalctl -u
fangd -b` snippet, and which steps passed/failed. That's enough to add a
verified profile for your machine.
