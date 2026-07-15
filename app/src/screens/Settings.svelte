<script>
  import Toggle from '../lib/components/Toggle.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { status, uiSettings, connected, versionInfo } from '../lib/stores.js';
  import { saveUiSettings, setBho, openExternal, inTauri } from '../lib/bridge.js';
  import { checkForUpdate } from '../lib/updater.js';

  let slider = null; // local slider position before release
  let updateStatus = 'idle';
  let updateInfo = null;

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

  async function checkUpdates() {
    if (!$versionInfo.app_version) return;

    updateStatus = 'checking';
    updateInfo = null;
    try {
      updateInfo = await checkForUpdate($versionInfo.app_version);
      updateStatus = updateInfo.available ? 'available' : 'current';
    } catch (error) {
      console.error('update check', error);
      updateStatus = 'error';
    }
  }

  function updateAction() {
    if (updateStatus === 'available' && updateInfo) {
      openExternal(updateInfo.releaseUrl);
    } else {
      checkUpdates();
    }
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
      <dd>Fang {$versionInfo.app_version || '--'} · fangd {$status?.daemon_version ?? '--'}</dd>
      <dt>API</dt>
      <dd class:ok={$versionInfo.compatible} class:bad={!$versionInfo.compatible}>
        app {$versionInfo.app_api_version} · daemon {$versionInfo.daemon_api_version ?? '--'}
      </dd>
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

  <div class="card rise pad updater" style="animation-delay:105ms">
    <div class="update-head">
      <span class="card-label">Updates</span>
      <span class="installed mono">v{$versionInfo.app_version || '--'} installed</span>
    </div>

    <div class="update-state" aria-live="polite">
      <span
        class="update-icon"
        class:success={updateStatus === 'current'}
        class:available={updateStatus === 'available'}
        class:error={updateStatus === 'error'}
        class:spinning={updateStatus === 'checking'}
      >
        <Icon
          name={updateStatus === 'current'
            ? 'check'
            : updateStatus === 'available'
              ? 'download'
              : updateStatus === 'error'
                ? 'warn'
                : 'refresh'}
          size={20}
        />
      </span>
      <div>
        {#if updateStatus === 'checking'}
          <strong>Checking GitHub…</strong>
          <p>Looking for the latest published Fang release.</p>
        {:else if updateStatus === 'current'}
          <strong>Fang is up to date</strong>
          <p>v{updateInfo.latestVersion} is the latest stable release.</p>
        {:else if updateStatus === 'available'}
          <strong>Fang v{updateInfo.latestVersion} is available</strong>
          <p>Open the release to download the matching app and daemon packages.</p>
        {:else if updateStatus === 'error'}
          <strong>Couldn't check for updates</strong>
          <p>Check your internet connection, then try again.</p>
        {:else}
          <strong>Check for a new version</strong>
          <p>Compare this installation with the latest stable release on GitHub.</p>
        {/if}
      </div>
    </div>

    <button
      type="button"
      class="update-button"
      class:available={updateStatus === 'available'}
      disabled={updateStatus === 'checking' || !$versionInfo.app_version}
      on:click={updateAction}
    >
      <Icon name={updateStatus === 'available' ? 'download' : 'refresh'} size={14} />
      {updateStatus === 'checking'
        ? 'Checking…'
        : updateStatus === 'available'
          ? `Open v${updateInfo.latestVersion} release`
          : updateStatus === 'idle'
            ? 'Check for updates'
            : 'Check again'}
    </button>
  </div>

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

  .updater {
    align-self: stretch;
    gap: 16px;
  }

  .update-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .installed {
    color: var(--ink-faint);
    font-size: 10px;
    white-space: nowrap;
  }

  .update-state {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 56px;
  }

  .update-state strong {
    display: block;
    color: var(--ink);
    font-size: 13px;
    font-weight: 600;
  }

  .update-state p {
    margin-top: 4px;
    color: var(--ink-dim);
    font-size: 11.5px;
    line-height: 1.45;
  }

  .update-icon {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    flex: 0 0 38px;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 50%;
    color: var(--ink-dim);
    background: rgba(255, 255, 255, 0.025);
  }

  .update-icon.success,
  .update-icon.available {
    color: var(--green);
    border-color: rgba(68, 214, 44, 0.35);
    background: rgba(68, 214, 44, 0.08);
    box-shadow: 0 0 14px rgba(68, 214, 44, 0.08);
  }

  .update-icon.error {
    color: var(--red);
    border-color: rgba(255, 92, 92, 0.35);
    background: rgba(255, 92, 92, 0.08);
  }

  .update-icon.spinning :global(svg) {
    animation: spin 0.9s linear infinite;
  }

  .update-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: fit-content;
    margin-top: auto;
    padding: 9px 13px;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 7px;
    color: var(--ink-dim);
    background: #15191c;
    font-family: var(--font-data);
    font-size: 10.5px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    transition: all 0.15s ease;
  }

  .update-button:hover:not(:disabled) {
    color: var(--ink);
    border-color: var(--ink-faint);
    background: #1a1f22;
  }

  .update-button.available {
    color: var(--green);
    border-color: rgba(68, 214, 44, 0.4);
    background: rgba(68, 214, 44, 0.08);
  }

  .update-button:disabled {
    opacity: 0.55;
    cursor: default;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
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
