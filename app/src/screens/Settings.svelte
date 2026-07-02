<script>
  import Toggle from '../lib/components/Toggle.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { status, uiSettings, connected } from '../lib/stores.js';
  import { saveUiSettings, inTauri } from '../lib/bridge.js';

  function save() {
    saveUiSettings($uiSettings);
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

  <div class="card rise pad about" style="animation-delay:120ms">
    <span class="card-label">About</span>
    <p>
      <strong>Fang</strong> is an open-source control center for Razer Blade laptops
      on Linux: performance modes, fan control and telemetry, no Windows required.
    </p>
    <p class="dim">
      GPL-2.0 · EC protocol derived from razer-laptop-control · not affiliated with
      Razer Inc.
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
</style>
