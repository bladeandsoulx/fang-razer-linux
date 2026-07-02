<script>
  export let data = [];
  export let label = '';
  export let unit = '';
  export let min = null;
  export let max = null;

  const W = 320;
  const H = 64;

  $: lo = min ?? (data.length ? Math.min(...data) - 2 : 0);
  $: hi = max ?? (data.length ? Math.max(...data) + 2 : 1);
  $: pts = data.map((v, i) => {
    const x = data.length > 1 ? (i / (data.length - 1)) * W : W;
    const y = H - ((v - lo) / Math.max(0.001, hi - lo)) * (H - 6) - 3;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  $: line = pts.join(' ');
  $: area = pts.length ? `${line} ${W},${H} 0,${H}` : '';
  $: last = data.length ? data[data.length - 1] : null;
</script>

<div class="spark card rise">
  <div class="head">
    <span class="card-label">{label}</span>
    <span class="now mono">{last == null ? '--' : Math.round(last)}<em>{unit}</em></span>
  </div>
  <svg viewBox="0 0 {W} {H}" preserveAspectRatio="none">
    <defs>
      <linearGradient id="sparkfill" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="rgba(68,214,44,0.28)" />
        <stop offset="1" stop-color="rgba(68,214,44,0)" />
      </linearGradient>
    </defs>
    {#if pts.length > 1}
      <polygon points={area} fill="url(#sparkfill)" />
      <polyline points={line} />
    {/if}
  </svg>
</div>

<style>
  .spark {
    padding: 14px 16px 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .now {
    font-size: 18px;
    color: var(--green-soft);
  }

  .now em {
    font-style: normal;
    font-size: 11px;
    color: var(--ink-dim);
    margin-left: 3px;
  }

  svg {
    width: 100%;
    height: 64px;
  }

  polyline {
    fill: none;
    stroke: var(--green);
    stroke-width: 1.8;
    filter: drop-shadow(0 0 3px var(--green-glow));
  }
</style>
