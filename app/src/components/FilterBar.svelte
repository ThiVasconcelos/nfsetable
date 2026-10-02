<script lang="ts">
  import { formatInt, plural } from '../lib/format'
  import { NOTES_PRESETS } from '../lib/period'
  import { store, type BaseFilter, type KindFilter } from '../lib/store.svelte'
  import Icon from './Icon.svelte'
  import PeriodPicker from './PeriodPicker.svelte'

  const FILTERS: { id: BaseFilter; label: string }[] = [
    { id: 'all', label: 'Todas' },
    { id: 'ok', label: 'OK' },
    { id: 'attention', label: 'Com erro' },
    { id: 'edited', label: 'Editadas' },
  ]

  const KINDS: { id: Exclude<KindFilter, 'all'>; label: string; hint: string }[] = [
    { id: 'revenue', label: 'Receitas', hint: 'Só as receitas (notas emitidas por você)' },
    { id: 'expense', label: 'Despesas', hint: 'Só as despesas (contas pagas pela empresa)' },
  ]

  let searchInput: HTMLInputElement | undefined = $state()

  /** The Receitas / Despesas filter only shows up when the table has expenses (or it is on). */
  const showKinds = $derived(store.totals.expenseRows > 0 || store.kindFilter !== 'all')

  /** Rows sent from the tax page that still have no competence. */
  const todoLeft = $derived(
    store.filter === 'noCompetence' ? store.visibleRows.filter((r) => !r.competence).length : 0,
  )

  /** The competence task is done: back to all notes, then to the tax page. */
  function backToTaxes() {
    store.setFilter('all')
    store.setView('taxes')
  }

  function onWindowKeydown(event: KeyboardEvent) {
    // Ctrl/Cmd+F focuses the search box.
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'f') {
      event.preventDefault()
      searchInput?.focus()
      searchInput?.select()
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div class="bar">
  {#if store.filter === 'noCompetence'}
    <div class="todo" role="status">
      <button type="button" class="back" title="Mostrar todas as notas" onclick={() => store.setFilter('all')}>
        <Icon name="chevronLeft" size={15} />
        Todas
      </button>
      <span class="todo-text">
        <Icon name="calendar" size={14} />
        {#if todoLeft}
          {plural(todoLeft, 'nota sem competência', 'notas sem competência')}: dê um duplo clique na coluna Competência
        {:else}
          <span class="done"><Icon name="check" size={14} /> Competências preenchidas</span>
        {/if}
      </span>
      <button type="button" class="link" onclick={backToTaxes}>Ver impostos</button>
    </div>
  {:else}
    <div class="filters">
      <PeriodPicker
        value={store.period}
        presets={NOTES_PRESETS}
        undated={{ include: store.period.includeUndated, count: store.periodInfo.undatedAll, left: store.periodInfo.undated }}
        summary={store.periodInfo.active
          ? `${plural(store.periodRows.length, 'nota no período', 'notas no período')} de ${formatInt(store.rows.length)}`
          : null}
        onchange={(p) => store.setPeriod(p)}
        onundated={(include) => store.setIncludeUndated(include)}
      />
      <div class="segmented" role="group" aria-label="Filtrar notas">
        {#each FILTERS as f (f.id)}
          <button
            type="button"
            class:active={store.filter === f.id}
            aria-pressed={store.filter === f.id}
            onclick={() => store.setFilter(f.id)}
          >
            {f.label}
            <span class="n num" class:warn={f.id === 'attention' && store.counts[f.id] > 0}>{formatInt(store.counts[f.id])}</span>
          </button>
        {/each}
      </div>
      {#if showKinds}
        <div class="segmented kinds" role="group" aria-label="Natureza">
          {#each KINDS as k (k.id)}
            <button
              type="button"
              class:active={store.kindFilter === k.id}
              aria-pressed={store.kindFilter === k.id}
              title={store.kindFilter === k.id ? 'Clique de novo para ver todas' : k.hint}
              onclick={() => store.setKindFilter(store.kindFilter === k.id ? 'all' : k.id)}
            >
              {k.label}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <span class="spacer" aria-hidden="true"></span>
  <label class="search">
    <Icon name="search" size={15} />
    <input
      bind:this={searchInput}
      type="search"
      placeholder="Buscar arquivo…"
      aria-label="Buscar por nome de arquivo"
      bind:value={store.search}
      onkeydown={(e) => {
        if (e.key === 'Escape') store.search = ''
      }}
    />
    {#if store.search}
      <button type="button" class="clear" aria-label="Limpar busca" onclick={() => (store.search = '')}>
        <Icon name="x" size={13} />
      </button>
    {/if}
  </label>
</div>

<style>
  /* One wrapping row: the period, the status filter and Receitas / Despesas from the left, the
     search at the right end of their line; whatever does not fit goes to the next line (the search
     alone there starts at the left). */
  .bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s-2);
  }

  /* Takes the free space before the search; its negative margin cancels its gap, so it never
     needs a line of its own. */
  .spacer {
    flex: 1 1 0;
    min-width: 0;
    margin-left: calc(-1 * var(--s-2));
  }

  .todo {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    min-width: 0;
    height: 36px;
    padding: 3px 12px 3px 3px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
    font-size: 13px;
  }

  .back {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 28px;
    padding: 0 10px 0 6px;
    border: 0;
    border-radius: 5px;
    background: var(--surface);
    box-shadow: 0 0 0 1px var(--border);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
  }

  .back:hover {
    background: var(--surface-3);
  }

  .todo-text {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .todo-text :global(svg) {
    flex: none;
    color: var(--warn);
  }

  .done {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--ok);
    font-weight: 500;
  }

  .done :global(svg) {
    color: var(--ok);
  }

  .todo .link {
    flex: none;
    font-size: 13px;
    white-space: nowrap;
  }

  /* The filters are items of the bar itself, so each one wraps on its own. */
  .filters {
    display: contents;
  }

  .segmented {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .segmented button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
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

  .segmented button:hover {
    color: var(--text);
  }

  .segmented button.active {
    background: var(--surface);
    color: var(--text);
    box-shadow:
      0 0 0 1px var(--border),
      0 1px 2px rgba(0, 0, 0, 0.04);
  }

  .kinds button.active {
    color: var(--accent-text);
  }

  .n {
    min-width: 18px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--surface-3);
    font-size: 11.5px;
    line-height: 18px;
    text-align: center;
    color: var(--text-3);
  }

  .active .n {
    background: var(--surface-2);
  }

  .n.warn {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .search {
    position: relative;
    display: flex;
    align-items: center;
    /* Grows first (up to 260px); the spacer takes what is left. */
    flex: 100 1 160px;
    max-width: 260px;
    min-width: 140px;
    height: 34px;
    padding: 0 8px 0 10px;
    gap: 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text-3);
    transition:
      border-color 0.12s var(--ease),
      box-shadow 0.12s var(--ease);
  }

  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft-2);
  }

  .search input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 13px;
  }

  .search input::placeholder {
    color: var(--text-3);
  }

  .search input::-webkit-search-cancel-button {
    display: none;
  }

  .clear {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-3);
  }

  .clear:hover {
    background: var(--surface-2);
    color: var(--text);
  }
</style>
