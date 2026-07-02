import { writable, derived } from 'svelte/store';

export const connected = writable(false);
export const status = writable(null);
export const telemetry = writable(null);
export const uiSettings = writable({ autostart: false, close_to_tray: true });

const HISTORY = 90; // seconds of sparkline

function ring() {
  const { subscribe, update } = writable([]);
  return {
    subscribe,
    push(v) {
      update((a) => {
        const next = a.length >= HISTORY ? a.slice(a.length - HISTORY + 1) : a.slice();
        next.push(v);
        return next;
      });
    }
  };
}

export const cpuHistory = ring();
export const gpuHistory = ring();
export const rpmHistory = ring();

telemetry.subscribe((t) => {
  if (!t) return;
  if (t.cpu_temp_c != null) cpuHistory.push(t.cpu_temp_c);
  if (t.gpu_temp_c != null) gpuHistory.push(t.gpu_temp_c);
  if (t.fan_rpm?.length) rpmHistory.push(t.fan_rpm.reduce((a, b) => a + b, 0) / t.fan_rpm.length);
});

export const avgRpm = derived(telemetry, (t) =>
  t?.fan_rpm?.length ? Math.round(t.fan_rpm.reduce((a, b) => a + b, 0) / t.fan_rpm.length) : null
);
