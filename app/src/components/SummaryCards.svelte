<script lang="ts">
  import { formatBRL, formatInt, plural } from '../lib/format'
  import { store, type KindFilter } from '../lib/store.svelte'
  import Icon from './Icon.svelte'

  const t = $derived(store.totals)
  const attention = $derived(t.attention)
  const pending = $derived(store.rows.filter((r) => r.status === 'pending').length)
  /** What is listed but left out of the totals. */
  const outside = $derived(
    [
      t.duplicates && !store.includeDuplicates ? plural(t.duplicates, 'duplicada', 'duplicadas') : '',
      t.cancelled ? plural(t.cancelled, 'cancelada', 'canceladas') : '',
    ].filter(Boolean),
  )
  const outsideCount = $derived(t.cancelled + (store.includeDuplicates ? 0 : t.duplicates))
  /** Selected rows of both kinds: the card shows the receitas, the despesas go below. */
  const mixed = $derived(t.selectedRevenue !== 0 && t.selectedExpense !== 0)
  /** Rows the period leaves out (outside its months, or without a competence month). */
  const period = $derived(store.periodInfo)
  const periodTitle = $derived(
    [
      period.outside ? `${plural(period.outside, 'nota fora', 'notas fora')} dos meses do período` : '',
      period.undated ? plural(period.undated, 'nota sem competência', 'notas sem competência') : '',
    ]
      .filter(Boolean)
      .join(' e '),
  )

  function toggleKind(kind: Exclude<KindFilter, 'all'>) {
    store.setKindFilter(store.kindFilter === kind ? 'all' : kind)
  }

  function showAttention() {
    store.setFilter(store.filter === 'attention' ? 'all' : 'attention')
  }
</script>

<section class="cards" aria-label="Resumo">
  <button
    type="button"
    class="card total"
    class:pressed={store.kindFilter === 'revenue'}
    aria-pressed={store.kindFilter === 'revenue'}
    title={store.kindFilter === 'revenue' ? 'Mostrando só as receitas. Clique para ver todas' : 'Mostrar só as receitas'}
    onclick={() => toggleKind('revenue')}
  >
    <span class="label">Receitas</span>
    <span class="value num">{formatBRL(t.revenue)}</span>
    <span
      class="sub"
      title={period.left ? `${periodTitle} (período)` : outside.length ? `${outside.join(' e ')} ficam fora dos totais` : undefined}
    >
      {#if store.phase === 'extracting'}
        Lendo {formatInt(store.progress.done)} de {formatInt(store.progress.total)}…
      {:else if pending}
        Somando…
      {:else if period.left}
        {plural(t.revenueCount, 'nota', 'notas')}<span class="outside period-left"
          >{`· ${formatInt(period.left)} fora`}<span class="long">{' do período'}</span></span
        >
      {:else}
        {plural(t.revenueCount, 'nota somada', 'notas somadas')}{#if outsideCount}<span class="outside"
            >{`· ${formatInt(outsideCount)} fora`}</span
          >{/if}
      {/if}
    </span>
  </button>

  <button
    type="button"
    class="card expense"
    class:has={t.expenseRows > 0}
    class:pressed={store.kindFilter === 'expense'}
    aria-pressed={store.kindFilter === 'expense'}
    disabled={!t.expenseRows && store.kindFilter !== 'expense'}
    title={t.expenseRows
      ? store.kindFilter === 'expense'
        ? 'Mostrando só as despesas. Clique para ver todas'
        : 'Mostrar só as despesas'
      : 'Nenhuma nota marcada como despesa'}
    onclick={() => toggleKind('expense')}
  >
    <span class="label">Despesas</span>
    <span class="value num" class:empty={!t.expenseCount}>{t.expenseCount ? formatBRL(t.expense) : '—'}</span>
    <span class="sub">
      {#if t.expenseRows}
        {plural(t.expenseCount, 'despesa somada', 'despesas somadas')}
      {:else}
        Nenhuma despesa ainda
      {/if}
    </span>
  </button>

  <div class="card">
    <span class="label">Selecionadas</span>
    <span class="value num" class:empty={!t.selectedCount}>
      {#if !t.selectedCount}
        —
      {:else if t.selectedRevenue === 0 && t.selectedExpense !== 0}
        {formatBRL(t.selectedExpense)}
      {:else}
        {formatBRL(t.selectedRevenue)}
      {/if}
    </span>
    <span class="sub">
      {#if t.selectedCount}
        {#if mixed}
          <span class="minus num" title="Despesas selecionadas">despesas {formatBRL(t.selectedExpense)}</span> ·
        {:else}
          {plural(t.selectedCount, 'selecionada', 'selecionadas')} ·
        {/if}
        <button type="button" class="link clear" onclick={() => store.clearSelection()}>Limpar</button>
      {:else}
        Marque linhas para somar
      {/if}
    </span>
  </div>

  <button
    type="button"
    class="card attention"
    class:has={attention > 0}
    class:pressed={store.filter === 'attention'}
    aria-pressed={store.filter === 'attention'}
    disabled={attention === 0 && store.filter !== 'attention'}
    onclick={showAttention}
    title={attention ? 'Mostrar só as notas que precisam de atenção' : undefined}
  >
    <span class="label">Precisam de atenção</span>
    <span class="value num">{formatInt(attention)}</span>
    <span class="sub">
      {#if attention}
        <Icon name="alert" size={12} />
        {store.filter === 'attention' ? 'Mostrando só estas' : 'Clique para ver'}
      {:else if pending}
        Aguardando a leitura
      {:else if store.rows.length}
        <Icon name="check" size={12} /> Tudo certo
      {:else}
        Sem pendências
      {/if}
    </span>
  </button>
</section>

<style>
  .cards {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: var(--s-3);
  }

  .card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    min-width: 0;
    padding: 14px var(--s-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    text-align: left;
    font: inherit;
    color: inherit;
  }

  button.card {
    cursor: pointer;
    transition:
      border-color 0.15s var(--ease),
      background-color 0.15s var(--ease);
  }

  button.card:disabled {
    cursor: default;
  }

  .label {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-3);
  }

  .value {
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.015em;
    line-height: 1.3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .value.empty {
    color: var(--text-3);
  }

  .sub {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .outside {
    white-space: pre;
  }

  .period-left {
    color: var(--accent-text);
  }

  .total .value {
    color: var(--text);
  }

  .total:hover,
  .expense.has:hover {
    border-color: var(--border-strong);
  }

  .total.pressed,
  .expense.pressed {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .expense.has .value:not(.empty) {
    color: var(--expense);
  }

  .minus {
    color: var(--expense);
  }

  .clear {
    font-size: 12px;
  }

  .attention.has .value,
  .attention.has .sub {
    color: var(--warn);
  }

  .attention:not(.has) {
    cursor: default;
  }

  .attention.has:hover {
    border-color: var(--warn);
  }

  .attention.pressed {
    border-color: var(--warn);
    background: var(--warn-soft);
  }

  .attention:not(.has) .sub :global(svg) {
    color: var(--ok);
  }

  @container main (max-width: 760px) {
    .cards {
      gap: var(--s-2);
    }

    .card {
      padding: 10px var(--s-3);
      border-radius: var(--radius);
    }

    .value {
      font-size: 18px;
    }

    /* "· 9 fora" (the title keeps the whole explanation). */
    .long {
      display: none;
    }
  }

  @media (max-height: 700px) {
    .card {
      padding-top: 10px;
      padding-bottom: 10px;
    }

    .value {
      font-size: 19px;
      line-height: 1.25;
    }
  }

  @container main (max-width: 520px) {
    .cards {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
