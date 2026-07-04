<script>
  import { createEventDispatcher } from 'svelte';
  import Icon from './Icon.svelte';

  export let mode;
  export let title;
  export let blurb;
  export let icon;
  export let active = false;
  export let delay = 0;

  const dispatch = createEventDispatcher();
</script>

<button
  class="mode card rise"
  class:active
  style="animation-delay:{delay}ms"
  on:click={() => dispatch('select', mode)}
>
  <span class="icon"><Icon name={icon} size={26} /></span>
  <span class="title">{title}</span>
  <span class="blurb">{blurb}</span>
  <span class="led" />
</button>

<style>
  .mode {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 18px 16px 16px;
    text-align: left;
    transition: transform 0.15s ease, border-color 0.2s ease;
  }

  .mode:hover {
    transform: translateY(-2px);
    border-color: var(--panel-edge-hi);
  }

  .mode.active {
    border-color: transparent;
    /* Static glow: animating box-shadow can't be composited and kept the
       card re-rasterizing every frame, forever. */
    box-shadow: 0 0 24px rgba(68, 214, 44, 0.26), inset 0 0 0 1px rgba(68, 214, 44, 0.7);
  }

  .icon {
    color: var(--ink-dim);
    transition: color 0.2s ease;
  }

  .mode.active .icon {
    color: var(--green);
  }

  .title {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.04em;
  }

  .blurb {
    font-size: 11.5px;
    line-height: 1.45;
    color: var(--ink-dim);
  }

  .led {
    position: absolute;
    top: 14px;
    right: 14px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #20262a;
    transition: background 0.2s ease, box-shadow 0.2s ease;
  }

  .mode.active .led {
    background: var(--green);
    box-shadow: 0 0 8px var(--green);
  }
</style>
