<script>
  import { onDestroy } from 'svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { openExternal } from '../lib/bridge.js';

  const WALLETS = [
    {
      id: 'btc',
      name: 'Bitcoin',
      symbol: 'BTC',
      network: 'Bitcoin network',
      address: 'bc1q4c3s3y22gmezt3pntls933gcvglek2w5gjjmuq'
    },
    {
      id: 'usdt',
      name: 'Tether',
      symbol: 'USDT',
      network: 'Confirm network before sending',
      address: '0xe5a217c178aff5D4f28224c304560656a22d60E4'
    },
    {
      id: 'sol',
      name: 'Solana',
      symbol: 'SOL',
      network: 'Solana network',
      address: '9sHGxcC3qQRmiBEqLJNqTodm3NnAq1hmam8ZzqUnzXbE'
    }
  ];

  const GOALS = [
    {
      icon: 'windows',
      title: 'A cleaner Windows edition',
      text: 'Explore a Windows 11 version that works fully offline—no required account, cloud connection, ads or bundled bloatware.'
    },
    {
      icon: 'download',
      title: 'Fedora & RHEL support',
      text: 'Explore native RPM packages and a first-class install and update path for Fedora and RHEL-family systems.'
    },
    {
      icon: 'layers',
      title: 'More hardware support',
      text: 'Test more Blade models, improve reliability and keep every hardware control safety-first.'
    }
  ];

  const PERIPHERALS = [
    'Mice',
    'Standalone keyboards',
    'Headsets',
    'Microphones',
    'Mouse docks & charging stations',
    'RGB mats & controllers'
  ];

  let copied = '';
  let copyError = '';
  let resetTimer;

  function fallbackCopy(value) {
    const field = document.createElement('textarea');
    field.value = value;
    field.setAttribute('readonly', '');
    field.style.position = 'fixed';
    field.style.opacity = '0';
    document.body.appendChild(field);
    field.select();
    const succeeded = document.execCommand('copy');
    field.remove();
    if (!succeeded) throw new Error('copy command was rejected');
  }

  async function copyAddress(wallet) {
    copyError = '';
    try {
      if (navigator.clipboard?.writeText) {
        try {
          await navigator.clipboard.writeText(wallet.address);
        } catch {
          fallbackCopy(wallet.address);
        }
      } else {
        fallbackCopy(wallet.address);
      }
      copied = wallet.id;
      clearTimeout(resetTimer);
      resetTimer = setTimeout(() => (copied = ''), 2200);
    } catch (error) {
      console.error('copy wallet address', error);
      copied = '';
      copyError = 'Could not copy automatically. Select the address and copy it manually.';
    }
  }

  onDestroy(() => clearTimeout(resetTimer));
</script>

<div class="support">
  <div class="hero card rise">
    <div class="hero-copy">
      <span class="eyebrow mono">Community powered · independent · open source</span>
      <h2>Help Fang grow beyond laptop control.</h2>
      <p>
        Donations support development time, hardware testing and new features while
        keeping Fang private, lightweight and useful without mandatory online services.
      </p>
      <button
        type="button"
        class="project-link"
        on:click={() => openExternal('https://github.com/bladeandsoulx/fang-razer-linux')}
      >
        View the open-source project <span aria-hidden="true">↗</span>
      </button>
    </div>
    <div class="hero-mark" aria-hidden="true">
      <Icon name="heart" size={42} stroke={1.5} />
      <span class="pulse"></span>
    </div>
  </div>

  <div class="section-head rise" style="animation-delay:60ms">
    <div>
      <span class="card-label">Donate crypto</span>
      <h3>Fang creator wallets</h3>
    </div>
    <p>Every contribution helps fund the next round of development and testing.</p>
  </div>

  <div class="responsible card rise" style="animation-delay:80ms" role="note">
    <span class="responsible-icon"><Icon name="heart" size={20} /></span>
    <div>
      <strong>Take care of yourself first.</strong>
      <p>
        Please do not donate if you are a student, living paycheck to paycheck,
        or working hard just to cover essential costs. Only donate money you can
        comfortably spare. Using, sharing or contributing to Fang is already support.
      </p>
    </div>
  </div>

  <div class="wallets">
    {#each WALLETS as wallet, i}
      <article class="wallet card rise" style="animation-delay:{90 + i * 45}ms">
        <div class="wallet-head">
          <div>
            <strong>{wallet.name}</strong>
            <span>{wallet.network}</span>
          </div>
          <span class="symbol mono">{wallet.symbol}</span>
        </div>
        <code>{wallet.address}</code>
        <button
          type="button"
          class:copied={copied === wallet.id}
          aria-label="Copy {wallet.name} wallet address"
          on:click={() => copyAddress(wallet)}
        >
          <Icon name={copied === wallet.id ? 'check' : 'copy'} size={14} />
          {copied === wallet.id ? 'Copied' : 'Copy address'}
        </button>
      </article>
    {/each}
  </div>

  <div class="safety rise" style="animation-delay:240ms" role="note">
    <Icon name="warn" size={15} />
    <p>
      Crypto transfers cannot be reversed. Verify the complete address and network
      before sending. For USDT, confirm the intended network with the Fang creator first.
    </p>
  </div>
  {#if copyError}<p class="copy-error" role="alert">{copyError}</p>{/if}

  <div class="section-head roadmap-head rise" style="animation-delay:270ms">
    <div>
      <span class="card-label">What support unlocks</span>
      <h3>Future directions</h3>
    </div>
    <p>Ideas to explore—not promises or finished features yet.</p>
  </div>

  <article class="peripherals card rise" style="animation-delay:300ms">
    <span class="peripheral-icon"><Icon name="keyboard" size={25} /></span>
    <div class="peripheral-copy">
      <div class="peripheral-title">
        <strong>Peripherals</strong>
        <span class="planned mono">Future direction</span>
      </div>
      <p>
        Fang could grow to recognize and configure external Razer devices connected
        to the laptop, with profiles, lighting and device-specific settings.
      </p>
    </div>
    <ul aria-label="Potential Razer peripheral support">
      {#each PERIPHERALS as peripheral}
        <li>{peripheral}</li>
      {/each}
    </ul>
  </article>

  <div class="goals">
    {#each GOALS as goal, i}
      <article class="goal card rise" style="animation-delay:{345 + i * 45}ms">
        <span class="goal-icon"><Icon name={goal.icon} size={23} /></span>
        <strong>{goal.title}</strong>
        <p>{goal.text}</p>
      </article>
    {/each}
  </div>
</div>

<style>
  .support {
    display: flex;
    flex-direction: column;
    gap: 18px;
    max-width: 1040px;
  }

  .hero {
    position: relative;
    min-height: 190px;
    padding: 28px 30px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 28px;
  }

  .hero::before {
    content: '';
    position: absolute;
    inset: 0;
    background:
      radial-gradient(420px 220px at 82% 50%, rgba(68, 214, 44, 0.12), transparent 68%),
      linear-gradient(105deg, rgba(68, 214, 44, 0.035), transparent 55%);
    pointer-events: none;
  }

  .hero-copy {
    position: relative;
    z-index: 1;
    max-width: 650px;
  }

  .eyebrow {
    color: var(--green);
    font-size: 9.5px;
    letter-spacing: 0.17em;
    text-transform: uppercase;
  }

  h2 {
    max-width: 620px;
    margin-top: 11px;
    font-size: clamp(23px, 3vw, 34px);
    line-height: 1.12;
    letter-spacing: -0.025em;
    color: var(--ink);
  }

  .hero-copy > p {
    max-width: 65ch;
    margin-top: 12px;
    color: var(--ink-dim);
    font-size: 12.5px;
    line-height: 1.6;
  }

  .project-link {
    margin-top: 18px;
    padding: 9px 13px;
    border: 1px solid rgba(68, 214, 44, 0.36);
    border-radius: 7px;
    color: var(--green);
    background: rgba(68, 214, 44, 0.07);
    font-family: var(--font-data);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    transition: all 0.15s ease;
  }

  .project-link:hover {
    color: var(--green-soft);
    border-color: rgba(68, 214, 44, 0.58);
    background: rgba(68, 214, 44, 0.11);
  }

  .hero-mark {
    position: relative;
    z-index: 1;
    display: grid;
    place-items: center;
    width: 100px;
    height: 100px;
    flex: 0 0 100px;
    border: 1px solid rgba(68, 214, 44, 0.35);
    border-radius: 50%;
    color: var(--green);
    background: rgba(10, 13, 14, 0.68);
    box-shadow: 0 0 42px rgba(68, 214, 44, 0.12), inset 0 0 24px rgba(68, 214, 44, 0.05);
  }

  .pulse {
    position: absolute;
    inset: 9px;
    border: 1px solid rgba(68, 214, 44, 0.13);
    border-radius: 50%;
  }

  .section-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    padding: 2px 2px 0;
  }

  .section-head h3 {
    margin-top: 6px;
    color: var(--ink);
    font-size: 17px;
    font-weight: 600;
  }

  .section-head > p {
    max-width: 48ch;
    color: var(--ink-dim);
    font-size: 11.5px;
    line-height: 1.45;
    text-align: right;
  }

  .wallets,
  .goals {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(215px, 1fr));
    gap: 12px;
  }

  .responsible {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 15px 17px;
    border-color: rgba(68, 214, 44, 0.23);
    background: linear-gradient(135deg, rgba(68, 214, 44, 0.065), var(--panel));
  }

  .responsible-icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    flex: 0 0 34px;
    border: 1px solid rgba(68, 214, 44, 0.28);
    border-radius: 50%;
    color: var(--green);
    background: rgba(68, 214, 44, 0.06);
  }

  .responsible strong {
    color: var(--ink);
    font-size: 12.5px;
  }

  .responsible p {
    max-width: 82ch;
    margin-top: 4px;
    color: var(--ink-dim);
    font-size: 11px;
    line-height: 1.55;
  }

  .wallet {
    min-width: 0;
    padding: 17px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .wallet-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .wallet-head div {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .wallet-head strong {
    font-size: 13px;
    color: var(--ink);
  }

  .wallet-head div span {
    font-size: 10px;
    color: var(--ink-faint);
  }

  .symbol {
    padding: 4px 7px;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 5px;
    color: var(--green);
    background: rgba(68, 214, 44, 0.05);
    font-size: 9.5px;
    letter-spacing: 0.08em;
  }

  code {
    min-height: 46px;
    padding: 9px 10px;
    border: 1px solid var(--panel-edge);
    border-radius: 6px;
    color: var(--ink-dim);
    background: rgba(7, 9, 10, 0.5);
    font-family: var(--font-data);
    font-size: 10.5px;
    line-height: 1.4;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .wallet button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    width: 100%;
    padding: 8px 10px;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 6px;
    color: var(--ink-dim);
    background: #15191c;
    font-family: var(--font-data);
    font-size: 9.5px;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    transition: all 0.15s ease;
  }

  .wallet button:hover,
  .wallet button.copied {
    color: var(--green);
    border-color: rgba(68, 214, 44, 0.4);
    background: rgba(68, 214, 44, 0.07);
  }

  .safety {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    margin-top: -4px;
    padding: 10px 12px;
    border: 1px solid rgba(255, 180, 84, 0.2);
    border-radius: 7px;
    color: var(--amber);
    background: rgba(255, 180, 84, 0.055);
  }

  .safety :global(svg) {
    flex: 0 0 auto;
    margin-top: 1px;
  }

  .safety p,
  .copy-error {
    font-size: 10.5px;
    line-height: 1.5;
  }

  .copy-error {
    margin-top: -10px;
    color: var(--red);
  }

  .roadmap-head {
    margin-top: 5px;
  }

  .peripherals {
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr);
    column-gap: 13px;
    row-gap: 14px;
    padding: 18px;
    border-color: rgba(68, 214, 44, 0.22);
  }

  .peripheral-icon {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border: 1px solid rgba(68, 214, 44, 0.28);
    border-radius: 9px;
    color: var(--green);
    background: rgba(68, 214, 44, 0.06);
  }

  .peripheral-copy {
    min-width: 0;
  }

  .peripheral-title {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .peripheral-title strong {
    color: var(--ink);
    font-size: 14px;
  }

  .planned {
    padding: 3px 6px;
    border: 1px solid rgba(255, 180, 84, 0.25);
    border-radius: 4px;
    color: var(--amber);
    background: rgba(255, 180, 84, 0.055);
    font-size: 8.5px;
    letter-spacing: 0.09em;
    text-transform: uppercase;
  }

  .peripheral-copy p {
    max-width: 76ch;
    margin-top: 5px;
    color: var(--ink-dim);
    font-size: 11px;
    line-height: 1.5;
  }

  .peripherals ul {
    grid-column: 2;
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
    list-style: none;
  }

  .peripherals li {
    padding: 6px 9px;
    border: 1px solid var(--panel-edge-hi);
    border-radius: 6px;
    color: var(--ink-dim);
    background: rgba(7, 9, 10, 0.38);
    font-family: var(--font-data);
    font-size: 9.5px;
  }

  .goal {
    padding: 18px;
    display: grid;
    grid-template-columns: 38px 1fr;
    column-gap: 11px;
    row-gap: 6px;
  }

  .goal-icon {
    grid-row: 1 / span 2;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border: 1px solid rgba(68, 214, 44, 0.24);
    border-radius: 8px;
    color: var(--green);
    background: rgba(68, 214, 44, 0.055);
  }

  .goal strong {
    align-self: end;
    color: var(--ink);
    font-size: 12.5px;
  }

  .goal p {
    color: var(--ink-dim);
    font-size: 11px;
    line-height: 1.5;
  }

  @media (max-width: 760px) {
    .hero {
      padding: 24px;
    }

    .hero-mark {
      display: none;
    }

    .section-head {
      align-items: flex-start;
      flex-direction: column;
      gap: 6px;
    }

    .section-head > p {
      text-align: left;
    }

    .peripherals ul {
      grid-column: 1 / -1;
    }
  }
</style>
