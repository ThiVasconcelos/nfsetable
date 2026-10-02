<script lang="ts">
  import { formatInt, plural } from '../lib/format'
  import { store } from '../lib/store.svelte'
  import Icon from './Icon.svelte'

  const status = $derived.by(() => {
    if (store.phase === 'scanning') return 'Procurando PDFs…'
    if (store.phase === 'extracting')
      return `Lendo notas… ${formatInt(store.progress.done)} de ${formatInt(store.progress.total)}`
    if (store.rows.length) return `${plural(store.rows.length, 'nota', 'notas')} na tabela`
    return ''
  })
</script>

<footer class="footer">
  <span class="privacy">
    <Icon name="shield" size={14} />
    Tudo roda no seu computador. Nenhum dado sai daqui.
  </span>
  <span class="right">
    {#if store.notesDoc.saveState === 'error'}
      <button
        type="button"
        class="unsaved"
        title="{store.notesDoc.saveError ?? 'Não foi possível salvar.'} {store.notesDoc.readBlocked
          ? 'Clique para ver as opções.'
          : 'Clique para tentar de novo.'}"
        onclick={() => store.notesDoc.retry()}
      >
        <Icon name="alertCircle" size={13} /> Edições não salvas
      </button>
    {/if}
    <span class="status" aria-live="polite">{status}</span>
    {#if store.info}
      <span class="version">v{store.info.version}</span>
    {/if}
  </span>
</footer>

<style>
  .footer {
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-4);
    padding: 0 var(--s-5);
    border-top: 1px solid var(--border);
    background: var(--surface);
    font-size: 12px;
    color: var(--text-3);
  }

  .privacy {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .privacy :global(svg) {
    color: var(--ok);
  }

  .right {
    display: inline-flex;
    align-items: center;
    gap: var(--s-4);
    flex: none;
  }

  .status {
    font-variant-numeric: tabular-nums;
  }

  .unsaved {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 6px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--danger);
    font-size: 12px;
  }

  .unsaved:hover {
    background: var(--danger-soft);
  }
</style>
