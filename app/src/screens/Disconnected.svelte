<div class="veil">
  <div class="box card">
    <svg viewBox="0 0 48 48" width="52" height="52">
      <path d="M10 8 L24 40 L27 26 L38 8 L30 8 L25 18 L19 8 Z" class="fang" />
    </svg>
    <h2>Daemon offline</h2>
    <p>Fang can't reach <span class="mono">fangd</span>. On your Blade, run:</p>
    <pre class="mono">sudo systemctl enable --now fangd
sudo usermod -aG fang $USER   # then log out/in once</pre>
    <p class="dim">Retrying automatically…</p>
  </div>
</div>

<style>
  .veil {
    position: fixed;
    inset: 0;
    background: rgba(6, 8, 9, 0.82);
    backdrop-filter: blur(6px);
    display: grid;
    place-items: center;
    z-index: 50;
  }

  .box {
    padding: 34px 40px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    text-align: center;
    max-width: 440px;
  }

  .fang {
    fill: var(--green);
    filter: drop-shadow(0 0 10px var(--green-glow));
  }

  h2 {
    font-size: 18px;
    letter-spacing: 0.06em;
  }

  p {
    font-size: 12.5px;
    color: var(--ink-dim);
  }

  pre {
    background: #0d1012;
    border: 1px solid var(--panel-edge);
    border-radius: 7px;
    padding: 12px 16px;
    font-size: 11.5px;
    color: var(--green-soft);
    text-align: left;
    user-select: text;
  }

  .dim {
    color: var(--ink-faint);
    animation: pulse 1.6s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.4;
    }
  }
</style>
