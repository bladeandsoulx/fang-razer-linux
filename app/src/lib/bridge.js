// Talks to the Tauri shell when embedded; in a plain browser it runs a
// built-in simulator with the same wire shapes, so the UI can be developed
// and demoed without the daemon.

import {
  connected,
  display,
  panel,
  status,
  telemetry,
  uiSettings,
  versionInfo
} from './stores.js';
import { createUiSettingsCommitter } from './ui-settings.js';

export const inTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

let invoke = null;
const uiSettingsCommitter = createUiSettingsCommitter(
  { autostart: false, close_to_tray: true },
  (settings) => uiSettings.set(settings)
);

export function initBridge() {
  if (inTauri) initTauri();
  else initSim();
}

async function initTauri() {
  const core = await import('@tauri-apps/api/core');
  const { listen } = await import('@tauri-apps/api/event');
  invoke = core.invoke;

  await listen('fang://connected', (e) => connected.set(e.payload));
  await listen('fang://compatibility', (e) => versionInfo.set(e.payload));
  await listen('fang://status', (e) => status.set(e.payload));
  await listen('fang://telemetry', (e) => telemetry.set(e.payload));

  try {
    const up = await invoke('daemon_connected');
    connected.set(up);
    versionInfo.set(await invoke('get_version_info'));
    if (up) status.set(await invoke('get_status'));
    uiSettingsCommitter.confirm(await invoke('get_ui_settings'));
    display.set(await invoke('get_display'));
    panel.set(await invoke('get_panel'));
  } catch (e) {
    console.error('bridge init', e);
  }
}

export async function setPerfMode(perfMode, cpuBoost = null, gpuBoost = null) {
  if (invoke) {
    status.set(await invoke('set_perf_mode', { perfMode, cpuBoost, gpuBoost }));
  } else {
    sim.setPerfMode(perfMode, cpuBoost, gpuBoost);
  }
}

export async function setFan(fan) {
  if (invoke) {
    status.set(await invoke('set_fan', { fan }));
  } else {
    sim.setFan(fan);
  }
}

export async function saveUiSettings(next) {
  if (invoke) {
    return uiSettingsCommitter.save(next, (settings) =>
      invoke('set_ui_settings', { settings })
    );
  }
  return uiSettingsCommitter.confirm(next);
}

export async function setGpuMode(gpuMode) {
  if (invoke) {
    status.set(await invoke('set_gpu_mode', { gpuMode }));
  } else {
    sim.setGpuMode(gpuMode);
  }
}

export async function setBho(enabled, threshold) {
  if (invoke) {
    status.set(await invoke('set_bho', { enabled, threshold }));
  } else {
    sim.setBho(enabled, threshold);
  }
}

/** Partial update: { brightness, kbdEffect, logoLed } — omit to keep. */
export async function setLighting(patch) {
  if (invoke) {
    status.set(await invoke('set_lighting', patch));
  } else {
    sim.setLighting(patch);
  }
}

/** Open a URL in the system browser (credits / donation links). */
export async function openExternal(url) {
  if (invoke) {
    await invoke('open_url', { url });
  } else {
    window.open(url, '_blank', 'noopener');
  }
}

export async function setRefreshRate(hz) {
  if (invoke) {
    display.set(await invoke('set_refresh_rate', { hz }));
  } else {
    sim.setRefreshRate(hz);
  }
}

/** Internal laptop-panel backlight brightness (percent). */
export async function setPanelBrightness(percent) {
  if (invoke) {
    panel.set(await invoke('set_panel_brightness', { percent }));
  } else {
    sim.setPanelBrightness(percent);
  }
}

/** External-monitor DDC color-temperature preset (value = VCP 0x14 code). */
export async function setColorPreset(value) {
  if (invoke) {
    status.set(await invoke('set_color_preset', { value }));
  } else {
    sim.setColorPreset(value);
  }
}

/** External-monitor DDC brightness (VCP 0x10), value = 0..=100 percent. */
export async function setMonitorBrightness(value) {
  if (invoke) {
    status.set(await invoke('set_monitor_brightness', { value }));
  } else {
    sim.setMonitorBrightness(value);
  }
}

/** Immediately retry external-monitor DDC/CI discovery. */
export async function rescanDdc() {
  if (invoke) {
    status.set(await invoke('rescan_ddc'));
  } else {
    sim.rescanDdc();
  }
}

/** AC/battery automation: enable + the profile and fan for each source. */
export async function setAutoPower(enabled, acProfile, batteryProfile, acFan, batteryFan) {
  if (invoke) {
    status.set(
      await invoke('set_auto_power', {
        enabled,
        acProfile,
        batteryProfile,
        acFan,
        batteryFan
      })
    );
  } else {
    sim.setAutoPower(enabled, acProfile, batteryProfile, acFan, batteryFan);
  }
}

// ---------------------------------------------------------------- simulator

const sim = {
  state: {
    model: 'Razer Blade 18 (simulated)',
    device_present: true,
    verified: true,
    mock: true,
    api_version: 2,
    perf_mode: 'balanced',
    cpu_boost: 'medium',
    gpu_boost: 'medium',
    fan: { mode: 'auto' },
    fan_curve: [],
    fan_rpm_min: 2200,
    fan_rpm_max: 5000,
    has_cpu_boost_oc: true,
    has_bho: true,
    bho_enabled: false,
    bho_threshold: 80,
    has_logo: true,
    kbd_brightness: 60,
    kbd_effect: { effect: 'static', r: 0x44, g: 0xd6, b: 0x2c },
    logo_led: 'static',
    color_ddc: true,
    color_presets: [
      { value: 0x03, name: 'Warm (5000K)' },
      { value: 0x04, name: 'sRGB · D65 (6500K)' },
      { value: 0x07, name: 'Cool (9300K)' },
      { value: 0x0b, name: 'Custom (User)' }
    ],
    color_current: 0x04,
    monitor_brightness: 75,
    auto_power: false,
    ac_profile: 'balanced',
    battery_profile: 'silent',
    ac_fan: { mode: 'auto' },
    battery_fan: { mode: 'auto' },
    gpu_mode: 'hybrid',
    gpu_mode_pending: false,
    daemon_version: '0.8.1-sim'
  },
  displayInfo: {
    supported: true,
    output: 'eDP-1 (simulated)',
    resolution: '2560x1600',
    current_hz: 240,
    available_hz: [60, 120, 240],
    hint: ''
  },
  panelInfo: { supported: true, brightness: 80, hint: '' },
  cpu: 52,
  gpu: 46,
  rpm: [2300, 2280],
  onAc: true,
  t: 0,
  thermalOverride: false,

  curveTarget(points, temp) {
    if (temp <= points[0].temp_c) return points[0].rpm;
    for (let i = 1; i < points.length; i += 1) {
      const low = points[i - 1];
      const high = points[i];
      if (temp <= high.temp_c) {
        const fraction = (temp - low.temp_c) / (high.temp_c - low.temp_c);
        return Math.round((low.rpm + (high.rpm - low.rpm) * fraction) / 100) * 100;
      }
    }
    return points[points.length - 1].rpm;
  },

  targets() {
    const mode = this.state.perf_mode;
    const temps = {
      silent: [54, 48],
      balanced: [58, 52],
      gaming: [74, 70],
      custom: [70, 66]
    }[mode];
    const watts = {
      silent: [16, 9],
      balanced: [28, 18],
      gaming: [58, 92],
      custom: [50, 70]
    }[mode];
    const automatic = { silent: 2200, balanced: 2600, gaming: 3800, custom: 3400 }[mode];
    let fanTarget = null;
    if (this.state.fan.mode === 'manual') fanTarget = this.state.fan.rpm;
    if (this.state.fan.mode === 'curve') {
      fanTarget = this.curveTarget(this.state.fan.points, Math.max(this.cpu, this.gpu));
    }
    if (fanTarget == null) {
      this.thermalOverride = false;
    } else if (this.thermalOverride) {
      this.thermalOverride = this.cpu >= 88 || this.gpu >= 82;
    } else {
      this.thermalOverride = this.cpu >= 95 || this.gpu >= 87;
    }
    const rpm = this.thermalOverride ? this.state.fan_rpm_max : (fanTarget ?? automatic);
    return { temps, watts, rpm, fanTarget: fanTarget == null ? null : rpm };
  },

  tick() {
    this.t += 1;
    const { temps, watts, rpm, fanTarget } = this.targets();
    const wiggle = Math.sin(this.t * 0.7) * 1.2 + Math.sin(this.t * 0.13) * 2;
    this.cpu += (temps[0] + wiggle - this.cpu) * 0.08;
    this.gpu += (temps[1] + wiggle * 0.8 - this.gpu) * 0.06;
    this.rpm = this.rpm.map(
      (r, i) => r + (rpm + Math.sin(this.t * 1.9 + i) * 25 - r) * 0.15
    );
    telemetry.set({
      cpu_temp_c: this.cpu,
      gpu_temp_c: this.gpu,
      cpu_power_w: watts[0] + wiggle * 1.5,
      gpu_power_w: watts[1] + wiggle * 2,
      on_ac: this.onAc,
      fan_rpm: this.rpm.map((r) => Math.round(r)),
      fan_target_rpm: fanTarget,
      thermal_override_active: this.thermalOverride,
      thermal_sensor_ok: true,
      thermal_override_reason: this.thermalOverride ? 'temperature' : null,
      ts_ms: Date.now()
    });
  },

  push() {
    const fan = {
      ...this.state.fan,
      ...(this.state.fan.points
        ? { points: this.state.fan.points.map((point) => ({ ...point })) }
        : {})
    };
    status.set({
      ...this.state,
      fan,
      fan_curve: this.state.fan_curve.map((point) => ({ ...point }))
    });
  },

  setPerfMode(perfMode, cpuBoost, gpuBoost) {
    this.state.perf_mode = perfMode;
    if (cpuBoost) this.state.cpu_boost = cpuBoost;
    if (gpuBoost) this.state.gpu_boost = gpuBoost;
    this.push();
  },

  setFan(fan) {
    if (fan.mode === 'manual') {
      fan = {
        mode: 'manual',
        rpm: Math.round(
          Math.min(this.state.fan_rpm_max, Math.max(this.state.fan_rpm_min, fan.rpm)) / 100
        ) * 100
      };
    } else if (fan.mode === 'curve') {
      if (!Array.isArray(fan.points) || fan.points.length < 2 || fan.points.length > 8) {
        throw new Error('fan curve needs 2..=8 points');
      }
      let previous = null;
      fan = {
        mode: 'curve',
        points: fan.points.map((raw) => {
          const point = {
            temp_c: Math.round(raw.temp_c),
            rpm: Math.round(
              Math.min(this.state.fan_rpm_max, Math.max(this.state.fan_rpm_min, raw.rpm)) / 100
            ) * 100
          };
          if (point.temp_c < 30 || point.temp_c > 100) {
            throw new Error('fan-curve temperatures must be between 30 and 100 C');
          }
          if (previous && point.temp_c <= previous.temp_c) {
            throw new Error('fan-curve temperatures must be strictly increasing');
          }
          if (previous && point.rpm < previous.rpm) {
            throw new Error('fan-curve RPM must not decrease as temperature rises');
          }
          previous = point;
          return point;
        })
      };
    }
    if (fan.mode === 'curve') {
      this.state.fan_curve = fan.points.map((point) => ({ ...point }));
    }
    this.state.fan = fan;
    this.push();
  },

  setGpuMode(mode) {
    if (this.state.gpu_mode !== mode) {
      this.state.gpu_mode = mode;
      this.state.gpu_mode_pending = true;
    }
    this.push();
  },

  setBho(enabled, threshold) {
    this.state.bho_enabled = enabled;
    this.state.bho_threshold = Math.min(80, Math.max(50, threshold));
    this.push();
  },

  setLighting({ brightness, kbdEffect, logoLed }) {
    if (brightness != null) this.state.kbd_brightness = Math.min(100, brightness);
    if (kbdEffect) this.state.kbd_effect = kbdEffect;
    if (logoLed) this.state.logo_led = logoLed;
    this.push();
  },

  setRefreshRate(hz) {
    this.displayInfo = { ...this.displayInfo, current_hz: hz };
    display.set(this.displayInfo);
  },

  setColorPreset(value) {
    this.state.color_current = value;
    this.push();
  },

  setMonitorBrightness(value) {
    this.state.monitor_brightness = Math.min(100, Math.max(0, value));
    this.push();
  },

  rescanDdc() {
    this.push();
  },

  setAutoPower(enabled, acProfile, batteryProfile, acFan, batteryFan) {
    this.state.auto_power = enabled;
    this.state.ac_profile = acProfile;
    this.state.battery_profile = batteryProfile;
    this.state.ac_fan = acFan;
    this.state.battery_fan = batteryFan;
    // Mirror the daemon: enabling enforces the current source's profile + fan.
    if (enabled) {
      this.state.perf_mode = this.onAc ? acProfile : batteryProfile;
      this.state.fan = this.onAc ? acFan : batteryFan;
    }
    this.push();
  },

  setPanelBrightness(percent) {
    this.panelInfo = { ...this.panelInfo, brightness: Math.min(100, Math.max(5, percent)) };
    panel.set(this.panelInfo);
  }
};

function initSim() {
  connected.set(true);
  versionInfo.set({
    app_version: '0.8.1-sim',
    app_api_version: 2,
    daemon_api_version: 2,
    compatible: true
  });
  uiSettingsCommitter.confirm({ autostart: false, close_to_tray: true });
  display.set(sim.displayInfo);
  panel.set(sim.panelInfo);
  sim.push();
  sim.tick();
  setInterval(() => sim.tick(), 1000);
}
