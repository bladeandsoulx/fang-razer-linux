<script>
  import Toggle from '../lib/components/Toggle.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { status, uiSettings, connected } from '../lib/stores.js';
  import { saveUiSettings, setBho, openExternal, inTauri } from '../lib/bridge.js';

  let slider = null; // local slider position before release

  $: bhoOn = $status?.bho_enabled ?? false;
  $: threshold = slider ?? $status?.bho_threshold ?? 80;
  $: fill = ((threshold - 50) / 30) * 100;

  function save() {
    saveUiSettings($uiSettings);
  }

  function toggleBho(e) {
    setBho(e.target.checked, threshold);
  }

  function commitThreshold(e) {
    slider = null;
    setBho(true, +e.target.value);
  }
</script>

<div class="cols">
  <div class="card rise pad">
    <span class="card-label">Application</span>
    <Toggle
      bind:checked={$uiSettings.autostart}
      on:change={save}
      label="Launch on login"
      hint="Start Fang minimized to the tray when you sign in"
    />
    <div class="rule" />
    <Toggle
      bind:checked={$uiSettings.close_to_tray}
      on:change={save}
      label="Close to tray"
      hint="Keep running in the tray when the window is closed"
    />
  </div>

  <div class="card rise pad" style="animation-delay:60ms">
    <span class="card-label">Daemon</span>
    <dl class="mono">
      <dt>state</dt>
      <dd class:ok={$connected} class:bad={!$connected}>
        {$connected ? 'connected' : 'offline'}
      </dd>
      <dt>device</dt>
      <dd>{$status?.model ?? '--'}</dd>
      <dt>version</dt>
      <dd>fangd {$status?.daemon_version ?? '--'}</dd>
      <dt>transport</dt>
      <dd>{inTauri ? 'unix socket / tcp' : 'browser simulator'}</dd>
    </dl>
    {#if $status?.mock}
      <div class="flag mock"><Icon name="warn" size={14} /> simulated hardware (mock mode)</div>
    {/if}
    {#if $status && !$status.verified && $status.device_present}
      <div class="flag warn">
        <Icon name="warn" size={14} />
        Unrecognized model — controls use conservative fan limits. See HARDWARE_TESTING.md.
      </div>
    {/if}
  </div>

  {#if $status?.has_bho}
    <div class="card rise pad" style="animation-delay:90ms">
      <span class="card-label">Battery</span>
      <Toggle
        checked={bhoOn}
        on:change={toggleBho}
        label="Battery Health Optimizer"
        hint="Cap charging below 100% to extend the battery's lifespan"
      />
      <div class="limit" class:off={!bhoOn}>
        <div class="cap mono">{threshold}<em>% charge cap</em></div>
        <input
          type="range"
          min="50"
          max="80"
          step="5"
          value={threshold}
          disabled={!bhoOn}
          style="--fill:{fill}%"
          on:input={(e) => (slider = +e.target.value)}
          on:change={commitThreshold}
        />
        <div class="scale mono"><span>50%</span><span>65%</span><span>80%</span></div>
      </div>
      <p class="hint">
        Applied by the EC and re-applied after reboot. Charging pauses at the
        cap; already-charged batteries drain to it slowly while plugged in.
      </p>
    </div>
  {/if}

  <div class="card rise pad about" style="animation-delay:120ms">
    <span class="card-label">About</span>
    <p>
      <strong>Fang</strong> is an open-source control center for Razer Blade laptops
      on Linux: performance modes, fan control and telemetry, no Windows required.
    </p>
    <p class="dim">
      GPL-2.0 · EC protocol, model table, battery limiter and lighting derived from
      <button class="link" on:click={() => openExternal('https://github.com/Rintastic247/Razer-Control')}
        >Razer-Control</button
      >
      by Rintastic247 (GPL-2.0) and razer-laptop-control · not affiliated with Razer Inc.
    </p>
    <p class="dim">
      If Fang is useful to you, consider supporting Razer-Control's author:
      <button
        class="link"
        on:click={() =>
          openExternal('https://www.paypal.com/donate/?hosted_button_id=H4SCC24R8KS4A')}
        >donate via PayPal</button
      >.
    </p>
  </div>
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
  }

  .rule {
    height: 1px;
    background: var(--panel-edge);
  }

  dl {
    display: grid;
    grid-template-columns: 90px 1fr;
    row-gap: 9px;
    padding: 14px 0 4px;
    font-size: 12px;
  }

  dt {
    color: var(--ink-faint);
    text-transform: uppercase;
    letter-spacing: 0.12em;
    font-size: 10px;
    align-self: baseline;
  }

  dd {
    color: var(--ink);
  }

  dd.ok {
    color: var(--green);
  }

  dd.bad {
    color: var(--red);
  }

  .flag {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    padding: 8px 12px;
    border-radius: 7px;
    font-size: 11.5px;
  }

  .flag.mock {
    background: rgba(68, 214, 44, 0.08);
    color: var(--green-soft);
    border: 1px solid rgba(68, 214, 44, 0.25);
  }

  .flag.warn {
    background: rgba(255, 180, 84, 0.08);
    color: var(--amber);
    border: 1px solid rgba(255, 180, 84, 0.25);
  }

  .limit {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 12px;
    transition: opacity 0.25s ease;
  }

  .limit.off {
    opacity: 0.45;
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
    margin-top: 12px;
  }

  .about {
    grid-column: 1 / -1;
    gap: 8px;
  }

  .about p {
    font-size: 12.5px;
    line-height: 1.6;
  }

  .dim {
    color: var(--ink-dim);
  }

  .link {
    padding: 0;
    font: inherit;
    color: var(--green);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .link:hover {
    color: var(--green-soft);
  }
</style>
