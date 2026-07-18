<script>
  export let checked = false;
  export let label = '';
  export let hint = '';
</script>

<label class="row">
  <span class="text">
    <span class="label">{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <input type="checkbox" bind:checked on:change />
  <span class="pill"><span class="knob"></span></span>
</label>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    cursor: pointer;
    padding: 14px 2px;
  }

  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .label {
    font-size: 13.5px;
    font-weight: 500;
  }

  .hint {
    font-size: 11.5px;
    color: var(--ink-dim);
  }

  input {
    display: none;
  }

  .pill {
    width: 40px;
    height: 22px;
    border-radius: 11px;
    background: #22282c;
    border: 1px solid #2c3439;
    position: relative;
    transition: background 0.2s ease, border-color 0.2s ease;
    flex-shrink: 0;
  }

  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #5a656d;
    transition: transform 0.18s cubic-bezier(0.3, 0.9, 0.4, 1.2), background 0.2s ease;
  }

  input:checked + .pill {
    background: rgba(68, 214, 44, 0.18);
    border-color: var(--green-dim);
  }

  input:checked + .pill .knob {
    transform: translateX(18px);
    background: var(--green);
    box-shadow: 0 0 8px var(--green-glow);
  }
</style>
