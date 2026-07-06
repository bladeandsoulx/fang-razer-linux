<script>
  // Mirrors CHANGELOG.md, condensed for the panel. Newest first.
  const RELEASES = [
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
    Credits: 'cred'
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

  .kind.chg {
    color: #6aa9ff;
  }

  .kind.cred {
    color: var(--green-soft);
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
