<script>
  import { status } from '../lib/stores.js';
  import { setLighting } from '../lib/bridge.js';

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
</script>

<div class="cols">
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
</div>

<style>
  .cols {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
    align-items: start;
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
</style>
