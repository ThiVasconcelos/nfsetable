<script lang="ts">
  import { isMock } from '../lib/api'
  import { store, type View } from '../lib/store.svelte'
  import CompanySwitcher from './CompanySwitcher.svelte'
  import ExportMenu from './ExportMenu.svelte'
  import Icon from './Icon.svelte'
  import type { IconName } from './icons'
  import ThemeToggle from './ThemeToggle.svelte'

  const VIEWS: { id: View; label: string; icon: IconName; hint: string }[] = [
    { id: 'notes', label: 'Notas', icon: 'table', hint: 'A tabela com as suas notas' },
    { id: 'taxes', label: 'Impostos', icon: 'calculator', hint: 'MEI, Simples Nacional, pró-labore e quanto sobra' },
  ]
</script>

<header class="header">
  <div class="brand">
    <svg class="mark" viewBox="0 0 24 24" width="26" height="26" aria-hidden="true">
      <rect x="1" y="1" width="22" height="22" rx="6.5" fill="var(--accent)" />
      <path d="M7 8.25h10M7 12h10M7 15.75h6" stroke="#fff" stroke-width="1.8" stroke-linecap="round" />
    </svg>
    <div class="names">
      <span class="wordmark">nfse<span>table</span></span>
      <span class="tagline">Suas NFS-e em uma tabela</span>
    </div>
    {#if isMock}
      <span class="demo" title="Rodando no navegador com dados fictícios">demonstração</span>
    {/if}
  </div>

  <CompanySwitcher />

  <nav class="views" aria-label="Seções">
    {#each VIEWS as v (v.id)}
      <button
        type="button"
        class:active={store.view === v.id}
        aria-current={store.view === v.id ? 'page' : undefined}
        title={v.hint}
        onclick={() => store.setView(v.id)}
      >
        <Icon name={v.icon} size={15} />
        {v.label}
      </button>
    {/each}
  </nav>

  <div class="actions">
    {#if store.view === 'notes'}
      <button
        type="button"
        class="btn btn-ghost"
        title="Perfis e regras: como as notas são lidas e classificadas"
        onclick={() => store.openRules()}
      >
        <Icon name="layers" size={15} />
        Regras
      </button>
      <button
        type="button"
        class="btn copy"
        onclick={() => store.copyTotal()}
        disabled={!store.rows.length}
        aria-label="Copiar total"
        title="Copiar o total para colar em outro lugar"
      >
        <Icon name="copy" size={15} />
        <span class="copy-label">Copiar total</span>
      </button>
      <ExportMenu />
      <span class="sep" aria-hidden="true"></span>
    {/if}
    <ThemeToggle />
    <button
      type="button"
      class="btn btn-ghost btn-icon"
      aria-label="Configurações"
      title="Configurações (pasta dos dados)"
      onclick={() => (store.settingsOpen = true)}
    >
      <Icon name="settings" size={17} />
    </button>
  </div>
</header>

<style>
  .header {
    height: var(--header-h);
    display: flex;
    align-items: center;
    gap: var(--s-4);
    padding: 0 var(--s-4) 0 var(--s-5);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .mark {
    flex: none;
  }

  .names {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }

  .wordmark {
    font-size: 16px;
    font-weight: 650;
    letter-spacing: -0.01em;
    color: var(--text);
  }

  .wordmark span {
    color: var(--accent-text);
  }

  .tagline {
    font-size: 13px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .demo {
    flex: none;
    padding: 2px 8px;
    border: 1px dashed var(--border-strong);
    border-radius: 999px;
    font-size: 11.5px;
    color: var(--text-3);
  }

  .views {
    display: inline-flex;
    flex: none;
    padding: 3px;
    gap: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .views button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px 0 10px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    transition:
      background-color 0.12s var(--ease),
      color 0.12s var(--ease);
  }

  .views button:hover {
    color: var(--text);
  }

  .views button.active {
    background: var(--surface);
    color: var(--text);
    box-shadow:
      0 0 0 1px var(--border),
      0 1px 2px rgba(0, 0, 0, 0.04);
  }

  .views button.active :global(svg) {
    color: var(--accent-text);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    flex: none;
    margin-left: auto;
  }

  .sep {
    width: 1px;
    height: 20px;
    margin: 0 var(--s-1);
    background: var(--border);
  }

  @media (max-width: 1100px) {
    .tagline,
    .demo {
      display: none;
    }

    .copy {
      padding: 0 9px;
    }

    .copy-label {
      display: none;
    }
  }
</style>
