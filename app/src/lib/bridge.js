// Talks to the Tauri shell when embedded; in a plain browser it runs a
// built-in simulator with the same wire shapes, so the UI can be developed
// and demoed without the daemon.

import { connected, display, status, telemetry, uiSettings } from './stores.js';

export const inTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

let invoke = null;

export function initBridge() {
  if (inTauri) initTauri();
  else initSim();
}

async function initTauri() {
  const core = await import('@tauri-apps/api/core');
  const { listen } = await import('@tauri-apps/api/event');
  invoke = core.invoke;

  await listen('fang://connected', (e) => connected.set(e.payload));
  await listen('fang://status', (e) => status.set(e.payload));
  await listen('fang://telemetry', (e) => telemetry.set(e.payload));

  try {
    const up = await invoke('daemon_connected');
    connected.set(up);
    if (up) status.set(await invoke('get_status'));
    uiSettings.set(await invoke('get_ui_settings'));
    display.set(await invoke('get_display'));
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
  uiSettings.set(next);
  if (invoke) await invoke('set_ui_settings', { settings: next });
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

/** External-monitor DDC color-temperature preset (value = VCP 0x14 code). */
export async function setColorPreset(value) {
  if (invoke) {
    status.set(await invoke('set_color_preset', { value }));
  } else {
    sim.setColorPreset(value);
  }
}

// ---------------------------------------------------------------- simulator

const sim = {
  state: {
    model: 'Razer Blade 18 (simulated)',
    device_present: true,
    verified: true,
    mock: true,
    perf_mode: 'balanced',
    cpu_boost: 'medium',
    gpu_boost: 'medium',
    fan: { mode: 'auto' },
    fan_rpm_min: 2200,
    fan_rpm_max: 5000,
    has_cpu_boost_oc: true,
    has_creator_mode: true,
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
    gpu_mode: 'hybrid',
    gpu_mode_pending: false,
    daemon_version: '0.1.0-sim'
  },
  displayInfo: {
    supported: true,
    output: 'eDP-1 (simulated)',
    resolution: '2560x1600',
    current_hz: 240,
    available_hz: [60, 120, 240],
    hint: ''
  },
  cpu: 52,
  gpu: 46,
  rpm: [2300, 2280],
  t: 0,

  targets() {
    const mode = this.state.perf_mode;
    const temps = {
      silent: [54, 48],
      balanced: [58, 52],
      creator: [68, 63],
      gaming: [74, 70],
      custom: [70, 66]
    }[mode];
    const watts = {
      silent: [16, 9],
      balanced: [28, 18],
      creator: [45, 60],
      gaming: [58, 92],
      custom: [50, 70]
    }[mode];
    const rpm =
      this.state.fan.mode === 'manual'
        ? this.state.fan.rpm
        : { silent: 2200, balanced: 2600, creator: 3300, gaming: 3800, custom: 3400 }[mode];
    return { temps, watts, rpm };
  },

  tick() {
    this.t += 1;
    const { temps, watts, rpm } = this.targets();
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
      fan_rpm: this.rpm.map((r) => Math.round(r)),
      ts_ms: Date.now()
    });
  },

  push() {
    status.set({ ...this.state, fan: { ...this.state.fan } });
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
        rpm: Math.min(this.state.fan_rpm_max, Math.max(this.state.fan_rpm_min, fan.rpm))
      };
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
  }
};

function initSim() {
  connected.set(true);
  uiSettings.set({ autostart: false, close_to_tray: true });
  display.set(sim.displayInfo);
  sim.push();
  sim.tick();
  setInterval(() => sim.tick(), 1000);
}
