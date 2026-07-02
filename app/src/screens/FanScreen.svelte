<script>
  import FanSpinner from '../lib/components/FanSpinner.svelte';
  import { status, telemetry, avgRpm } from '../lib/stores.js';
  import { setFan } from '../lib/bridge.js';

  let slider = null; // local slider position before release

  $: manual = $status?.fan?.mode === 'manual';
  $: min = $status?.fan_rpm_min ?? 2200;
  $: max = $status?.fan_rpm_max ?? 5000;
  $: target = manual ? $status.fan.rpm : null;
  $: shown = slider ?? target ?? min;
  $: fill = ((shown - min) / (max - min)) * 100;

  function toAuto() {
    slider = null;
    setFan({ mode: 'auto' });
  }

  function toManual() {
    setFan({ mode: 'manual', rpm: target ?? Math.round((min + max) / 2 / 100) * 100 });
  }

  function commit(e) {
    slider = null;
    setFan({ mode: 'manual', rpm: +e.target.value });
  }
</script>

<div class="wrap">
  <div class="visual card rise">
    <FanSpinner rpm={$avgRpm ?? 0} size={190} />
    <div class="live">
      <span class="big mono">{$avgRpm ?? '--'}</span>
      <span class="card-label">measured rpm</span>
    </div>
    <div class="pair mono">
      {#each $telemetry?.fan_rpm ?? [] as r, i}
        <span>FAN{i + 1}<em>{r}</em></span>
      {/each}
    </div>
  </div>

  <div class="controls card rise" style="animation-delay:70ms">
    <span class="card-label">Fan mode</span>
    <div class="seg">
      <button class:on={!manual} on:click={toAuto}>Auto</button>
      <button class:on={manual} on:click={toManual}>Manual</button>
    </div>

    <div class="slider" class:off={!manual}>
      <div class="target mono">
        {#if manual}{shown}<em>rpm target</em>{:else}<em>EC fan curve active</em>{/if}
      </div>
      <input
        type="range"
        {min}
        {max}
        step="100"
        value={shown}
        disabled={!manual}
        style="--fill:{fill}%"
        on:input={(e) => (slider = +e.target.value)}
        on:change={commit}
      />
      <div class="scale mono">
        <span>{min}</span>
        <span>{Math.round((min + max) / 2 / 100) * 100}</span>
        <span>{max}</span>
      </div>
    </div>

    <p class="hint">
      Manual mode pins both fans to a fixed speed. Auto returns control to the
      EC's thermal curve. Limits ({min}–{max} rpm) come from the model profile.
    </p>
  </div>
</div>

<style>
  .wrap {
    display: grid;
    grid-template-columns: minmax(240px, 320px) 1fr;
    gap: 14px;
    align-items: stretch;
  }

  .visual {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    padding: 26px 16px;
  }

  .live {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .big {
    font-size: 36px;
    color: var(--green-soft);
    text-shadow: 0 0 14px var(--green-glow);
  }

  .pair {
    display: flex;
    gap: 18px;
    font-size: 10.5px;
    letter-spacing: 0.08em;
    color: var(--ink-dim);
  }

  .pair em {
    font-style: normal;
    color: var(--ink);
    margin-left: 5px;
  }

  .controls {
    padding: 22px 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .seg {
    display: flex;
    width: fit-content;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 7px;
    overflow: hidden;
  }

  .seg button {
    padding: 9px 26px;
    font-family: var(--font-data);
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ink-dim);
    background: #15191c;
    transition: all 0.15s ease;
  }

  .seg button + button {
    border-left: 1px solid var(--panel-edge);
  }

  .seg button.on {
    background: rgba(68, 214, 44, 0.14);
    color: var(--green);
    text-shadow: 0 0 8px var(--green-glow);
  }

  .slider {
    display: flex;
    flex-direction: column;
    gap: 6px;
    transition: opacity 0.25s ease;
  }

  .slider.off {
    opacity: 0.45;
  }

  .target {
    font-size: 22px;
    color: var(--ink);
    min-height: 30px;
  }

  .target em {
    font-style: normal;
    font-size: 11.5px;
    letter-spacing: 0.1em;
    color: var(--ink-dim);
    margin-left: 8px;
  }

  .scale {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: var(--ink-faint);
  }

  .hint {
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--ink-dim);
    margin-top: auto;
  }
</style>
