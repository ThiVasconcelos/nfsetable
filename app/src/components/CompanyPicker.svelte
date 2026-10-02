<script lang="ts">
  // "Escolha a empresa": the full-window choice when the app opens with 2+ companies (or from the
  // header menu). Nothing of a company is read or scanned before it is picked. The gear opens
  // "Configurações", where the data folder (the companies' documents) can be changed from here too.
  import { onMount, tick } from 'svelte'
  import { companies } from '../lib/companies.svelte'
  import { shortenPath } from '../lib/format'
  import { store } from '../lib/store.svelte'
  import Icon from './Icon.svelte'

  let list: HTMLUListElement | undefined = $state()

  onMount(() => {
    // The last company used gets the focus: Enter opens it.
    void tick().then(() => list?.querySelector<HTMLButtonElement>(`[data-id="${CSS.escape(companies.activeId)}"]`)?.focus())
  })

  function onKeydown(event: KeyboardEvent) {
    const buttons = [...(list?.querySelectorAll<HTMLButtonElement>('button.company') ?? [])]
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement)
    const to =
      event.key === 'ArrowDown' ? (index + 1) % buttons.length
      : event.key === 'ArrowUp' ? (index - 1 + buttons.length) % buttons.length
      : event.key === 'Home' ? 0
      : event.key === 'End' ? buttons.length - 1
      : -1
    if (to < 0) return
    event.preventDefault()
    buttons[to]?.focus()
  }
</script>

<main class="screen" aria-labelledby="picker-title" aria-busy={companies.busy}>
  <div class="panel">
    <div class="brand">
      <svg class="mark" viewBox="0 0 24 24" width="30" height="30" aria-hidden="true">
        <rect x="1" y="1" width="22" height="22" rx="6.5" fill="var(--accent)" />
        <path d="M7 8.25h10M7 12h10M7 15.75h6" stroke="#fff" stroke-width="1.8" stroke-linecap="round" />
      </svg>
      <span class="wordmark">nfse<span>table</span></span>
      <button
        type="button"
        class="btn btn-ghost btn-icon settings"
        aria-label="Configurações"
        title="Configurações (pasta dos dados)"
        disabled={companies.busy}
        onclick={() => (store.settingsOpen = true)}
      >
        <Icon name="settings" size={17} />
      </button>
    </div>

    <h1 id="picker-title">Escolha a empresa</h1>
    <p class="lead">Cada empresa tem as próprias pastas, notas e planejamento dos impostos.</p>

    {#if store.info?.dataDirError}
      <div class="notice" role="alert">
        <Icon name="alert" size={15} />
        <span>{store.info.dataDirError}</span>
      </div>
    {/if}

    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <ul class="list" bind:this={list} aria-label="Empresas" onkeydown={onKeydown}>
      {#each companies.list as c (c.id)}
        {@const last = c.id === companies.activeId}
        <li>
          <button
            type="button"
            class="company"
            class:last
            data-id={c.id}
            disabled={companies.busy}
            aria-label="Abrir {c.name}{c.cnpj ? `, CNPJ ${c.cnpj}` : ''}{last ? ', última usada' : ''}"
            onclick={() => companies.open(c.id)}
          >
            <span class="icon"><Icon name="building" size={18} /></span>
            <span class="text">
              <span class="name">{c.name}</span>
              {#if c.cnpj}<span class="cnpj num">{c.cnpj}</span>{/if}
            </span>
            {#if last}<span class="tag">última usada</span>{/if}
            <Icon name="chevronRight" size={16} class="go" />
          </button>
        </li>
      {/each}
    </ul>

    <div class="foot">
      <button type="button" class="btn" disabled={companies.busy} onclick={() => (companies.dialog = { mode: 'create' })}>
        <Icon name="plus" size={15} />
        Nova empresa
      </button>
      {#if companies.busy}
        <span class="opening"><span class="spinner"></span> Abrindo…</span>
      {/if}
    </div>

    {#if store.info}
      <p class="where" title="Pasta dos dados: {store.info.dataDir}">
        <Icon name="hardDrive" size={13} />
        <span>Pasta dos dados:</span>
        <span class="path mono">{shortenPath(store.info.dataDir, 3)}</span>
      </p>
    {/if}
  </div>
</main>

<style>
  .screen {
    height: 100%;
    display: grid;
    place-items: center;
    padding: var(--s-5);
    overflow: auto;
    background: var(--bg);
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    width: min(460px, 100%);
    padding: var(--s-6) var(--s-5) var(--s-5);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: 0 1px 2px rgba(16, 16, 24, 0.04);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: var(--s-2);
  }

  .wordmark {
    font-size: 17px;
    font-weight: 650;
    letter-spacing: -0.01em;
    color: var(--text);
  }

  .wordmark span {
    color: var(--accent-text);
  }

  .settings {
    margin-left: auto;
  }

  .notice {
    display: flex;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--warn-soft);
    color: var(--warn);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .notice :global(svg) {
    flex: none;
    margin-top: 1px;
  }

  h1 {
    font-size: 20px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }

  .lead {
    margin-top: -6px;
    font-size: 13px;
    color: var(--text-2);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: var(--s-2) 0 0;
    padding: 0;
    list-style: none;
  }

  .company {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 56px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    text-align: left;
    transition:
      border-color 0.12s var(--ease),
      background-color 0.12s var(--ease);
  }

  .company:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .company:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft-2);
  }

  .company:disabled {
    cursor: wait;
    opacity: 0.7;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    flex: none;
    border-radius: 9px;
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .text {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .name {
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cnpj {
    font-size: 12px;
    color: var(--text-3);
  }

  .tag {
    flex: none;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 11.5px;
    font-weight: 500;
  }

  .company :global(.go) {
    flex: none;
    color: var(--text-3);
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
    margin-top: var(--s-2);
  }

  .opening {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--text-3);
  }

  .where {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    margin-top: var(--s-1);
    padding-top: var(--s-3);
    border-top: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-3);
  }

  .where :global(svg) {
    flex: none;
  }

  .where .path {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-2);
  }
</style>
