<script>
  import FanSpinner from '../lib/components/FanSpinner.svelte';
  import { status, telemetry, avgRpm } from '../lib/stores.js';
  import { setFan } from '../lib/bridge.js';

  let slider = null;
  let curvePoints = [];
  let curveDirty = false;
  let busy = false;
  let error = '';

  $: mode = $status?.fan?.mode ?? 'auto';
  $: manual = mode === 'manual';
  $: curve = mode === 'curve';
  $: min = $status?.fan_rpm_min ?? 2200;
  $: max = $status?.fan_rpm_max ?? 5000;
  $: target = manual ? $status.fan.rpm : null;
  $: shown = slider ?? target ?? min;
  $: fill = ((shown - min) / Math.max(1, max - min)) * 100;
  $: runtimeTarget = $telemetry?.fan_target_rpm ?? $avgRpm;
  $: lastCurveTemp = curvePoints.length ? curvePoints[curvePoints.length - 1].temp_c : null;
  $: storedCurve = curve ? $status?.fan?.points : $status?.fan_curve;

  $: if (!curveDirty && storedCurve?.length) {
    curvePoints = storedCurve.map((point) => ({ ...point }));
  }
  $: if (!curveDirty && !curve && curvePoints.length === 0 && $status) {
    curvePoints = defaultCurve(min, max);
  }

  function defaultCurve(lo, hi) {
    const temps = [45, 60, 70, 80, 90];
    const levels = [0, 0.2, 0.45, 0.72, 1];
    return temps.map((temp_c, i) => ({
      temp_c,
      rpm: Math.round((lo + (hi - lo) * levels[i]) / 100) * 100
    }));
  }

  async function applyFan(fan) {
    error = '';
    busy = true;
    try {
      await setFan(fan);
      if (fan.mode !== 'curve' && curveDirty) {
        curvePoints = defaultCurve(min, max);
      }
      curveDirty = false;
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }

  function toAuto() {
    slider = null;
    applyFan({ mode: 'auto' });
  }

  function toManual() {
    applyFan({ mode: 'manual', rpm: target ?? Math.round((min + max) / 200) * 100 });
  }

  function toCurve() {
    const points = curvePoints.length ? curvePoints : defaultCurve(min, max);
    curvePoints = points;
    applyFan({ mode: 'curve', points });
  }

  function commitManual(e) {
    slider = null;
    applyFan({ mode: 'manual', rpm: +e.target.value });
  }

  function editPoint(index, field, value) {
    curvePoints = curvePoints.map((point, i) =>
      i === index ? { ...point, [field]: +value } : point
    );
    curveDirty = true;
  }

  function removePoint(index) {
    if (curvePoints.length <= 2) return;
    curvePoints = curvePoints.filter((_, i) => i !== index);
    curveDirty = true;
  }

  function addPoint() {
    if (curvePoints.length >= 8) return;
    const last = curvePoints[curvePoints.length - 1] ?? { temp_c: 40, rpm: min };
    if (last.temp_c >= 100) return;
    curvePoints = [
      ...curvePoints,
      { temp_c: Math.min(100, last.temp_c + 5), rpm: last.rpm }
    ];
    curveDirty = true;
  }

  function applyCurve() {
    applyFan({ mode: 'curve', points: curvePoints });
  }
</script>

<div class="wrap">
  <div class="visual card rise">
    <FanSpinner rpm={runtimeTarget ?? 0} size={190} />
    <div class="live">
      <span class="big mono">{runtimeTarget ?? '--'}</span>
      <span class="card-label">active fan target</span>
    </div>
    <div class="pair mono">
      {#each $telemetry?.fan_rpm ?? [] as r, i}
        <span>FAN{i + 1}<em>{r}</em></span>
      {/each}
    </div>
    {#if $telemetry?.thermal_override_active}
      <div class="guard active" role="status">
        {#if $telemetry?.thermal_override_reason === 'sensor_unavailable'}
          CPU temperature sensor unavailable · forcing {max} RPM
        {:else}
          Thermal override active · forcing {max} RPM
        {/if}
      </div>
    {/if}
  </div>

  <div class="controls card rise" style="animation-delay:70ms">
    <span class="card-label">Fan mode</span>
    <div class="seg" aria-label="Fan mode">
      <button class:on={mode === 'auto'} aria-pressed={mode === 'auto'} disabled={busy} on:click={toAuto}>
        Auto
      </button>
      <button class:on={manual} aria-pressed={manual} disabled={busy} on:click={toManual}>
        Manual
      </button>
      <button class:on={curve} aria-pressed={curve} disabled={busy} on:click={toCurve}>
        Curve
      </button>
    </div>

    {#if manual}
      <div class="slider">
        <div class="target mono">{shown}<em>rpm target</em></div>
        <label class="range-label">
          <span>Manual fan target</span>
          <input
            type="range"
            {min}
            {max}
            step="100"
            value={shown}
            disabled={busy}
            style="--fill:{fill}%"
            on:input={(e) => (slider = +e.target.value)}
            on:change={commitManual}
          />
        </label>
        <div class="scale mono">
          <span>{min}</span>
          <span>{Math.round((min + max) / 200) * 100}</span>
          <span>{max}</span>
        </div>
      </div>
    {:else if curve}
      <div class="curve-editor">
        <div class="curve-head">
          <span class="card-label">Temperature points</span>
          <span class="mono live-target">now {runtimeTarget ?? '--'} rpm</span>
        </div>
        {#each curvePoints as point, i}
          <div class="curve-row">
            <label>
              <span>Point {i + 1} temperature</span>
              <input
                class="number mono"
                type="number"
                min="30"
                max="100"
                step="1"
                value={point.temp_c}
                on:input={(e) => editPoint(i, 'temp_c', e.target.value)}
              />
              <em>°C</em>
            </label>
            <label class="rpm-point">
              <span>Point {i + 1} fan target</span>
              <input
                type="range"
                {min}
                {max}
                step="100"
                value={point.rpm}
                style="--fill:{((point.rpm - min) / Math.max(1, max - min)) * 100}%"
                on:input={(e) => editPoint(i, 'rpm', e.target.value)}
              />
              <strong class="mono">{point.rpm}</strong>
            </label>
            <button
              class="remove"
              aria-label="Remove curve point {i + 1}"
              disabled={curvePoints.length <= 2}
              on:click={() => removePoint(i)}
            >×</button>
          </div>
        {/each}
        <div class="curve-actions">
          <button
            class="add"
            disabled={curvePoints.length >= 8 || lastCurveTemp >= 100}
            on:click={addPoint}>+ Add point</button
          >
          <button class="apply" disabled={!curveDirty || busy} on:click={applyCurve}>
            {busy ? 'Applying…' : curveDirty ? 'Apply curve' : 'Curve applied'}
          </button>
        </div>
      </div>
    {:else}
      <div class="auto-note mono">EC thermal curve active</div>
    {/if}

    <div class="guard">
      <strong>Thermal safety is always on.</strong>
      Manual and Curve modes are forced to {max} RPM at CPU ≥95 °C or GPU ≥87 °C.
      The override cannot be disabled and releases only after temperatures cool.
    </div>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <p class="hint">
      Curve mode interpolates between your points using the hotter CPU/GPU sensor.
      Limits ({min}–{max} rpm) come from the model profile. Razer laptops expose
      no live tachometer, so displayed RPM is the EC target, not a measurement.
    </p>
  </div>
</div>

<style>
  .wrap {
    display: grid;
    grid-template-columns: minmax(240px, 320px) 1fr;
    gap: 14px;
    align-items: stretch;
  }

  .visual {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    padding: 26px 16px;
  }

  .live {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .big {
    font-size: 36px;
    color: var(--green-soft);
    text-shadow: 0 0 14px var(--green-glow);
  }

  .pair {
    display: flex;
    gap: 18px;
    font-size: 10.5px;
    letter-spacing: 0.08em;
    color: var(--ink-dim);
  }

  .pair em {
    font-style: normal;
    color: var(--ink);
    margin-left: 5px;
  }

  .controls {
    padding: 22px 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .seg {
    display: flex;
    width: fit-content;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 7px;
    overflow: hidden;
  }

  .seg button {
    padding: 9px 24px;
    font-family: var(--font-data);
    font-size: 12px;
    letter-spacing: 0.12em;
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

  .seg button:disabled {
    cursor: wait;
    opacity: 0.65;
  }

  .slider {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .range-label > span,
  .curve-row label > span {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  .target {
    font-size: 22px;
    color: var(--ink);
    min-height: 30px;
  }

  .target em {
    font-style: normal;
    font-size: 11.5px;
    letter-spacing: 0.1em;
    color: var(--ink-dim);
    margin-left: 8px;
  }

  .scale {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: var(--ink-faint);
  }

  .curve-editor {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--panel-edge);
    border-radius: 8px;
    background: rgba(8, 10, 11, 0.24);
  }

  .curve-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 2px;
  }

  .live-target {
    color: var(--green-soft);
    font-size: 11px;
  }

  .curve-row {
    display: grid;
    grid-template-columns: 108px minmax(200px, 1fr) 28px;
    align-items: center;
    gap: 12px;
  }

  .curve-row label {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .curve-row em {
    font-style: normal;
    font-size: 11px;
    color: var(--ink-dim);
  }

  .number {
    width: 70px;
    padding: 7px 8px;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 6px;
    color: var(--ink);
    background: #111518;
  }

  .rpm-point strong {
    width: 42px;
    text-align: right;
    font-size: 11px;
    color: var(--green-soft);
  }

  .apply {
    padding: 8px 13px;
    border: 1px solid var(--green-dim);
    border-radius: 6px;
    color: var(--green);
    background: rgba(68, 214, 44, 0.1);
  }

  .apply:disabled {
    border-color: var(--panel-edge-hi);
    color: var(--ink-faint);
    background: #15191c;
    cursor: default;
  }

  .curve-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 2px;
  }

  .add {
    padding: 7px 9px;
    color: var(--ink-dim);
    font-size: 11px;
  }

  .add:hover:not(:disabled) {
    color: var(--ink);
  }

  .remove {
    width: 26px;
    height: 26px;
    border-radius: 5px;
    color: var(--ink-dim);
    font-size: 18px;
    line-height: 1;
  }

  .remove:hover:not(:disabled) {
    color: var(--red);
    background: rgba(255, 92, 92, 0.08);
  }

  .add:disabled,
  .remove:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .auto-note {
    padding: 16px;
    border: 1px solid var(--panel-edge);
    border-radius: 7px;
    font-size: 12px;
    color: var(--green-soft);
  }

  .guard {
    padding: 9px 12px;
    border: 1px solid rgba(255, 180, 84, 0.25);
    border-radius: 7px;
    background: rgba(255, 180, 84, 0.06);
    color: var(--amber);
    font-size: 11.5px;
    line-height: 1.45;
  }

  .guard strong {
    display: block;
  }

  .guard.active {
    text-align: center;
    background: rgba(255, 92, 92, 0.1);
    border-color: rgba(255, 92, 92, 0.35);
    color: var(--red);
  }

  .error {
    color: var(--red);
    font-size: 11.5px;
  }

  .hint {
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--ink-dim);
    margin-top: auto;
  }
</style>
