<script>
  import ModeCard from '../lib/components/ModeCard.svelte';
  import { status, telemetry } from '../lib/stores.js';
  import { setPerfMode, setAutoPower } from '../lib/bridge.js';

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

  // ---- power-source automation -------------------------------------------
  const AUTO_MODES = [
    { mode: 'silent', title: 'Silent' },
    { mode: 'balanced', title: 'Balanced' },
    { mode: 'creator', title: 'Creator' },
    { mode: 'gaming', title: 'Gaming' }
  ];
  $: autoModes =
    $status?.has_creator_mode === false
      ? AUTO_MODES.filter((m) => m.mode !== 'creator')
      : AUTO_MODES;

  $: auto = $status?.auto_power ?? false;
  $: acProfile = $status?.ac_profile ?? 'balanced';
  $: batteryProfile = $status?.battery_profile ?? 'silent';
  $: acFan = $status?.ac_fan ?? { mode: 'auto' };
  $: batteryFan = $status?.battery_fan ?? { mode: 'auto' };
  $: acFanQuiet = acFan?.mode === 'manual';
  $: batteryFanQuiet = batteryFan?.mode === 'manual';
  $: source = $telemetry?.on_ac == null ? null : $telemetry.on_ac ? 'ac' : 'battery';
  $: quietRpm = $status?.fan_rpm_min ?? 2200;

  const fanFor = (kind) => (kind === 'quiet' ? { mode: 'manual', rpm: quietRpm } : { mode: 'auto' });

  // Merge one field into the current config and re-send the whole thing.
  function commit(patch) {
    setAutoPower(
      patch.enabled ?? auto,
      patch.ac ?? acProfile,
      patch.battery ?? batteryProfile,
      patch.acFan ?? acFan,
      patch.batteryFan ?? batteryFan
    );
  }
  const toggleAuto = (on) => commit({ enabled: on });
  const pickAc = (mode) => commit({ ac: mode });
  const pickBattery = (mode) => commit({ battery: mode });
  const pickAcFan = (kind) => commit({ acFan: fanFor(kind) });
  const pickBatteryFan = (kind) => commit({ batteryFan: fanFor(kind) });
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

<div class="auto card rise">
  <div class="auto-head">
    <div class="lbl">
      <span class="card-label">Power automation</span>
      <p class="sub">Switch profile automatically when you plug in or unplug.</p>
    </div>
    <div class="seg">
      <button class:on={!auto} on:click={() => toggleAuto(false)}>Off</button>
      <button class:on={auto} on:click={() => toggleAuto(true)}>On</button>
    </div>
  </div>

  <div class="rules" class:off={!auto}>
    <div class="rule">
      <span class="src">
        On AC
        {#if source === 'ac'}<em class="cur">now</em>{/if}
      </span>
      <div class="opts">
        <div class="seg">
          {#each autoModes as m}
            <button class:on={acProfile === m.mode} on:click={() => pickAc(m.mode)}>{m.title}</button>
          {/each}
        </div>
        <div class="fanpick">
          <span class="fanlbl">fan</span>
          <div class="seg">
            <button class:on={!acFanQuiet} on:click={() => pickAcFan('auto')}>Auto</button>
            <button class:on={acFanQuiet} on:click={() => pickAcFan('quiet')}>Quiet</button>
          </div>
        </div>
      </div>
    </div>
    <div class="rule">
      <span class="src">
        On battery
        {#if source === 'battery'}<em class="cur">now</em>{/if}
      </span>
      <div class="opts">
        <div class="seg">
          {#each autoModes as m}
            <button class:on={batteryProfile === m.mode} on:click={() => pickBattery(m.mode)}>
              {m.title}
            </button>
          {/each}
        </div>
        <div class="fanpick">
          <span class="fanlbl">fan</span>
          <div class="seg">
            <button class:on={!batteryFanQuiet} on:click={() => pickBatteryFan('auto')}>Auto</button>
            <button class:on={batteryFanQuiet} on:click={() => pickBatteryFan('quiet')}>Quiet</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>

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

  .auto {
    margin-top: 16px;
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .auto-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .lbl .sub {
    margin-top: 6px;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--ink-dim);
    max-width: 44ch;
  }

  .rules {
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: opacity 0.2s ease;
  }

  .rules.off {
    opacity: 0.4;
  }

  .rule {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
  }

  .opts {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    align-items: center;
  }

  .fanpick {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .fanlbl {
    font-family: var(--font-data);
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--ink-faint);
  }

  .src {
    min-width: 96px;
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: var(--font-data);
    font-size: 11.5px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-dim);
  }

  .cur {
    font-style: normal;
    font-size: 9px;
    letter-spacing: 0.1em;
    color: var(--green);
    border: 1px solid var(--green-dim);
    border-radius: 4px;
    padding: 1px 6px;
    text-shadow: 0 0 8px var(--green-glow);
  }
</style>
