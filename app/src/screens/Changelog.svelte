<script>
  // Mirrors CHANGELOG.md, condensed for the panel. Newest first.
  const RELEASES = [
    {
      version: '0.9.5',
      date: '2026-07-23',
      title: 'Neon Fang installer',
      groups: [
        {
          kind: 'Changed',
          items: [
            'The one-command installer now opens with the Neon Fang terminal banner.',
            'Installation guidance is shorter and more beginner-friendly while keeping advanced verification and manual-install options.',
            'Release-documentation checks keep historical release notes aligned with the code that actually shipped.'
          ]
        }
      ]
    },
    {
      version: '0.9.4',
      date: '2026-07-23',
      title: 'Immutable release installer',
      groups: [
        {
          kind: 'Added',
          items: [
            'A release-locked one-command installer selects and validates the matching Fang and fangd package pair before asking for sudo.',
            'USDT donations identify BNB Smart Chain (BEP20) and Ethereum (ERC20) as the accepted networks.'
          ]
        },
        {
          kind: 'Changed',
          items: [
            'Releases publish as an immutable six-asset set containing the installer, checksum manifest, two DEBs and two RPMs.'
          ]
        },
        {
          kind: 'Removed',
          items: [
            'The previous generic crypto-transfer warning and instruction to confirm the USDT network with the creator were removed.'
          ]
        }
      ]
    },
    {
      version: '0.9.3',
      date: '2026-07-18',
      title: 'Fedora RPM support',
      groups: [
        {
          kind: 'Added',
          items: [
            'Native x86_64 RPM packages support Fedora 43 and Fedora 44.',
            'Fedora package gates cover build, installation, launch, dependencies and removal.'
          ]
        },
        {
          kind: 'Changed',
          items: [
            'GitHub releases are created only after both DEBs and both RPMs pass their release gates.'
          ]
        }
      ]
    },
    {
      version: '0.9.2',
      date: '2026-07-17',
      title: 'Support Fang',
      groups: [
        {
          kind: 'Added',
          items: [
            'A dedicated Support screen explains how contributions fund development, testing and future features.',
            'Fang creator BTC, USDT and Solana wallets include one-click copying, responsible-donation guidance and transfer-safety warnings.',
            'A planned Peripherals area covers Razer mice, keyboards, headsets, microphones, docks, charging stations, RGB mats and controllers.',
            'Other future directions include broader laptop support, native Fedora/RHEL packages and an offline, bloatware-free Windows 11 edition.'
          ]
        }
      ]
    },
    {
      version: '0.9.1',
      date: '2026-07-17',
      title: 'Safety, reliability & dependency hardening',
      groups: [
        {
          kind: 'Added',
          items: [
            'Failure-injection tests cover EC rollback, partial two-fan updates, startup recovery and shutdown restoration without touching real hardware.',
            'Protocol and process tests reject malformed, stale, truncated and oversized HID replies and verify single-daemon socket behavior and helper timeouts.',
            'Multi-monitor, power-supply and transactional-autostart fixtures were added alongside unknown-model bring-up documentation.'
          ]
        },
        {
          kind: 'Changed',
          items: [
            'EC replies now require exact framing, length, checksum, transaction, command and data-size matches; invalid replies are retried once.',
            'A process-wide hardware lock prevents multiple controllers, and live sockets or unrelated filesystem entries are never removed.',
            'Unknown Razer product IDs default to monitor-only mode unless explicitly approved with an exact PID opt-in.',
            'KDE and X11 target the primary active display; display helpers now run off the UI thread with hard timeouts.',
            'Svelte, Vite and the Svelte Vite plugin were upgraded together, and all 0.9.1 package metadata was synchronized.'
          ]
        },
        {
          kind: 'Fixed',
          items: [
            'Svelte 5 now mounts the desktop root with mount(), fixing the empty black application window.',
            'Razer HID checksums cover the complete payload, matching real Blade EC responses.',
            'Failed fan changes restore the complete previous state or safely return both fan zones to EC Auto.',
            'AC detection checks every barrel, USB, USB-C/PD, wireless and compatible external power supply.',
            'Autostart changes are transactional across the operating system, settings file and frontend state.'
          ]
        },
        {
          kind: 'Security',
          items: [
            'A restrictive Tauri CSP now permits only bundled assets, Tauri IPC and Fang\'s GitHub release check.',
            'The plist and quick-xml dependency chain was updated to resolve both tracked RustSec advisories; npm audits are clean.'
          ]
        }
      ]
    },
    {
      version: '0.9.0',
      date: '2026-07-15',
      title: 'In-app update checker',
      groups: [
        {
          kind: 'Added',
          items: [
            'A new Check for updates button in Settings compares the installed version with the latest stable GitHub release.',
            'When an update is available, Fang opens the release with matching app and daemon packages.'
          ]
        }
      ]
    },
    {
      version: '0.8.2',
      date: '2026-07-13',
      title: 'Single-instance desktop app',
      groups: [
        {
          kind: 'Added',
          items: [
            'Opening Fang again restores and focuses the existing window instead of starting a duplicate.',
            'A native message explains that Fang is already running.'
          ]
        }
      ]
    },
    {
      version: '0.8.1',
      date: '2026-07-13',
      title: 'External-monitor recovery',
      groups: [
        {
          kind: 'Added',
          items: ['One-click DDC/CI rescan on the Lighting screen.']
        },
        {
          kind: 'Changed',
          items: [
            'While no monitor is available, fangd retries discovery every 15 seconds.',
            'App/daemon API v2 adds the explicit DDC rescan command.'
          ]
        },
        {
          kind: 'Fixed',
          items: [
            'External monitors now recover after early boot or hot-plug without restarting fangd.',
            'Failed brightness or color writes clear stale monitor state and trigger rediscovery.'
          ]
        }
      ]
    },
    {
      version: '0.8.0',
      date: '2026-07-12',
      title: 'Fan curves & safety hardening',
      groups: [
        {
          kind: 'Added',
          items: [
            'Editable custom fan curves using the hotter CPU/GPU temperature.',
            'Mandatory max-fan override at CPU ≥95 °C or GPU ≥87 °C.',
            'Sensor-loss watchdog: software fan control stays at max until CPU telemetry is fresh.',
            'App/daemon API handshake blocks writes when package versions are incompatible.'
          ]
        },
        {
          kind: 'Changed',
          items: [
            'TCP is now mock-only and loopback-only.',
            'DDC/GPU helpers are isolated from thermal control and have hard timeouts.',
            'Stopping or crashing fangd restores the EC automatic fan policy.'
          ]
        },
        {
          kind: 'Fixed',
          items: ['All release manifests are 0.8.0 and the app requires a compatible fangd package.']
        },
        {
          kind: 'Removed',
          items: ['Creator performance mode.']
        }
      ]
    },
    {
      version: '0.7.0',
      date: '2026-07-07',
      title: 'Power-source automation',
      groups: [
        {
          kind: 'Added',
          items: [
            'Auto-switch the performance profile when you plug in or unplug (AC ↔ battery).',
            "A profile per source, plus an independent fan choice — the mode's curve, or pinned quiet.",
            'Reads the AC adapter from sysfs; live source shown with a "now" badge. Off by default.'
          ]
        }
      ]
    },
    {
      version: '0.6.0',
      date: '2026-07-06',
      title: 'External-monitor brightness',
      groups: [
        {
          kind: 'Added',
          items: [
            'External-monitor brightness over DDC/CI (VCP 0x10), on the External monitor card.',
            'In-app Changelog screen between Lighting and Settings.'
          ]
        },
        {
          kind: 'Changed',
          items: [
            'Lighting layout: internal-panel brightness now sits under the lid-logo card (two columns).'
          ]
        },
        {
          kind: 'Fixed',
          items: [
            'Creator mode re-enabled on the Blade 18 — it is a standard EC mode, not an undefined one.',
            'Honest fan labels: the figure is the EC setpoint, not a live tachometer reading.'
          ]
        }
      ]
    },
    {
      version: '0.5.0',
      date: '2026-07-05',
      title: 'Display color & brightness',
      groups: [
        {
          kind: 'Added',
          items: [
            'External-monitor color temperature over DDC/CI (Warm / sRGB 6500K / Neutral / Cool / Custom).',
            "Internal laptop-panel brightness via logind's SetBrightness."
          ]
        },
        {
          kind: 'Changed',
          items: [
            'Replaced the inert colord "Color profile" (did nothing on GNOME Wayland) with the DDC/CI path.',
            'Panel brightness and monitor color moved onto the Lighting screen.'
          ]
        }
      ]
    },
    {
      version: '0.4.0',
      date: '2026-07-05',
      title: 'Refresh-rate switching on GNOME',
      groups: [
        {
          kind: 'Added',
          items: [
            'GNOME Mutter refresh-rate backend — works on Wayland and Xorg, drives the primary monitor.'
          ]
        },
        {
          kind: 'Fixed',
          items: ['No more "no supported tool" on GNOME Wayland (xrandr under XWayland is blind to outputs).']
        },
        {
          kind: 'Credits',
          items: ["Attribution + donation link for Rintastic247's Razer-Control (GPL-2.0)."]
        }
      ]
    },
    {
      version: '0.3.0',
      date: '2026-07-04',
      title: 'Lighting & power telemetry',
      groups: [
        {
          kind: 'Added',
          items: [
            'Keyboard backlight brightness and effects (Static / Spectrum / Wave) + lid logo LED.',
            'CPU and GPU power draw (watts) on the dashboard, under the temperature gauges.'
          ]
        }
      ]
    },
    {
      version: '0.2.0',
      date: '2026-07-04',
      title: 'Hardware support & battery',
      groups: [
        {
          kind: 'Added',
          items: [
            'Battery Health Optimizer — charge limiter (50–80%).',
            "48-model device table imported from Razer-Control's laptops.json.",
            'Verified profile for the Razer Blade 18 2024.'
          ]
        },
        {
          kind: 'Fixed',
          items: ['Creator mode gated per-model — it was an undefined EC mode on most Blades.']
        }
      ]
    },
    {
      version: '0.1.1',
      date: '2026-07-04',
      title: 'First-hardware fixes',
      groups: [
        {
          kind: 'Fixed',
          items: [
            '"Daemon offline" on every launch — added the missing Tauri v2 capability.',
            'Daemon hang shown as a stuck "searching…" — NVML no longer cycled every second.',
            'Idle dashboard pinned the GPU — throttled the fan rotor and made glows static (5.7% → 0.8%).',
            'Silent was the loudest mode — it was sending an undefined EC power mode.'
          ]
        }
      ]
    },
    {
      version: '0.1.0',
      date: '',
      title: 'Initial release',
      groups: [
        {
          kind: 'Added',
          items: [
            'Performance modes and CPU/GPU boost, fan control, live dashboard, GPU mode switching, tray + autostart.'
          ]
        }
      ]
    }
  ];

  const TONE = {
    Added: 'add',
    Fixed: 'fix',
    Changed: 'chg',
    Removed: 'remove',
    Credits: 'cred',
    Security: 'security'
  };
</script>

<div class="log">
  {#each RELEASES as r, i}
    <div class="rel card rise" style="animation-delay:{i * 45}ms">
      <div class="head">
        <span class="ver mono">{r.version}</span>
        <span class="title">{r.title}</span>
        {#if r.date}<span class="date mono">{r.date}</span>{/if}
      </div>
      {#each r.groups as g}
        <div class="group">
          <span class="kind {TONE[g.kind]}">{g.kind}</span>
          <ul>
            {#each g.items as it}
              <li>{it}</li>
            {/each}
          </ul>
        </div>
      {/each}
    </div>
  {/each}
</div>

<style>
  .log {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .rel {
    padding: 18px 20px;
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--panel-edge);
  }

  .ver {
    font-size: 15px;
    font-weight: 600;
    color: var(--green);
    text-shadow: 0 0 10px var(--green-glow);
  }

  .title {
    font-size: 13.5px;
    color: var(--ink);
  }

  .date {
    margin-left: auto;
    font-size: 11px;
    color: var(--ink-faint);
  }

  .group {
    display: grid;
    grid-template-columns: 72px 1fr;
    gap: 12px;
    margin-top: 12px;
  }

  .kind {
    font-family: var(--font-data);
    font-size: 9.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 3px 0;
  }

  .kind.add {
    color: var(--green);
  }

  .kind.fix {
    color: var(--amber);
  }

  .kind.remove {
    color: var(--red);
  }

  .kind.chg {
    color: #6aa9ff;
  }

  .kind.cred {
    color: var(--green-soft);
  }

  .kind.security {
    color: #c792ea;
  }

  ul {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  li {
    position: relative;
    padding-left: 14px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--ink-dim);
  }

  li::before {
    content: '';
    position: absolute;
    left: 0;
    top: 8px;
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--panel-edge-hi);
  }
</style>
