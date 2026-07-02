# First run on real hardware (Razer Blade, Ubuntu)

Fang's EC packet layer is byte-verified against razer-laptop-control, but
this checklist is for the first boot on a physical Blade. Work through it in
order; each step has a rollback.

## 0. Baseline

```sh
lsusb -d 1532:            # note the product id (e.g. 1532:02a0)
sensors | head -30        # confirm coretemp is visible
```

If your PID is **not** `02a0` (Blade 18 2023), Fang runs in "unverified"
mode: controls work but fan limits use conservative defaults, and the UI
shows a warning badge. To promote your model to verified, add its PID and
fan limits to `crates/fang-protocol/src/models.rs` (find the limits for your
model in razer-laptop-control's `laptops.json`).

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
telemetry `fan_rpm` should read 3000 ± 200. Then back to Auto — the EC curve
resumes (RPM drifts back toward idle). **Don't leave a low manual RPM set
under heavy load**; the EC has its own thermal failsafe but there's no
reason to fight it.

## 6. Custom mode boosts

Custom + CPU High / GPU High, run a combined load, watch temps on the
dashboard. On the Blade 18, CPU "Boost" (overclock) is available — expect
temps near the high 90s °C under all-core load; that's Razer's intended
envelope, but back off if you're uncomfortable.

## 7. Persistence

- `sudo systemctl restart fangd` → previous mode/fan settings re-applied
  (journal: applying state line, UI reflects it).
- Suspend, wait 30 s, resume → journal shows
  `wall clock jump detected (resume from suspend); reapplying state`.

## 8. Rollback

```sh
sudo systemctl disable --now fangd     # stop controlling the EC
```

Reboot returns the EC fully to its default behavior. To remove everything:
`sudo apt remove fang fangd` (deb installs) or delete `/usr/bin/fangd`,
`/lib/systemd/system/fangd.service`, `/var/lib/fangd`.

## Reporting results

Open an issue with: model + year, `lsusb -d 1532:` output, `journalctl -u
fangd -b` snippet, and which steps passed/failed. That's enough to add a
verified profile for your machine.
