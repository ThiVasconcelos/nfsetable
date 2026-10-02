<script lang="ts">
  import { formatBRL, formatInt, plural } from '../lib/format'
  import { store, type TypeTotal } from '../lib/store.svelte'

  const revenue = $derived(store.typeTotals.filter((t) => t.kind === 'revenue'))
  const expense = $derived(store.typeTotals.filter((t) => t.kind === 'expense'))
  /** Group labels only when both kinds are listed. */
  const both = $derived(revenue.length > 0 && expense.length > 0)
</script>

{#snippet chips(list: TypeTotal[], label: string)}
  <ul aria-label={label} class:expense={list[0]?.kind === 'expense'}>
    {#each list as t (t.type)}
      <li title="{plural(t.count, 'nota', 'notas')} do tipo {t.type} ({label.toLowerCase()})">
        <span class="type">{t.type}</span>
        <span class="count num">{formatInt(t.count)}</span>
        <span class="sum num">{formatBRL(t.cents)}</span>
      </li>
    {/each}
  </ul>
{/snippet}

<div class="totals" role="group" aria-labelledby="type-totals-title">
  <span class="title" id="type-totals-title">Totais por tipo</span>
  {#if store.typeTotals.length}
    {#if revenue.length}
      {#if both}<span class="group">Receitas</span>{/if}
      {@render chips(revenue, 'Receitas')}
    {/if}
    {#if expense.length}
      <span class="group expense">Despesas</span>
      {@render chips(expense, 'Despesas')}
    {/if}
  {:else}
    <span class="none">—</span>
  {/if}
</div>

<style>
  /* One line: when the chips do not fit, the line scrolls sideways (the table keeps its height). */
  .totals {
    display: flex;
    align-items: center;
    gap: 6px var(--s-3);
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: thin;
    padding: 2px 28px 2px 0;
    /* The right edge fades when chips are hidden there (the padding keeps the last one clear). */
    mask-image: linear-gradient(to right, #000 calc(100% - 28px), transparent);
  }

  .totals > * {
    flex: none;
  }

  .title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-3);
    white-space: nowrap;
  }

  .group {
    margin-left: 2px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-3);
    white-space: nowrap;
  }

  .group.expense {
    color: var(--expense);
  }

  ul {
    display: flex;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    font-size: 12.5px;
    white-space: nowrap;
  }

  .type {
    font-weight: 500;
  }

  .count {
    min-width: 16px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--surface-3);
    font-size: 11px;
    line-height: 16px;
    text-align: center;
    color: var(--text-3);
  }

  .sum {
    color: var(--text-2);
  }

  ul.expense .sum {
    color: var(--expense);
  }

  .none {
    color: var(--text-3);
    font-size: 12.5px;
  }
</style>
