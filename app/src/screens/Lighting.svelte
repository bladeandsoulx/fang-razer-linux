<script>
  import { status, panel } from '../lib/stores.js';
  import {
    setLighting,
    setPanelBrightness,
    setColorPreset,
    setMonitorBrightness,
    rescanDdc
  } from '../lib/bridge.js';

  const EFFECTS = [
    { id: 'off', label: 'Off' },
    { id: 'static', label: 'Static' },
    { id: 'spectrum', label: 'Spectrum' },
    { id: 'wave', label: 'Wave' }
  ];
  const LOGO_MODES = [
    { id: 'off', label: 'Off' },
    { id: 'static', label: 'Static' },
    { id: 'breathing', label: 'Breathing' }
  ];

  let slider = null; // local slider position before release

  $: brightness = slider ?? $status?.kbd_brightness ?? 60;
  $: fill = brightness;
  $: effect = $status?.kbd_effect?.effect ?? 'static';
  $: color = rgbToHex($status?.kbd_effect);

  function rgbToHex(e) {
    if (!e || e.effect !== 'static') return '#44d62c';
    const h = (v) => (v ?? 0).toString(16).padStart(2, '0');
    return `#${h(e.r)}${h(e.g)}${h(e.b)}`;
  }

  function hexToRgb(hex) {
    return {
      r: parseInt(hex.slice(1, 3), 16),
      g: parseInt(hex.slice(3, 5), 16),
      b: parseInt(hex.slice(5, 7), 16)
    };
  }

  function commitBrightness(e) {
    slider = null;
    setLighting({ brightness: +e.target.value });
  }

  function pickEffect(id) {
    const kbdEffect =
      id === 'static' ? { effect: 'static', ...hexToRgb(color) } : { effect: id };
    setLighting({ kbdEffect });
  }

  function pickColor(e) {
    setLighting({ kbdEffect: { effect: 'static', ...hexToRgb(e.target.value) } });
  }

  function pickLogo(id) {
    setLighting({ logoLed: id });
  }

  // ---- laptop panel brightness + external-monitor brightness/color -------
  let panelSlider = null;
  let monSlider = null;
  let brightError = '';
  let colorError = '';
  let rescanBusy = false;
  let scanMessage = '';

  $: panelBrightness = panelSlider ?? $panel?.brightness ?? 80;
  $: monitorBrightness = monSlider ?? $status?.monitor_brightness ?? 75;

  async function commitPanel(e) {
    panelSlider = null;
    brightError = '';
    try {
      await setPanelBrightness(+e.target.value);
    } catch (err) {
      brightError = String(err);
    }
  }

  async function commitMonitor(e) {
    monSlider = null;
    colorError = '';
    try {
      await setMonitorBrightness(+e.target.value);
    } catch (err) {
      colorError = String(err);
    }
  }

  async function pickMonitorColor(value) {
    colorError = '';
    try {
      await setColorPreset(value);
    } catch (err) {
      colorError = String(err);
    }
  }

  async function scanMonitor() {
    rescanBusy = true;
    colorError = '';
    scanMessage = '';
    try {
      await rescanDdc();
      scanMessage = $status?.color_ddc
        ? 'Monitor detected.'
        : 'No compatible monitor found yet; automatic retry continues.';
    } catch (err) {
      colorError = String(err);
    } finally {
      rescanBusy = false;
    }
  }
</script>

<div class="cols">
  <div class="col">
    <div class="card rise pad">
      <span class="card-label">Keyboard backlight</span>

      <div class="bright">
        <div class="cap mono">{brightness}<em>% brightness</em></div>
        <input
          type="range"
          min="0"
          max="100"
          step="5"
          value={brightness}
          style="--fill:{fill}%"
          on:input={(e) => (slider = +e.target.value)}
          on:change={commitBrightness}
        />
      </div>

      <div class="group">
        <span class="card-label">Effect</span>
        <div class="seg">
          {#each EFFECTS as e}
            <button class:on={effect === e.id} on:click={() => pickEffect(e.id)}>{e.label}</button>
          {/each}
        </div>
      </div>

      {#if effect === 'static'}
        <label class="colorrow">
          <span>Color</span>
          <input type="color" value={color} on:change={pickColor} />
          <span class="mono dim">{color}</span>
        </label>
      {/if}
    </div>

    <div class="card rise pad" style="animation-delay:140ms">
      <span class="card-label">External monitor</span>
      {#if $status?.color_ddc}
        {#if $status.monitor_brightness != null}
          <div class="bright">
            <div class="cap mono">{monitorBrightness}<em>% brightness</em></div>
            <input
              type="range"
              min="0"
              max="100"
              step="5"
              value={monitorBrightness}
              style="--fill:{monitorBrightness}%"
              on:input={(e) => (monSlider = +e.target.value)}
              on:change={commitMonitor}
            />
          </div>
        {/if}

        {#if $status.color_presets.length}
          <div class="group">
            <span class="card-label">Color temperature</span>
            <div class="presets">
              {#each $status.color_presets as p}
                <button
                  class="chip"
                  class:on={$status.color_current === p.value}
                  on:click={() => pickMonitorColor(p.value)}
                >
                  {p.name}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <div class="monitor-note">
          <p class="hint">
            Brightness and color presets on the external monitor, sent over DDC/CI.
            The laptop panel can't be color-managed on Linux — no Synapse-style
            gamut clamp exists.
          </p>
          <button class="chip" disabled={rescanBusy} on:click={scanMonitor}>
            {rescanBusy ? 'Scanning…' : 'Rescan'}
          </button>
        </div>
      {:else if $status}
        <div class="monitor-note">
          <p class="hint">
            No DDC/CI monitor detected yet. Fang retries automatically after boot
            and hot-plug. Check that DDC/CI is enabled in the monitor's on-screen menu.
          </p>
          <button class="chip" disabled={rescanBusy} on:click={scanMonitor}>
            {rescanBusy ? 'Scanning…' : 'Rescan now'}
          </button>
        </div>
      {/if}
      {#if scanMessage}<p class="scan">{scanMessage}</p>{/if}
      {#if colorError}<p class="err">{colorError}</p>{/if}
    </div>
  </div>

  <div class="col">
    {#if $status?.has_logo}
      <div class="card rise pad" style="animation-delay:70ms">
        <span class="card-label">Lid logo</span>
        <div class="group">
          <div class="seg">
            {#each LOGO_MODES as m}
              <button class:on={$status?.logo_led === m.id} on:click={() => pickLogo(m.id)}>
                {m.label}
              </button>
            {/each}
          </div>
        </div>
        <p class="hint">
          The snake on the lid. Static keeps it lit, Breathing pulses it slowly.
        </p>
      </div>
    {/if}

    {#if $panel?.supported}
      <div class="card rise pad" style="animation-delay:210ms">
        <span class="card-label">Laptop panel brightness</span>
        <div class="bright">
          <div class="cap mono">{panelBrightness}<em>% brightness</em></div>
          <input
            type="range"
            min="5"
            max="100"
            step="5"
            value={panelBrightness}
            style="--fill:{panelBrightness}%"
            on:input={(e) => (panelSlider = +e.target.value)}
            on:change={commitPanel}
          />
        </div>
        <p class="hint">The built-in screen's backlight — applies instantly.</p>
        {#if brightError}<p class="err">{brightError}</p>{/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .cols {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
    align-items: start;
  }

  /* Each column packs its own cards top-down, so the internal-panel card
     sits tight under the lid-logo card instead of leaving a grid gap. */
  .col {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .pad {
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .bright {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .cap {
    font-size: 20px;
    color: var(--ink);
  }

  .cap em {
    font-style: normal;
    font-size: 11px;
    letter-spacing: 0.1em;
    color: var(--ink-dim);
    margin-left: 7px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .seg {
    display: flex;
    width: fit-content;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 7px;
    overflow: hidden;
  }

  .seg button {
    padding: 9px 18px;
    font-family: var(--font-data);
    font-size: 11.5px;
    letter-spacing: 0.1em;
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

  .colorrow {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12.5px;
    cursor: pointer;
  }

  .colorrow input[type='color'] {
    width: 44px;
    height: 28px;
    padding: 0;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 6px;
    background: none;
    cursor: pointer;
  }

  .dim {
    color: var(--ink-dim);
  }

  .hint {
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--ink-dim);
  }

  .monitor-note {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .monitor-note .hint {
    flex: 1;
  }

  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .chip {
    padding: 8px 12px;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 7px;
    font-size: 12px;
    color: var(--ink-dim);
    background: #15191c;
    transition: all 0.15s ease;
  }

  .chip:hover {
    color: var(--ink);
  }

  .chip:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .chip.on {
    background: rgba(68, 214, 44, 0.14);
    color: var(--green);
    border-color: var(--green-dim);
    text-shadow: 0 0 8px var(--green-glow);
  }

  .err {
    font-size: 11.5px;
    color: var(--red);
  }

  .scan {
    font-size: 11.5px;
    color: var(--green-soft);
  }
</style>
