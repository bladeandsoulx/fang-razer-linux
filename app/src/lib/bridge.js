// Talks to the Tauri shell when embedded; in a plain browser it runs a
// built-in simulator with the same wire shapes, so the UI can be developed
// and demoed without the daemon.

import { connected, status, telemetry, uiSettings } from './stores.js';

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
    connected.set(await invoke('daemon_connected'));
    if (await invoke('daemon_connected')) status.set(await invoke('get_status'));
    uiSettings.set(await invoke('get_ui_settings'));
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
    daemon_version: '0.1.0-sim'
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
    const rpm =
      this.state.fan.mode === 'manual'
        ? this.state.fan.rpm
        : { silent: 2200, balanced: 2600, creator: 3300, gaming: 3800, custom: 3400 }[mode];
    return { temps, rpm };
  },

  tick() {
    this.t += 1;
    const { temps, rpm } = this.targets();
    const wiggle = Math.sin(this.t * 0.7) * 1.2 + Math.sin(this.t * 0.13) * 2;
    this.cpu += (temps[0] + wiggle - this.cpu) * 0.08;
    this.gpu += (temps[1] + wiggle * 0.8 - this.gpu) * 0.06;
    this.rpm = this.rpm.map(
      (r, i) => r + (rpm + Math.sin(this.t * 1.9 + i) * 25 - r) * 0.15
    );
    telemetry.set({
      cpu_temp_c: this.cpu,
      gpu_temp_c: this.gpu,
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
  }
};

function initSim() {
  connected.set(true);
  uiSettings.set({ autostart: false, close_to_tray: true });
  sim.push();
  sim.tick();
  setInterval(() => sim.tick(), 1000);
}
