<script>
  // Segmented 240° instrument gauge: value colors the arc segments,
  // the number tweens between samples.
  import { tweened } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';

  export let value = null; // may be null (sensor absent)
  export let min = 30;
  export let max = 100;
  export let unit = '°C';
  export let label = '';
  export let sub = null; // secondary reading shown under the label
  export let warn = 78;
  export let danger = 90;

  const display = tweened(min, { duration: 700, easing: cubicOut });
  $: if (value != null) display.set(value);

  const SEGMENTS = 28;
  const START = 150; // degrees; sweep 240° clockwise
  const SWEEP = 240;
  const R = 74;

  function segPath(i) {
    const a0 = ((START + (SWEEP / SEGMENTS) * i + 1.2) * Math.PI) / 180;
    const a1 = ((START + (SWEEP / SEGMENTS) * (i + 1) - 1.2) * Math.PI) / 180;
    const p = (r, a) => `${100 + r * Math.cos(a)} ${100 + r * Math.sin(a)}`;
    return `M ${p(R, a0)} A ${R} ${R} 0 0 1 ${p(R, a1)}`;
  }

  $: frac = value == null ? 0 : Math.max(0, Math.min(1, (value - min) / (max - min)));
  $: lit = Math.round(frac * SEGMENTS);
  $: tone = value == null ? 'off' : value >= danger ? 'danger' : value >= warn ? 'warn' : 'ok';
</script>

<div class="gauge card rise">
  <svg viewBox="0 0 200 172">
    {#each Array(SEGMENTS) as _, i}
      <path
        d={segPath(i)}
        class="seg"
        class:lit={i < lit}
        class:warm={i < lit && i / SEGMENTS > 0.62}
        class:hot={i < lit && i / SEGMENTS > 0.8}
        style="transition-delay:{i * 12}ms"
      />
    {/each}
    <text x="100" y="102" class="value {tone}">
      {value == null ? '--' : $display.toFixed(0)}
    </text>
    <text x="100" y="124" class="unit">{unit}</text>
  </svg>
  <div class="card-label">{label}</div>
  {#if sub}
    <div class="sub mono">{sub}</div>
  {/if}
</div>

<style>
  .gauge {
    padding: 18px 16px 14px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  svg {
    width: 100%;
    max-width: 210px;
  }

  .seg {
    fill: none;
    stroke: #232a2e;
    stroke-width: 9;
    stroke-linecap: butt;
    transition: stroke 0.3s ease, filter 0.3s ease;
  }

  .seg.lit {
    stroke: var(--green);
    filter: drop-shadow(0 0 3px var(--green-glow));
  }

  .seg.lit.warm {
    stroke: var(--amber);
    filter: drop-shadow(0 0 3px rgba(255, 180, 84, 0.4));
  }

  .seg.lit.hot {
    stroke: var(--red);
    filter: drop-shadow(0 0 4px rgba(255, 92, 92, 0.45));
  }

  .value {
    font-family: var(--font-data);
    font-size: 44px;
    font-weight: 350;
    fill: var(--ink);
    text-anchor: middle;
  }

  .value.warn {
    fill: var(--amber);
  }

  .value.danger {
    fill: var(--red);
  }

  .value.off {
    fill: var(--ink-faint);
  }

  .unit {
    font-family: var(--font-data);
    font-size: 11px;
    letter-spacing: 0.2em;
    fill: var(--ink-dim);
    text-anchor: middle;
  }

  .sub {
    font-size: 12px;
    letter-spacing: 0.12em;
    color: var(--green-soft);
  }
</style>
