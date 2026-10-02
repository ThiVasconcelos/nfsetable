<script lang="ts">
  // "Fluxo mês a mês": each month considered in the current regime, from report.cashFlow: its
  // revenue (notes or projection), taxes, INSS + IRRF (not in the MEI: no pró-labore), fixed and
  // variable costs, reserve and what is left, plus the totals. In the Simples each month has its
  // own RBT12, Fator R and annex: a III / V tag per month.
  import { formatAmount, formatBRL, formatMonthLong, formatMonthShort, formatPercent, plural } from '../lib/format'
  import { tax } from '../lib/tax.svelte'
  import type { CashFlowMonth } from '../lib/types'
  import TaxCard from './TaxCard.svelte'

  interface Props {
    flow: CashFlowMonth[]
    regimeLabel: string
  }

  let { flow, regimeLabel }: Props = $props()

  type AmountKey = 'revenueCents' | 'companyTaxesCents' | 'ownerTaxesCents' | 'fixedCostsCents' | 'variableCostsCents' | 'reserveCents' | 'leftoverCents'

  const ALL_COLUMNS: { key: AmountKey; label: string; hint: string }[] = [
    { key: 'revenueCents', label: 'Receita', hint: 'Faturamento do mês: das notas ou o previsto' },
    { key: 'companyTaxesCents', label: 'Impostos', hint: 'Impostos da empresa sobre a receita do mês' },
    { key: 'ownerTaxesCents', label: 'INSS + IRRF', hint: 'INSS e IRRF do pró-labore' },
    { key: 'fixedCostsCents', label: 'Fixos', hint: 'Custos fixos (anuais divididos por 12)' },
    { key: 'variableCostsCents', label: 'Variáveis', hint: 'Gastos variáveis do mês' },
    { key: 'reserveCents', label: 'Reserva', hint: 'Reserva guardada sobre a receita do mês' },
    { key: 'leftoverCents', label: 'Sobra', hint: 'Receita − impostos − INSS e IRRF − custos − reserva' },
  ]

  const regime = $derived(tax.settings.regime)
  /** No pró-labore in the MEI: no INSS + IRRF column. */
  const COLUMNS = $derived(regime === 'mei' ? ALL_COLUMNS.filter((c) => c.key !== 'ownerTaxesCents') : ALL_COLUMNS)
  const sources = $derived(new Map(tax.planMonths.map((m) => [m.month, m.source])))
  const totals = $derived.by(() => {
    const t = { revenueCents: 0, companyTaxesCents: 0, ownerTaxesCents: 0, fixedCostsCents: 0, variableCostsCents: 0, reserveCents: 0, leftoverCents: 0 }
    for (const m of flow) for (const c of ALL_COLUMNS) t[c.key] += m[c.key]
    return t
  })
  const negatives = $derived(flow.filter((m) => m.leftoverCents < 0).length)
  /** Simples: each month has its annex. */
  const annexes = $derived(flow.some((m) => m.annex))

  /** Tooltip of a month's annex tag. */
  function annexTitle(m: CashFlowMonth): string {
    if (m.annex === 'V') return 'Fator R abaixo de 28% neste mês: o DAS vai pelo Anexo V'
    if (tax.activity === 'annexIii') return 'Anexo III: a atividade é sempre do Anexo III'
    return `Anexo III: Fator R de ${formatPercent(m.fatorR ?? 0)} neste mês`
  }

</script>

<TaxCard
  title="Fluxo mês a mês"
  subtitle="{annexes ? 'Simples Nacional, cada mês com o próprio RBT12, Fator R e anexo' : regimeLabel} · a receita e os gastos de cada mês"
  icon="calendar"
  wide
>
  <div class="wrap">
    <table aria-describedby="cashflow-note">
      <caption class="sr-only">Fluxo mês a mês, valores em reais</caption>
      <thead>
        <tr>
          <th class="c-month" scope="col">Mês</th>
          {#each COLUMNS as c (c.key)}
            <th scope="col" class="c-num" class:c-revenue={c.key === 'revenueCents'} title={c.hint}>{c.label}</th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each flow as m (m.month)}
          {@const source = sources.get(m.month)}
          <tr data-month={m.month}>
            <th class="c-month" scope="row" title={formatMonthLong(m.month)}>
              {formatMonthShort(m.month, 'short')}
              {#if m.annex}
                <span class="annex" data-annex={m.annex} title={annexTitle(m)}>
                  {m.annex}<span class="sr-only">: {annexTitle(m)}</span>
                </span>
              {/if}
            </th>
            <td class="c-num c-revenue num">
              {#if source}
                <span class="badge" class:forecast={source === 'forecast'}>{source === 'forecast' ? 'previsto' : 'notas'}</span>
              {/if}
              {formatAmount(m.revenueCents)}
            </td>
            <td class="c-num num">{formatAmount(m.companyTaxesCents)}</td>
            {#if regime !== 'mei'}<td class="c-num num">{formatAmount(m.ownerTaxesCents)}</td>{/if}
            <td class="c-num num">{formatAmount(m.fixedCostsCents)}</td>
            <td class="c-num num">{formatAmount(m.variableCostsCents)}</td>
            <td class="c-num num">{formatAmount(m.reserveCents)}</td>
            <td class="c-num num leftover" class:negative={m.leftoverCents < 0}>{formatAmount(m.leftoverCents)}</td>
          </tr>
        {/each}
      </tbody>
      <tfoot>
        <tr>
          <th class="c-month" scope="row">Total</th>
          {#each COLUMNS as c (c.key)}
            <td
              class="c-num num"
              class:c-revenue={c.key === 'revenueCents'}
              class:leftover={c.key === 'leftoverCents'}
              class:negative={c.key === 'leftoverCents' && totals.leftoverCents < 0}
            >
              {formatAmount(totals[c.key])}
            </td>
          {/each}
        </tr>
      </tfoot>
    </table>
  </div>
  <p class="note" id="cashflow-note">
    {#if annexes}
      Valores em R$. Impostos de cada mês pelo RBT12 e pelo Fator R dele; o pró-labore é o do resumo.
    {:else if regime === 'mei'}
      Valores em R$. No MEI o imposto é o DAS fixo do mês.
    {:else}
      Valores em R$. Impostos sobre a receita de cada mês; o pró-labore é o do resumo.
    {/if}
    {#if negatives}
      <strong class="neg">{plural(negatives, 'mês ficou', 'meses ficaram')} no vermelho</strong> (sobra negativa).
    {:else}
      Sobra total de <strong class="num">{formatBRL(totals.leftoverCents)}</strong> em {plural(flow.length, 'mês', 'meses')}.
    {/if}
  </p>
</TaxCard>

<style>
  .wrap {
    margin: 0 calc(-1 * var(--s-4));
    overflow-x: auto;
  }

  table {
    width: 100%;
    min-width: 600px;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 12.5px;
  }

  th,
  td {
    padding: 8px 8px;
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
  }

  thead th {
    height: 32px;
    padding-top: 0;
    padding-bottom: 0;
    color: var(--text-3);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
  }

  th:first-child,
  td:first-child {
    padding-left: var(--s-4);
  }

  th:last-child,
  td:last-child {
    padding-right: var(--s-4);
  }

  .c-month {
    width: 72px;
    font-weight: 500;
    text-align: left;
  }

  .annex {
    display: inline-block;
    min-width: 24px;
    margin-left: 4px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--accent-soft-2);
    color: var(--accent-text);
    font-size: 10.5px;
    font-weight: 650;
    line-height: 16px;
    text-align: center;
    vertical-align: 1px;
  }

  .annex[data-annex='V'] {
    background: var(--warn-soft);
    color: var(--warn);
  }

  tbody .c-month {
    color: var(--text);
  }

  .c-num {
    text-align: right;
  }

  thead .c-num {
    text-align: right;
  }

  .c-revenue {
    min-width: 128px;
  }

  td.c-num {
    color: var(--text-2);
  }

  td.c-revenue,
  td.leftover {
    color: var(--text);
    font-weight: 600;
  }

  td.leftover.negative {
    color: var(--danger);
  }

  .badge {
    display: inline-block;
    margin-right: 6px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--surface-3);
    color: var(--text-3);
    font-size: 10.5px;
    font-weight: 500;
    line-height: 16px;
    vertical-align: 1px;
  }

  .badge.forecast {
    background: repeating-linear-gradient(45deg, var(--viz-1-soft) 0 3px, transparent 3px 6px);
    color: var(--accent-text);
  }

  tfoot th,
  tfoot td {
    border-bottom: 0;
    padding-top: 10px;
    font-weight: 600;
    color: var(--text);
  }

  tfoot td.c-num {
    color: var(--text);
  }

  .note {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }

  .note strong {
    font-weight: 600;
    color: var(--text-2);
  }

  .note .neg {
    color: var(--danger);
  }
</style>
