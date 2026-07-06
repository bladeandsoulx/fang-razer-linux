<script>
  import ModeCard from '../lib/components/ModeCard.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { status, display } from '../lib/stores.js';
  import { setGpuMode, setRefreshRate } from '../lib/bridge.js';

  const GPU_MODES = [
    {
      mode: 'integrated',
      title: 'Integrated',
      icon: 'battery',
      blurb: 'iGPU only — the NVIDIA GPU powers down. Maximum battery life.'
    },
    {
      mode: 'hybrid',
      title: 'Hybrid',
      icon: 'layers',
      blurb: 'NVIDIA on demand (PRIME offload). The sensible default.'
    },
    {
      mode: 'dedicated',
      title: 'dGPU',
      icon: 'gpu',
      blurb: 'NVIDIA drives everything. Lowest latency, best for external displays.'
    }
  ];

  let gpuError = '';
  let rateError = '';
  let busyHz = null;

  $: gpuSupported = $status?.gpu_mode != null;

  async function pickGpu(e) {
    gpuError = '';
    try {
      await setGpuMode(e.detail);
    } catch (err) {
      gpuError = String(err);
    }
  }

  async function pickRate(hz) {
    rateError = '';
    busyHz = hz;
    try {
      await setRefreshRate(hz);
    } catch (err) {
      rateError = String(err);
    } finally {
      busyHz = null;
    }
  }

</script>

<div class="section-label card-label">GPU mode</div>

{#if gpuSupported}
  <div class="cards">
    {#each GPU_MODES as m, i}
      <ModeCard {...m} active={$status?.gpu_mode === m.mode} delay={i * 45} on:select={pickGpu} />
    {/each}
  </div>
  {#if $status?.gpu_mode_pending}
    <div class="flag warn rise">
      <Icon name="warn" size={14} />
      GPU switch staged — it takes effect after you log out or reboot.
    </div>
  {/if}
{:else}
  <div class="card rise pad unsupported">
    <p>
      GPU switching isn't available: no <span class="mono">prime-select</span> or
      <span class="mono">envycontrol</span> found on this system.
    </p>
    <p class="dim">
      On Ubuntu it ships with the NVIDIA driver (<span class="mono">nvidia-prime</span>);
      elsewhere: <span class="mono">pip install envycontrol</span>.
    </p>
  </div>
{/if}
{#if gpuError}
  <div class="flag error rise"><Icon name="warn" size={14} /> {gpuError}</div>
{/if}

<div class="section-label card-label second">Refresh rate</div>

<div class="card rise pad">
  {#if $display?.supported}
    <div class="rate-row">
      <Icon name="monitor" size={22} />
      <div class="panel-info">
        <span class="mono panel">{$display.output}</span>
        <span class="dim mono">{$display.resolution}</span>
      </div>
      <div class="seg">
        {#each $display.available_hz as hz}
          <button
            class:on={$display.current_hz === hz}
            disabled={busyHz != null}
            on:click={() => pickRate(hz)}
          >
            {hz}<em>Hz</em>
          </button>
        {/each}
      </div>
    </div>
    <p class="dim note">Applies instantly to the internal panel; no reboot needed.</p>
  {:else if $display}
    <p>Refresh-rate switching isn't available here.</p>
    <p class="dim">{$display.hint}</p>
  {:else}
    <p class="dim">Reading display modes…</p>
  {/if}
  {#if rateError}
    <div class="flag error"><Icon name="warn" size={14} /> {rateError}</div>
  {/if}
</div>

<style>
  .section-label {
    margin: 2px 0 12px;
  }

  .section-label.second {
    margin-top: 26px;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    gap: 14px;
  }

  .pad {
    padding: 18px 20px;
  }

  .flag {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 14px;
    padding: 9px 13px;
    border-radius: 7px;
    font-size: 11.5px;
  }

  .flag.warn {
    background: rgba(255, 180, 84, 0.08);
    color: var(--amber);
    border: 1px solid rgba(255, 180, 84, 0.25);
  }

  .flag.error {
    background: rgba(255, 92, 92, 0.08);
    color: var(--red);
    border: 1px solid rgba(255, 92, 92, 0.25);
  }

  .unsupported p {
    font-size: 12.5px;
    line-height: 1.6;
  }

  .dim {
    color: var(--ink-dim);
  }

  .rate-row {
    display: flex;
    align-items: center;
    gap: 14px;
    color: var(--ink-dim);
  }

  .panel-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }

  .panel {
    font-size: 12.5px;
    color: var(--ink);
  }

  .panel-info .dim {
    font-size: 10.5px;
    letter-spacing: 0.08em;
  }

  .seg {
    display: flex;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 7px;
    overflow: hidden;
  }

  .seg button {
    padding: 9px 20px;
    font-family: var(--font-data);
    font-size: 13px;
    color: var(--ink-dim);
    background: #15191c;
    transition: all 0.15s ease;
  }

  .seg button em {
    font-style: normal;
    font-size: 9.5px;
    margin-left: 2px;
    letter-spacing: 0.08em;
  }

  .seg button + button {
    border-left: 1px solid var(--panel-edge);
  }

  .seg button:hover:not(:disabled) {
    color: var(--ink);
  }

  .seg button.on {
    background: rgba(68, 214, 44, 0.14);
    color: var(--green);
    text-shadow: 0 0 8px var(--green-glow);
  }

  .note {
    font-size: 11.5px;
    margin-top: 12px;
  }
</style>
