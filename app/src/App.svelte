<script>
  import Icon from './lib/components/Icon.svelte';
  import Dashboard from './screens/Dashboard.svelte';
  import Performance from './screens/Performance.svelte';
  import FanScreen from './screens/FanScreen.svelte';
  import GpuDisplay from './screens/GpuDisplay.svelte';
  import Lighting from './screens/Lighting.svelte';
  import Changelog from './screens/Changelog.svelte';
  import Settings from './screens/Settings.svelte';
  import Disconnected from './screens/Disconnected.svelte';
  import { connected, status } from './lib/stores.js';
  import { inTauri } from './lib/bridge.js';

  const SCREENS = [
    { id: 'dashboard', title: 'Dashboard', icon: 'dashboard', component: Dashboard },
    { id: 'performance', title: 'Performance', icon: 'performance', component: Performance },
    { id: 'fan', title: 'Fan', icon: 'fan', component: FanScreen },
    { id: 'gpu', title: 'GPU & Display', icon: 'gpu', component: GpuDisplay },
    { id: 'lighting', title: 'Lighting', icon: 'light', component: Lighting },
    { id: 'changelog', title: 'Changelog', icon: 'history', component: Changelog },
    { id: 'settings', title: 'Settings', icon: 'settings', component: Settings }
  ];

  // #dashboard / #performance / #fan / #settings deep-link the screens
  let current =
    SCREENS.find((s) => s.id === location.hash.replace('#', '')) ?? SCREENS[0];

  function nav(s) {
    current = s;
    location.hash = s.id;
  }
</script>

<div class="shell">
  <aside>
    <div class="brand">
      <svg viewBox="0 0 48 48" width="26" height="26">
        <path d="M10 8 L24 40 L27 26 L38 8 L30 8 L25 18 L19 8 Z" class="fang" />
      </svg>
      <span class="word">FANG</span>
    </div>

    <nav>
      {#each SCREENS as s}
        <button class:on={current.id === s.id} on:click={() => nav(s)}>
          <Icon name={s.icon} size={18} />
          <span>{s.title}</span>
        </button>
      {/each}
    </nav>

    <footer>
      <span class="led" class:ok={$connected} />
      <div class="who">
        <span class="model">{$status?.model ?? 'searching…'}</span>
        <span class="daemon mono">
          {$connected ? `fangd ${$status?.daemon_version ?? ''}` : 'daemon offline'}
          {#if $status?.mock}· MOCK{/if}
        </span>
      </div>
    </footer>
  </aside>

  <main>
    <header>
      <h1>{current.title}</h1>
    </header>
    <section>
      <svelte:component this={current.component} />
    </section>
  </main>

  {#if inTauri && !$connected}
    <Disconnected />
  {/if}
</div>

<style>
  .shell {
    position: relative;
    display: grid;
    grid-template-columns: var(--rail-w) 1fr;
    height: 100%;
    z-index: 1;
  }

  aside {
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--panel-edge);
    background: rgba(13, 16, 18, 0.72);
    padding: 20px 12px 14px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 2px 10px 22px;
  }

  .fang {
    fill: var(--green);
    filter: drop-shadow(0 0 8px var(--green-glow));
  }

  .word {
    font-family: var(--font-data);
    font-size: 17px;
    letter-spacing: 0.42em;
    font-weight: 600;
    background: linear-gradient(180deg, #eafbe6, var(--green-soft));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  nav button {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 7px;
    font-size: 13px;
    color: var(--ink-dim);
    position: relative;
    transition: all 0.15s ease;
  }

  nav button:hover {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.03);
  }

  nav button.on {
    color: var(--ink);
    background: rgba(68, 214, 44, 0.09);
  }

  nav button.on::before {
    content: '';
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 2.5px;
    border-radius: 2px;
    background: var(--green);
    box-shadow: 0 0 8px var(--green);
  }

  footer {
    margin-top: auto;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 10px 2px;
    border-top: 1px solid var(--panel-edge);
  }

  .led {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--red);
    box-shadow: 0 0 8px rgba(255, 92, 92, 0.6);
    flex-shrink: 0;
  }

  .led.ok {
    background: var(--green);
    box-shadow: 0 0 8px var(--green);
  }

  .who {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .model {
    font-size: 11px;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .daemon {
    font-size: 9.5px;
    letter-spacing: 0.08em;
    color: var(--ink-faint);
  }

  main {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  header {
    padding: 22px 26px 14px;
  }

  h1 {
    font-size: 13px;
    font-family: var(--font-data);
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.34em;
    color: var(--ink-dim);
  }

  section {
    flex: 1;
    overflow-y: auto;
    padding: 4px 26px 26px;
  }
</style>
