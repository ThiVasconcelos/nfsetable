<script lang="ts">
  import { formatInt, plural } from '../lib/format'
  import { store } from '../lib/store.svelte'
  import ExcludeChips from './ExcludeChips.svelte'
  import Icon from './Icon.svelte'
  import SourceItem from './SourceItem.svelte'

  interface Props {
    /** Show only a narrow rail (keeps room for the table while the preview is open). */
    collapsed: boolean
    ontoggle: () => void
  }

  let { collapsed, ontoggle }: Props = $props()

  const excludedCount = $derived(store.scan?.excluded.length ?? 0)
</script>

{#if collapsed}
  <aside class="rail" aria-label="Fontes (recolhido)" class:dragging={store.dragging}>
    <button type="button" class="rail-btn" title="Mostrar fontes" aria-label="Mostrar o painel de fontes" onclick={ontoggle}>
      <Icon name="panelOpen" size={18} />
    </button>
    <span class="rail-sep" aria-hidden="true"></span>
    <button type="button" class="rail-btn" title="Adicionar pasta" aria-label="Adicionar pasta" onclick={() => store.pickFolders()}>
      <Icon name="folderPlus" size={18} />
    </button>
    <button type="button" class="rail-btn" title="Adicionar arquivos" aria-label="Adicionar arquivos" onclick={() => store.pickFiles()}>
      <Icon name="filePlus" size={18} />
    </button>
    {#if store.sources.length}
      <button
        type="button"
        class="rail-btn with-count"
        title={plural(store.sources.length, 'fonte', 'fontes')}
        aria-label="{plural(store.sources.length, 'fonte', 'fontes')}. Mostrar o painel de fontes"
        onclick={ontoggle}
      >
        <Icon name="folder" size={18} />
        <span class="rail-count num">{formatInt(store.sources.length)}</span>
      </button>
    {/if}
  </aside>
{:else}
  <aside class="sidebar" aria-labelledby="sources-title">
    <section class="block">
      <div class="block-head">
        <h2 id="sources-title">Fontes</h2>
        {#if store.sources.length}<span class="count num">{store.sources.length}</span>{/if}
        <button
          type="button"
          class="btn btn-ghost btn-icon btn-sm collapse"
          title="Recolher painel"
          aria-label="Recolher o painel de fontes"
          onclick={ontoggle}
        >
          <Icon name="panelClose" size={16} />
        </button>
      </div>

      <div class="add">
        <button type="button" class="btn" onclick={() => store.pickFolders()}>
          <Icon name="folderPlus" size={15} />
          Adicionar pasta
        </button>
        <button type="button" class="btn" onclick={() => store.pickFiles()}>
          <Icon name="filePlus" size={15} />
          Adicionar arquivos
        </button>
      </div>

      <div class="dropzone" class:active={store.dragging}>
        <Icon name="drop" size={18} />
        <span>{store.dragging ? 'Solte para adicionar' : 'Arraste pastas ou PDFs para cá'}</span>
      </div>

      {#if store.sources.length}
        <ul class="sources">
          {#each store.sources as source (source.path)}
            <SourceItem {source} info={store.sourceInfo(source)} />
          {/each}
        </ul>
      {/if}
    </section>

    <section class="block">
      <div class="block-head">
        <h2 id="exclude-title">Ignorar arquivos que contenham</h2>
      </div>
      <ExcludeChips />
      {#if store.scan}
        {#if excludedCount}
          <button type="button" class="link ignored" onclick={() => (store.excludedOpen = true)}>
            <Icon name="eyeOff" size={14} />
            {plural(excludedCount, 'ignorado', 'ignorados')}
          </button>
        {:else}
          <span class="ignored none">Nenhum arquivo ignorado</span>
        {/if}
      {/if}
    </section>
  </aside>
{/if}

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: var(--s-5);
    padding: 20px var(--s-4) var(--s-5);
    background: var(--surface);
    border-right: 1px solid var(--border);
    overflow-y: auto;
    min-height: 0;
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
  }

  .block-head {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    min-height: 28px;
  }

  h2 {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--text-3);
  }

  .count {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--surface-3);
    font-size: 11px;
    font-weight: 600;
    line-height: 18px;
    text-align: center;
    color: var(--text-2);
  }

  .collapse {
    margin-left: auto;
    margin-right: -6px;
    color: var(--text-3);
  }

  .add {
    display: grid;
    grid-template-columns: 1fr;
    gap: var(--s-2);
  }

  .add .btn {
    justify-content: flex-start;
    height: 34px;
  }

  .dropzone {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--s-2);
    min-height: 56px;
    padding: var(--s-3);
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
    color: var(--text-3);
    font-size: 12.5px;
    text-align: center;
    transition:
      background-color 0.15s var(--ease),
      border-color 0.15s var(--ease),
      color 0.15s var(--ease);
  }

  .dropzone.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .sources {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .ignored {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    align-self: flex-start;
    font-size: 12.5px;
  }

  .ignored.none {
    color: var(--text-3);
  }

  /* ------------------------------------------------------------ rail */

  .rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-1);
    padding: var(--s-3) 0;
    background: var(--surface);
    border-right: 1px solid var(--border);
    min-height: 0;
    transition: background-color 0.15s var(--ease);
  }

  .rail.dragging {
    background: var(--accent-soft);
  }

  .rail-btn {
    position: relative;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border: 0;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);
    transition:
      background-color 0.12s var(--ease),
      color 0.12s var(--ease);
  }

  .rail-btn:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .rail-sep {
    width: 24px;
    height: 1px;
    margin: var(--s-1) 0;
    background: var(--border);
  }

  .rail-count {
    position: absolute;
    top: 3px;
    right: 1px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 10px;
    font-weight: 600;
    line-height: 16px;
  }
</style>
