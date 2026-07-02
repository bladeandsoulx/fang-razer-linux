<script>
  // Fan rotor whose angular velocity tracks the real RPM (scaled down so
  // blades stay readable). Pure rAF; no CSS animation restarts.
  import { onMount } from 'svelte';

  export let rpm = 0;
  export let size = 120;

  let angle = 0;
  let el;

  onMount(() => {
    let raf;
    let prev = performance.now();
    const loop = (now) => {
      const dt = (now - prev) / 1000;
      prev = now;
      // visual speed: 1/12th of real revs, capped for readability
      angle = (angle + Math.min(rpm, 6000) / 12 / 60 * 360 * dt) % 360;
      if (el) el.style.transform = `rotate(${angle}deg)`;
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  });
</script>

<svg width={size} height={size} viewBox="0 0 100 100">
  <circle cx="50" cy="50" r="47" class="housing" />
  <circle cx="50" cy="50" r="41" class="ring" />
  <g bind:this={el} style="transform-origin: 50px 50px">
    {#each Array(9) as _, i}
      <path
        d="M50 50 Q56 30 50 14 Q44 26 44 38 Z"
        class="blade"
        transform="rotate({i * 40} 50 50)"
      />
    {/each}
    <circle cx="50" cy="50" r="9" class="hub" />
    <circle cx="50" cy="50" r="3.2" class="hub-dot" />
  </g>
</svg>

<style>
  .housing {
    fill: #101315;
    stroke: var(--panel-edge-hi);
    stroke-width: 1.5;
  }

  .ring {
    fill: none;
    stroke: #1d2327;
    stroke-width: 1;
  }

  .blade {
    fill: #2a3238;
    stroke: #39434a;
    stroke-width: 0.6;
  }

  .hub {
    fill: #171c1f;
    stroke: var(--green-dim);
    stroke-width: 1.2;
  }

  .hub-dot {
    fill: var(--green);
    filter: drop-shadow(0 0 3px var(--green-glow));
  }
</style>
