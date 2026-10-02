<script lang="ts">
  import { tick } from 'svelte'
  import { formatBRL, formatInt, plural, shortenPath } from '../lib/format'
  import { STATUS_HINT, originLabel } from '../lib/labels'
  import { KIND_LABEL, store, type Row, type SortKey } from '../lib/store.svelte'
  import BulkTypeMenu from './BulkTypeMenu.svelte'
  import CompetenceCell from './CompetenceCell.svelte'
  import Icon from './Icon.svelte'
  import RowMenu from './RowMenu.svelte'
  import StatusBadge from './StatusBadge.svelte'
  import TypeCell from './TypeCell.svelte'
  import TypeTotals from './TypeTotals.svelte'
  import ValueCell from './ValueCell.svelte'

  const COLUMNS: { key: SortKey; label: string; cls: string }[] = [
    { key: 'file', label: 'Arquivo', cls: 'c-file' },
    { key: 'type', label: 'Tipo', cls: 'c-type' },
    { key: 'kind', label: 'Natureza', cls: 'c-kind' },
    { key: 'competence', label: 'Competência', cls: 'c-comp' },
    { key: 'value', label: 'Valor', cls: 'c-value' },
    { key: 'status', label: 'Status', cls: 'c-status' },
    { key: 'origin', label: 'Origem', cls: 'c-origin' },
  ]
  /** Rows rendered above and below the viewport (the table only renders what is visible). */
  const OVERSCAN = 4

  let card: HTMLDivElement | undefined = $state()
  let scroller: HTMLDivElement | undefined = $state()
  let thead: HTMLTableSectionElement | undefined = $state()
  let tbody: HTMLTableSectionElement | undefined = $state()
  let scrollTop = $state(0)
  let viewportHeight = $state(0)
  let headHeight = $state(37)
  let rowHeight = $state(48)
  let focusPath = $state<string | null>(null)

  const rows = $derived(store.visibleRows)
  const checkedVisible = $derived(rows.reduce((n, r) => (store.selected.has(r.path) ? n + 1 : n), 0))
  const allChecked = $derived(rows.length > 0 && checkedVisible === rows.length)
  const someChecked = $derived(checkedVisible > 0 && !allChecked)
  const selectedCount = $derived(store.selectedRows.length)
  const hiddenSelected = $derived(Math.max(0, selectedCount - checkedVisible))
  const selectedHasRevenue = $derived(store.selectedRows.some((r) => r.kind === 'revenue'))
  const selectedHasExpense = $derived(store.selectedRows.some((r) => r.kind === 'expense'))

  // ------------------------------------------------------------ windowing (fixed row height)

  const contentHeight = $derived(headHeight + rows.length * rowHeight)
  const top = $derived(Math.max(0, Math.min(scrollTop, contentHeight - viewportHeight)))
  const firstVisible = $derived(Math.min(rows.length - 1, Math.floor(top / rowHeight)))
  const start = $derived(Math.max(0, firstVisible - OVERSCAN))
  const end = $derived(
    Math.min(rows.length, Math.ceil((top + Math.max(rowHeight, viewportHeight - headHeight)) / rowHeight) + OVERSCAN),
  )
  const slice = $derived(rows.slice(start, end))
  const hasRows = $derived(slice.length > 0)

  // Roving tabindex: exactly one (rendered) row is reachable with Tab.
  const tabPath = $derived.by(() => {
    const rendered = (path: string | null) => path != null && slice.some((r) => r.path === path)
    if (rendered(focusPath)) return focusPath
    if (rendered(store.activePath)) return store.activePath
    return rows[firstVisible]?.path ?? null
  })

  // The CSS decides the row height (48px, 44px on short windows): measure it instead of guessing.
  $effect(() => {
    void viewportHeight
    if (!hasRows) return
    const tr = tbody?.querySelector<HTMLElement>('tr.row')
    const h = tr?.getBoundingClientRect().height ?? 0
    if (h > 0 && Math.abs(h - rowHeight) > 0.25) rowHeight = h
    const hh = thead?.getBoundingClientRect().height ?? 0
    if (hh > 0 && Math.abs(hh - headHeight) > 0.25) headHeight = hh
  })

  function onScroll() {
    if (!scroller) return
    const next = scroller.scrollTop
    keepFocus(next)
    scrollTop = next
  }

  /**
   * A focused row that is about to leave the rendered window hands the focus to the scroll area,
   * so the arrow keys keep working (an open editor commits first).
   */
  function keepFocus(nextTop: number) {
    const active = document.activeElement
    if (!(active instanceof HTMLElement) || !tbody?.contains(active)) return
    const tr = active.closest<HTMLElement>('tr.row')
    if (!tr) return
    const index = Number(tr.dataset.index)
    const first = Math.floor(nextTop / rowHeight) - OVERSCAN
    const last = Math.ceil((nextTop + viewportHeight - headHeight) / rowHeight) + OVERSCAN
    if (index >= first && index < last) return
    if (active instanceof HTMLInputElement) active.blur()
    scroller?.focus({ preventScroll: true })
  }

  /** Scrolls a row into view (below the sticky header), then focuses it. */
  async function focusRow(path: string) {
    focusPath = path
    const index = rows.findIndex((r) => r.path === path)
    if (index < 0 || !scroller) return
    const rowTop = index * rowHeight
    let next = scroller.scrollTop
    if (rowTop < next) next = rowTop
    else if (headHeight + rowTop + rowHeight > next + viewportHeight) next = headHeight + rowTop + rowHeight - viewportHeight
    if (Math.abs(next - scroller.scrollTop) > 0.5) {
      scroller.scrollTop = next
      scrollTop = scroller.scrollTop
    }
    await tick()
    tbody?.querySelector<HTMLTableRowElement>(`tr[data-path="${CSS.escape(path)}"]`)?.focus({ preventScroll: true })
  }

  // ------------------------------------------------------------ rows

  function rowTitle(row: Row): string {
    const text =
      row.status === 'duplicate'
        ? `Cópia idêntica de ${row.duplicateName}; fica fora dos totais.`
        : row.status === 'ok'
          ? STATUS_HINT.ok
          : (row.message ?? STATUS_HINT[row.status])
    const origin = originLabel(row.origin)
    return origin ? `${text} Origem: ${origin}.` : text
  }

  let previewTimer: ReturnType<typeof setTimeout> | undefined

  function onRowClick(event: MouseEvent, row: Row) {
    focusPath = row.path
    clearTimeout(previewTimer)
    if (event.shiftKey) {
      store.selectRange(row.path, true)
      return
    }
    if (event.ctrlKey || event.metaKey) {
      store.toggleSelected(row.path)
      return
    }
    store.setAnchor(row.path)
    // Opening the preview reflows (or covers) the table: on the editable cells, wait a moment so
    // that a double-click (edit) still lands on the same cell. Switching rows with the preview
    // already open changes nothing around the cell.
    const editable = (event.target as Element).closest('td.c-value, td.c-comp')
    if (editable && !store.activePath) {
      if (event.detail < 2) previewTimer = setTimeout(() => store.openPreview(row.path), 260)
    } else {
      store.openPreview(row.path)
    }
  }

  function onCheckboxClick(event: MouseEvent, row: Row) {
    event.stopPropagation()
    focusPath = row.path
    if (event.shiftKey) store.selectRange(row.path, !store.selected.has(row.path))
    else store.toggleSelected(row.path)
  }

  function onRowDblClick(event: MouseEvent, row: Row) {
    // Double-click on the value or competence cell opens its editor (and not the preview).
    clearTimeout(previewTimer)
    const cell = (event.target as Element).closest('td')
    if (cell?.classList.contains('c-value')) store.editingPath = row.path
    else if (cell?.classList.contains('c-comp')) store.editingCompetencePath = row.path
  }

  /** Keys on a row, or on the scroll area after the focused row scrolled away. */
  function onKeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement
    const onRow = target.matches('tr.row')
    if (!onRow && target !== scroller) return // inner buttons and editors handle their own keys
    const path = onRow ? (target.dataset.path ?? null) : focusPath
    const index = path ? rows.findIndex((r) => r.path === path) : -1
    const current = index >= 0 ? rows[index] : null
    const page = Math.max(1, Math.floor((viewportHeight - headHeight) / rowHeight) - 1)
    const go = (i: number, extend = false) => {
      event.preventDefault()
      const next = rows[Math.max(0, Math.min(rows.length - 1, i))]
      if (!next) return
      void focusRow(next.path)
      if (extend) store.selectRange(next.path, true)
      else if (store.activePath) store.openPreview(next.path)
    }
    const from = index >= 0 ? index : firstVisible - 1
    switch (event.key) {
      case 'ArrowDown':
        go(from + 1, event.shiftKey)
        break
      case 'ArrowUp':
        go(index >= 0 ? index - 1 : firstVisible, event.shiftKey)
        break
      case 'PageDown':
        go(from + page)
        break
      case 'PageUp':
        go(Math.max(0, from - page))
        break
      case 'Home':
        go(0)
        break
      case 'End':
        go(rows.length - 1)
        break
      case ' ':
        if (!current) return
        event.preventDefault()
        if (event.shiftKey) store.selectRange(current.path, true)
        else store.toggleSelected(current.path)
        break
      case 'Enter':
        if (!current) return
        event.preventDefault()
        store.editingPath = current.path
        break
      case 'Escape':
        if (store.activePath) {
          event.preventDefault()
          store.closePreview()
        } else if (store.selected.size) {
          event.preventDefault()
          store.clearSelection()
        }
        break
      case 'a':
      case 'A':
        if (event.ctrlKey || event.metaKey) {
          event.preventDefault()
          store.setVisibleSelected(true)
        }
        break
    }
  }

  /** Delete (or Backspace) anywhere in the table: removes the selected rows, else the focused one. */
  function onCardKeydown(event: KeyboardEvent) {
    if (event.key !== 'Delete' && event.key !== 'Backspace') return
    const target = event.target as HTMLElement
    if (target.closest('input:not([type="checkbox"]), textarea, select, [role="menu"], [role="listbox"]')) return
    const focused = focusPath ? rows.find((r) => r.path === focusPath) : undefined
    const list = store.selectedRows.length ? store.selectedRows : focused ? [focused] : []
    if (!list.length) return
    event.preventDefault()
    void removeRows(list)
  }

  /** Removes rows (one undo toast) and keeps the keyboard focus in the table. */
  async function removeRows(list: Row[]) {
    if (!list.length) return
    const indexes = list.map((r) => rows.indexOf(r)).filter((i) => i >= 0)
    const focusedIndex = focusPath ? rows.findIndex((r) => r.path === focusPath) : -1
    const at = indexes.length ? Math.min(...indexes) : Math.max(0, focusedIndex)
    const hadFocus = !!card?.contains(document.activeElement)
    store.removeRows(list)
    await tick()
    const next = rows[Math.min(at, rows.length - 1)]
    if (next && hadFocus) void focusRow(next.path)
  }

  // ------------------------------------------------------------ header

  function ariaSort(key: SortKey): 'ascending' | 'descending' | 'none' {
    if (store.sort?.key !== key) return 'none'
    return store.sort.dir === 'asc' ? 'ascending' : 'descending'
  }

  function sortTitle(key: SortKey, label: string): string {
    if (store.sort?.key !== key) return `Ordenar por ${label.toLowerCase()}`
    return store.sort.dir === 'asc'
      ? 'Ordem crescente. Clique para decrescente'
      : 'Ordem decrescente. Clique para voltar à ordem original'
  }

  function clearFilters() {
    store.setFilter('all')
    store.setKindFilter('all')
    store.search = ''
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="card" bind:this={card} onkeydown={onCardKeydown}>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="scroll"
    bind:this={scroller}
    bind:clientHeight={viewportHeight}
    tabindex="-1"
    onscroll={onScroll}
    onkeydown={onKeydown}
  >
    <table aria-rowcount={rows.length + 1}>
      <thead bind:this={thead}>
        <tr aria-rowindex={1}>
          <th class="c-check">
            <input
              type="checkbox"
              aria-label="Selecionar todas as notas visíveis"
              title="Selecionar todas as notas visíveis (Ctrl+A na tabela)"
              checked={allChecked}
              indeterminate={someChecked}
              disabled={!rows.length}
              onchange={(e) => store.setVisibleSelected(e.currentTarget.checked)}
            />
          </th>
          {#each COLUMNS as col (col.key)}
            {@const active = store.sort?.key === col.key}
            <th class={col.cls} aria-sort={ariaSort(col.key)}>
              <button
                type="button"
                class="sort"
                class:active
                title={sortTitle(col.key, col.label)}
                onclick={() => store.cycleSort(col.key)}
              >
                <span class="label">{col.label}</span>
                <span class="arrow" aria-hidden="true">
                  <Icon
                    name={active ? (store.sort?.dir === 'asc' ? 'arrowUp' : 'arrowDown') : 'arrowUpDown'}
                    size={13}
                    strokeWidth={2}
                  />
                </span>
              </button>
            </th>
          {/each}
          <th class="c-actions"><span class="sr-only">Ações</span></th>
        </tr>
      </thead>
      <tbody bind:this={tbody}>
        <!-- Spacers stand for the rows that are not rendered. One cell, no colspan: a colspan larger
             than the displayed columns (Origem hides on narrow tables) would add a phantom column. -->
        {#if start > 0}
          <tr class="spacer" aria-hidden="true"><td style="height: {start * rowHeight}px"></td></tr>
        {/if}
        {#each slice as row, i (row.path)}
          {@const tabbable = row.path === tabPath}
          {@const checked = store.selected.has(row.path)}
          <tr
            class="row"
            data-path={row.path}
            data-index={start + i}
            aria-rowindex={start + i + 2}
            tabindex={tabbable ? 0 : -1}
            class:active={row.path === store.activePath}
            class:checked
            class:dim={row.status === 'duplicate'}
            class:last={start + i === rows.length - 1}
            aria-current={row.path === store.activePath ? 'true' : undefined}
            onclick={(e) => onRowClick(e, row)}
            onmousedown={(e) => {
              // No text selection on Shift+click ranges.
              if (e.shiftKey) e.preventDefault()
            }}
            ondblclick={(e) => onRowDblClick(e, row)}
            onfocus={() => (focusPath = row.path)}
          >
            <td class="c-check">
              <input
                type="checkbox"
                tabindex="-1"
                aria-label="Selecionar {row.name}"
                title="Shift+clique seleciona um intervalo"
                {checked}
                onclick={(e) => onCheckboxClick(e, row)}
              />
            </td>
            <td class="c-file">
              <div class="name" title={row.path}>{row.name}</div>
              <div class="dir" title={row.dir}>{shortenPath(row.dir, 3)}</div>
            </td>
            <td class="c-type"><TypeCell {row} {tabbable} /></td>
            <td class="c-kind">
              <button
                type="button"
                class="kind"
                class:expense={row.kind === 'expense'}
                class:edited={row.kindEdited}
                tabindex={tabbable ? 0 : -1}
                title="{row.kindEdited ? 'Natureza escolhida por você. ' : ''}Clique para marcar como {row.kind === 'expense'
                  ? 'receita'
                  : 'despesa'}"
                aria-label="Natureza: {KIND_LABEL[row.kind]}. Marcar como {row.kind === 'expense' ? 'receita' : 'despesa'}"
                onclick={(e) => {
                  e.stopPropagation()
                  store.toggleKind(row)
                }}
              >
                {KIND_LABEL[row.kind]}
              </button>
            </td>
            <td class="c-comp"><CompetenceCell {row} {tabbable} /></td>
            <td class="c-value"><ValueCell {row} {tabbable} /></td>
            <td class="c-status"><StatusBadge status={row.status} title={rowTitle(row)} /></td>
            <td class="c-origin">
              <span class="origin" title={originLabel(row.origin)}>{originLabel(row.origin) || '—'}</span>
            </td>
            <td class="c-actions"><RowMenu {row} {tabbable} /></td>
          </tr>
        {/each}
        {#if end < rows.length}
          <tr class="spacer" aria-hidden="true"><td style="height: {(rows.length - end) * rowHeight}px"></td></tr>
        {/if}
      </tbody>
    </table>

    {#if !rows.length}
      <div class="empty">
        {#if store.rows.length}
          <Icon name="search" size={20} />
          <p>Nenhuma nota corresponde ao filtro.</p>
          <button type="button" class="link" onclick={clearFilters}>Limpar filtros</button>
        {:else if store.busy}
          <span class="spinner"></span>
          <p>Procurando PDFs…</p>
        {:else}
          <Icon name="fileText" size={20} />
          <p>Nenhum PDF encontrado nas fontes escolhidas.</p>
          <p class="muted">Confira os termos ignorados ou ative “Incluir subpastas”.</p>
        {/if}
      </div>
    {/if}
  </div>

  {#if selectedCount}
    <div class="foot bulk" role="region" aria-label="Ações para as notas selecionadas">
      <div class="bulk-info">
        <span class="count num">{plural(selectedCount, 'selecionada', 'selecionadas')}</span>
        {#if store.totals.selectedRevenue}
          <span class="sum num" title="Soma das receitas selecionadas que entram no total">{formatBRL(store.totals.selectedRevenue)}</span>
        {/if}
        {#if store.totals.selectedExpense}
          <span class="sum expense num" title="Soma das despesas selecionadas que entram no total">
            despesas {formatBRL(store.totals.selectedExpense)}
          </span>
        {/if}
        {#if hiddenSelected}
          <span class="hidden" title="Selecionadas que o filtro atual esconde; as ações valem para elas também">
            {formatInt(hiddenSelected)} fora do filtro
          </span>
        {/if}
      </div>
      <div class="bulk-actions">
        <BulkTypeMenu rows={store.selectedRows} />
        {#if selectedHasRevenue}
          <button
            type="button"
            class="btn btn-sm"
            title="As selecionadas passam a contar como despesas. Dá para desfazer."
            onclick={() => store.setKindMany(store.selectedRows, 'expense')}
          >
            <Icon name="receipt" size={14} />
            Marcar como despesa
          </button>
        {/if}
        {#if selectedHasExpense}
          <button
            type="button"
            class="btn btn-sm"
            title="As selecionadas passam a contar como receitas. Dá para desfazer."
            onclick={() => store.setKindMany(store.selectedRows, 'revenue')}
          >
            <Icon name="coins" size={14} />
            Marcar como receita
          </button>
        {/if}
        <button
          type="button"
          class="btn btn-sm remove"
          title="Tira as notas da lista (Delete). Dá para desfazer."
          onclick={() => removeRows(store.selectedRows)}
        >
          <Icon name="trash" size={14} />
          Remover selecionadas ({formatInt(selectedCount)})
        </button>
        <button type="button" class="btn btn-sm btn-ghost" onclick={() => store.clearSelection()}>Limpar seleção</button>
      </div>
    </div>
  {:else}
    <div class="foot">
      <TypeTotals />
      <div class="foot-right">
        {#if store.totals.duplicates}
          <label class="dup">
            <input
              type="checkbox"
              checked={store.includeDuplicates}
              onchange={(e) => store.setIncludeDuplicates(e.currentTarget.checked)}
            />
            Somar duplicadas
          </label>
        {/if}
        {#if store.removedCount}
          <span class="removed">
            {plural(store.removedCount, 'removida', 'removidas')}
            <button type="button" class="link" onclick={() => store.restoreRemoved()}>Restaurar</button>
          </span>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .card {
    --row-h: 48px;
    container: doctable / inline-size;
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
  }

  .scroll:focus {
    outline: none;
  }

  table {
    width: 100%;
    table-layout: fixed;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 13px;
  }

  th {
    position: sticky;
    top: 0;
    z-index: 2;
    height: 36px;
    padding: 0 var(--s-3);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    color: var(--text-3);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
  }

  .sort {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    height: 26px;
    margin-left: -6px;
    padding: 0 6px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: inherit;
    font: inherit;
    transition:
      background-color 0.12s var(--ease),
      color 0.12s var(--ease);
  }

  .sort .label {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sort:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .sort.active {
    color: var(--text);
  }

  .arrow {
    display: grid;
    place-items: center;
    flex: none;
    opacity: 0;
    transition: opacity 0.12s var(--ease);
  }

  .sort:hover .arrow,
  .sort:focus-visible .arrow {
    opacity: 0.55;
  }

  .sort.active .arrow {
    opacity: 1;
    color: var(--accent-text);
  }

  /* Numbers are right-aligned: the arrow goes before the label. */
  th.c-value .sort {
    flex-direction: row-reverse;
    margin-left: 0;
    margin-right: -6px;
  }

  td {
    height: var(--row-h);
    padding: 0 var(--s-3);
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }

  tbody tr.last td {
    border-bottom-color: transparent;
  }

  tr {
    outline: none;
  }

  tr.spacer td {
    padding: 0;
    border: 0;
  }

  tr.row {
    cursor: default;
    transition: background-color 0.1s var(--ease);
    /* Keep rows scrolled into view clear of the sticky header. */
    scroll-margin-top: 40px;
    scroll-margin-bottom: 4px;
  }

  tr.row:hover {
    background: var(--surface-2);
  }

  tr.row.checked {
    background: var(--accent-soft);
  }

  tr.row.active {
    background: var(--accent-soft);
  }

  tr.row.active td:first-child {
    box-shadow: inset 3px 0 0 var(--accent);
  }

  tr.row:focus-visible td {
    box-shadow:
      inset 0 1px 0 var(--focus-ring),
      inset 0 -1px 0 var(--focus-ring);
  }

  tr.row:focus-visible td:first-child {
    box-shadow:
      inset 2px 0 0 var(--focus-ring),
      inset 0 1px 0 var(--focus-ring),
      inset 0 -1px 0 var(--focus-ring);
  }

  tr.row:focus-visible td:last-child {
    box-shadow:
      inset -2px 0 0 var(--focus-ring),
      inset 0 1px 0 var(--focus-ring),
      inset 0 -1px 0 var(--focus-ring);
  }

  .c-check {
    width: 44px;
    padding-right: 0;
    text-align: center;
  }

  .c-type {
    width: 112px;
    padding-left: 6px;
  }

  th.c-type {
    padding-left: 14px;
  }

  .c-comp {
    width: 128px;
  }

  /* Natureza: its own column on wide tables; narrower ones mark expenses under the type. */
  .c-kind {
    display: none;
    width: 100px;
  }

  .kind {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 8px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: var(--muted-soft);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    transition:
      border-color 0.12s var(--ease),
      background-color 0.12s var(--ease);
  }

  .kind.expense {
    background: var(--expense-soft);
    color: var(--expense);
  }

  .kind:hover,
  .kind:focus-visible {
    border-color: var(--border-strong);
  }

  .kind.edited {
    font-weight: 600;
  }

  @container doctable (min-width: 1180px) {
    .c-kind {
      display: table-cell;
    }
  }

  .c-value {
    width: 128px;
    text-align: right;
  }

  .c-status {
    width: 138px;
  }

  .c-origin {
    width: 160px;
    color: var(--text-2);
  }

  .c-actions {
    width: 44px;
    padding: 0 8px 0 0;
    text-align: right;
  }

  .name {
    font-weight: 500;
    line-height: 18px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dir {
    margin-top: 1px;
    font-size: 12px;
    line-height: 16px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  tr.dim .name {
    color: var(--text-2);
  }

  .origin {
    display: block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--s-2);
    padding: var(--s-8) var(--s-4);
    color: var(--text-3);
    text-align: center;
    font-size: 13px;
  }

  .empty p:first-of-type {
    color: var(--text-2);
    font-weight: 500;
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
    min-height: 44px;
    padding: 8px var(--s-3) 8px var(--s-4);
    border-top: 1px solid var(--border);
    background: var(--surface-2);
    flex-wrap: wrap;
  }

  .foot:not(.bulk) {
    flex-wrap: nowrap;
  }

  .foot:not(.bulk) > :global(.totals) {
    flex: 1;
  }

  .foot-right {
    display: flex;
    align-items: center;
    flex: none;
    gap: var(--s-4);
    font-size: 12.5px;
    color: var(--text-2);
  }

  .dup {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    white-space: nowrap;
  }

  .removed {
    display: inline-flex;
    gap: 6px;
    white-space: nowrap;
    color: var(--text-3);
  }

  /* ------------------------------------------------------------ bulk actions */

  .foot.bulk {
    background: var(--accent-soft);
    border-top-color: var(--accent-soft-2);
    animation: rise 0.14s var(--ease);
  }

  .bulk-info {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
    font-size: 13px;
    white-space: nowrap;
  }

  .count {
    font-weight: 600;
    color: var(--accent-text);
  }

  .sum {
    font-weight: 500;
    color: var(--text);
  }

  .sum.expense {
    color: var(--expense);
  }

  .hidden {
    font-size: 12px;
    color: var(--text-3);
  }

  .bulk-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .remove {
    color: var(--danger);
  }

  .remove:hover:not(:disabled) {
    background: var(--danger-soft);
    border-color: var(--danger);
  }

  @keyframes rise {
    from {
      opacity: 0.4;
    }
  }

  @media (max-height: 700px) {
    .card {
      --row-h: 44px;
    }
  }

  @container doctable (max-width: 940px) {
    .c-origin {
      display: none;
    }
  }

  @container doctable (max-width: 760px) {
    .c-check {
      width: 38px;
    }

    .c-type {
      width: 100px;
    }

    .c-comp {
      width: 118px;
    }

    th.c-comp {
      padding-left: 8px;
      padding-right: 8px;
    }

    td.c-comp {
      padding-left: 10px;
    }

    .c-value {
      width: 120px;
    }

    .c-status {
      width: 104px;
      padding-right: 4px;
    }

    .c-status :global(.badge.has-short .full) {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
    }

    .c-status :global(.badge .short) {
      display: inline;
    }

    .c-actions {
      width: 40px;
      padding-right: 6px;
    }

    .sort .arrow {
      display: none;
    }

    .sort.active .arrow {
      display: grid;
    }
  }
</style>
