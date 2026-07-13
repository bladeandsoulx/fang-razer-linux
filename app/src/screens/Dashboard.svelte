<script>
  import Gauge from '../lib/components/Gauge.svelte';
  import Sparkline from '../lib/components/Sparkline.svelte';
  import FanSpinner from '../lib/components/FanSpinner.svelte';
  import { telemetry, status, cpuHistory, rpmHistory, avgRpm } from '../lib/stores.js';

  const MODE_LABEL = {
    silent: 'Silent',
    balanced: 'Balanced',
    gaming: 'Gaming',
    custom: 'Custom'
  };

  const watts = (w) => (w == null ? null : `${w.toFixed(1)} W`);
</script>

<div class="grid">
  <Gauge value={$telemetry?.cpu_temp_c} label="CPU package" sub={watts($telemetry?.cpu_power_w)} />
  <Gauge value={$telemetry?.gpu_temp_c} label="GPU core" sub={watts($telemetry?.gpu_power_w)} />

  <div class="fan card rise" style="animation-delay:80ms">
    <FanSpinner rpm={$avgRpm ?? 0} size={124} />
    <div class="rpm">
      <span class="big mono">{$avgRpm ?? '--'}</span>
      <span class="card-label">RPM</span>
    </div>
    <div class="fans mono">
      {#if $telemetry?.fan_rpm?.length}
        {#each $telemetry.fan_rpm as r, i}
          <span>FAN{i + 1} <em>{r}</em></span>
        {/each}
      {:else}
        <span>NO FAN DATA</span>
      {/if}
    </div>
  </div>

  <div class="wide">
    <Sparkline data={$cpuHistory} label="CPU temperature — 90 s" unit="°C" min={35} max={95} />
  </div>
  <div class="wide2">
    <Sparkline data={$rpmHistory} label="Fan speed — 90 s" unit="rpm" min={1800} max={5200} />
  </div>

  <div class="modebar card rise" style="animation-delay:140ms">
    <span class="card-label">Active profile</span>
    <span class="mode mono">{MODE_LABEL[$status?.perf_mode] ?? '--'}</span>
    <span class="sub">
      {#if $status?.fan?.mode === 'manual'}
        fan pinned at {$status.fan.rpm} rpm
      {:else}
        fan curve: automatic
      {/if}
    </span>
  </div>
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 14px;
  }

  .fan {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 16px;
  }

  .rpm {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .big {
    font-size: 30px;
    color: var(--green-soft);
  }

  .fans {
    display: flex;
    gap: 14px;
    font-size: 10.5px;
    letter-spacing: 0.08em;
    color: var(--ink-dim);
  }

  .fans em {
    font-style: normal;
    color: var(--ink);
  }

  .wide {
    grid-column: 1 / span 2;
  }

  .wide2 {
    grid-column: 3;
  }

  .modebar {
    grid-column: 1 / -1;
    display: flex;
    align-items: baseline;
    gap: 16px;
    padding: 14px 18px;
  }

  .mode {
    font-size: 20px;
    color: var(--green);
    text-shadow: 0 0 12px var(--green-glow);
  }

  .sub {
    font-size: 12px;
    color: var(--ink-dim);
    margin-left: auto;
  }
</style>
