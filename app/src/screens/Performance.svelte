<script>
  import ModeCard from '../lib/components/ModeCard.svelte';
  import { status } from '../lib/stores.js';
  import { setPerfMode } from '../lib/bridge.js';

  const MODES = [
    { mode: 'silent', title: 'Silent', icon: 'power', blurb: 'Lowest fan noise, capped power. For late nights and libraries.' },
    { mode: 'balanced', title: 'Balanced', icon: 'dashboard', blurb: 'The everyday default. Sensible power, sensible acoustics.' },
    { mode: 'creator', title: 'Creator', icon: 'gpu', blurb: 'Sustained GPU workloads: rendering, compile farms, ML.' },
    { mode: 'gaming', title: 'Gaming', icon: 'performance', blurb: 'Full tilt. Maximum sustained CPU + GPU power.' },
    { mode: 'custom', title: 'Custom', icon: 'settings', blurb: 'Pick CPU and GPU power levels yourself.' }
  ];

  const CPU_LEVELS = ['low', 'medium', 'high', 'boost'];
  const GPU_LEVELS = ['low', 'medium', 'high'];

  $: cpuLevels = $status?.has_cpu_boost_oc ? CPU_LEVELS : CPU_LEVELS.slice(0, 3);
  // Most ECs don't define Creator (power mode 2); hide it unless the model
  // profile says otherwise — the daemon rejects it anyway.
  $: modes = $status?.has_creator_mode === false ? MODES.filter((m) => m.mode !== 'creator') : MODES;

  function select(e) {
    const mode = e.detail;
    setPerfMode(mode, $status?.cpu_boost, $status?.gpu_boost);
  }

  function setCpu(level) {
    setPerfMode('custom', level, $status?.gpu_boost);
  }

  function setGpu(level) {
    setPerfMode('custom', $status?.cpu_boost, level);
  }
</script>

<div class="cards">
  {#each modes as m, i (m.mode)}
    <ModeCard {...m} active={$status?.perf_mode === m.mode} delay={i * 45} on:select={select} />
  {/each}
</div>

{#if $status?.perf_mode === 'custom'}
  <div class="boosts card rise">
    <div class="group">
      <span class="card-label">CPU power</span>
      <div class="seg">
        {#each cpuLevels as level}
          <button
            class:on={$status.cpu_boost === level}
            class:oc={level === 'boost'}
            on:click={() => setCpu(level)}>{level}</button
          >
        {/each}
      </div>
    </div>
    <div class="group">
      <span class="card-label">GPU power</span>
      <div class="seg">
        {#each GPU_LEVELS as level}
          <button class:on={$status.gpu_boost === level} on:click={() => setGpu(level)}
            >{level}</button
          >
        {/each}
      </div>
    </div>
    {#if $status.cpu_boost === 'boost'}
      <p class="note">Boost overclocks CPU power limits — expect heat and fan noise.</p>
    {/if}
  </div>
{/if}

<style>
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 14px;
  }

  .boosts {
    margin-top: 16px;
    padding: 18px 20px;
    display: flex;
    gap: 40px;
    flex-wrap: wrap;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .seg {
    display: flex;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 7px;
    overflow: hidden;
  }

  .seg button {
    padding: 8px 18px;
    font-family: var(--font-data);
    font-size: 11.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--ink-dim);
    background: #15191c;
    border-right: 1px solid var(--panel-edge);
    transition: all 0.15s ease;
  }

  .seg button:last-child {
    border-right: none;
  }

  .seg button:hover {
    color: var(--ink);
  }

  .seg button.on {
    background: rgba(68, 214, 44, 0.14);
    color: var(--green);
    text-shadow: 0 0 8px var(--green-glow);
  }

  .seg button.oc.on {
    background: rgba(255, 180, 84, 0.14);
    color: var(--amber);
    text-shadow: 0 0 8px rgba(255, 180, 84, 0.4);
  }

  .note {
    width: 100%;
    font-size: 11.5px;
    color: var(--amber);
  }
</style>
